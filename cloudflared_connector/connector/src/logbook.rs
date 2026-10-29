//! What cloudflared prints: the last lines for the panel, and the state of each edge connection
//! as its log events describe it. The connection table is kept apart from the ring so a
//! connection registered weeks ago stays known after its line has scrolled out.

use std::collections::{BTreeMap, VecDeque};

use serde::Serialize;

use crate::clock;

pub const CAPACITY: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    /// A line without cloudflared's level column, such as Go's own `log` output.
    Other,
}

#[derive(Debug, Clone, Serialize)]
pub struct Line {
    pub seq: u64,
    pub at: Option<u64>,
    pub level: Level,
    /// The line as cloudflared printed it.
    pub text: String,
    /// The line without its time and level columns.
    pub message: String,
}

/// One edge connection as the log describes it, keyed by cloudflared's `connIndex`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Link {
    pub up: bool,
    pub location: Option<String>,
    pub protocol: Option<String>,
    pub since: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Default)]
pub struct LogBook {
    lines: VecDeque<Line>,
    next_seq: u64,
    links: BTreeMap<u32, Link>,
}

/// Messages after which a connection is down.
const DOWN: [&str; 3] = [
    "Connection terminated",
    "Unregistered tunnel connection",
    "Serve tunnel error",
];

impl LogBook {
    pub fn push(&mut self, text: &str) {
        let (at, level, message) = split(text);
        self.track(at, message);
        if self.lines.len() == CAPACITY {
            self.lines.pop_front();
        }
        self.lines.push_back(Line {
            seq: self.next_seq,
            at,
            level,
            text: text.to_string(),
            message: message.to_string(),
        });
        self.next_seq += 1;
    }

    /// cloudflared started again: its connections are new, its earlier lines stay.
    pub fn new_run(&mut self) {
        self.links.clear();
    }

    pub fn lines(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter()
    }

    pub fn links(&self) -> &BTreeMap<u32, Link> {
        &self.links
    }

    fn track(&mut self, at: Option<u64>, message: &str) {
        let Some(index) = field(message, "connIndex").and_then(|v| v.parse::<u32>().ok()) else {
            return;
        };
        if message.starts_with("Registered tunnel connection") {
            self.links.insert(
                index,
                Link {
                    up: true,
                    location: field(message, "location"),
                    protocol: field(message, "protocol"),
                    since: at,
                    error: None,
                },
            );
        } else if DOWN.iter().any(|m| message.starts_with(m)) {
            let link = self.links.entry(index).or_default();
            link.up = false;
            link.since = at;
            link.error = field(message, "error");
        }
    }
}

/// Splits `2026-10-29T09:53:19Z INF message` into its time, level and message.
fn split(text: &str) -> (Option<u64>, Level, &str) {
    let mut parts = text.splitn(3, ' ');
    let (Some(stamp), Some(tag)) = (parts.next(), parts.next()) else {
        return (None, Level::Other, text);
    };
    let Some(at) = clock::parse_utc(stamp) else {
        return (None, Level::Other, text);
    };
    let level = match tag {
        "DBG" => Level::Debug,
        "INF" => Level::Info,
        "WRN" => Level::Warn,
        "ERR" => Level::Error,
        "FTL" => Level::Fatal,
        _ => return (Some(at), Level::Other, text),
    };
    (Some(at), level, parts.next().unwrap_or(""))
}

/// The value of `key=value` or `key="quoted value"` in a cloudflared message.
pub fn field(message: &str, key: &str) -> Option<String> {
    let needle = format!(" {key}=");
    let start = message.find(&needle)? + needle.len();
    let rest = &message[start..];
    if let Some(quoted) = rest.strip_prefix('"') {
        let mut value = String::new();
        let mut chars = quoted.chars();
        while let Some(c) = chars.next() {
            match c {
                '\\' => value.push(chars.next()?),
                '"' => return Some(value),
                c => value.push(c),
            }
        }
        None
    } else {
        Some(rest.split(' ').next().unwrap_or("").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{CAPACITY, Level, LogBook, field};

    #[test]
    fn lines_keep_their_time_level_and_text() {
        let mut b = LogBook::default();
        b.push("2026-10-29T09:53:19Z WRN Serve tunnel error error=\"x\" connIndex=1");
        b.push("2026/10/29 21:42:30 failed to sufficiently increase receive buffer size");
        b.push("[cloudflared-connector] starting cloudflared");
        let lines: Vec<_> = b.lines().collect();
        assert_eq!(lines[0].level, Level::Warn);
        assert_eq!(lines[0].at, Some(1_793_267_599));
        assert_eq!(
            lines[0].message,
            "Serve tunnel error error=\"x\" connIndex=1"
        );
        assert_eq!(lines[1].level, Level::Other);
        assert_eq!(lines[1].at, None);
        assert_eq!(lines[2].level, Level::Other);
        assert_eq!(
            lines[2].text,
            "[cloudflared-connector] starting cloudflared"
        );
        assert_eq!(
            lines.iter().map(|l| l.seq).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn the_ring_keeps_the_last_lines_and_the_connection_table_outlives_them() {
        let mut b = LogBook::default();
        b.push("2026-10-29T09:53:19Z INF Registered tunnel connection connIndex=2 connection=c0ffee event=0 ip=198.51.100.7 location=ord05 protocol=quic");
        for i in 0..CAPACITY + 5 {
            b.push(&format!("2026-10-29T10:00:00Z DBG line {i}"));
        }
        assert_eq!(b.lines().count(), CAPACITY);
        assert_eq!(b.lines().next().unwrap().seq, 6);
        let link = &b.links()[&2];
        assert!(link.up);
        assert_eq!(link.location.as_deref(), Some("ord05"));
        assert_eq!(link.protocol.as_deref(), Some("quic"));
        assert_eq!(link.since, Some(1_793_267_599));
    }

    #[test]
    fn a_terminated_connection_is_down_with_its_error_until_it_registers_again() {
        let mut b = LogBook::default();
        b.push("2026-10-29T09:53:19Z INF Registered tunnel connection connIndex=1 connection=c0ffee event=0 ip=198.51.100.7 location=ord05 protocol=quic");
        b.push("2026-10-29T10:43:59Z WRN Connection terminated error=\"datagram manager error: timeout: no recent network activity\" connIndex=1");
        let link = &b.links()[&1];
        assert!(!link.up);
        assert_eq!(
            link.error.as_deref(),
            Some("datagram manager error: timeout: no recent network activity")
        );
        assert_eq!(link.location.as_deref(), Some("ord05"));
        b.push("2026-10-29T10:44:05Z INF Registered tunnel connection connIndex=1 connection=beef event=0 ip=198.51.100.9 location=iad08 protocol=http2");
        let link = &b.links()[&1];
        assert!(link.up);
        assert_eq!(link.location.as_deref(), Some("iad08"));
        assert_eq!(link.protocol.as_deref(), Some("http2"));
        assert_eq!(link.error, None);
        b.new_run();
        assert!(b.links().is_empty());
        assert_eq!(b.lines().count(), 3);
    }

    #[test]
    fn retries_and_dial_failures_do_not_change_a_connection() {
        let mut b = LogBook::default();
        b.push("2026-10-29T21:42:30Z ERR Register tunnel error from server side error=\"Failed to get tunnel\" connIndex=0 event=0 ip=198.51.100.7");
        b.push("2026-10-29T21:42:30Z INF Retrying connection in up to 2s connIndex=0 event=0 ip=198.51.100.7");
        assert!(b.links().is_empty());
    }

    #[test]
    fn fields_read_plain_and_quoted_values() {
        let m = "Connection terminated error=\"a \\\"b\\\" c\" connIndex=3 location=ord02";
        assert_eq!(field(m, "error").as_deref(), Some("a \"b\" c"));
        assert_eq!(field(m, "connIndex").as_deref(), Some("3"));
        assert_eq!(field(m, "location").as_deref(), Some("ord02"));
        assert_eq!(field(m, "protocol"), None);
        assert_eq!(field("error=\"unterminated", "error"), None);
    }
}
