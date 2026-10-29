//! Unix seconds, and the one timestamp form cloudflared prints (`2026-10-29T09:53:19Z`).

use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Parses `YYYY-MM-DDTHH:MM:SSZ`, with optional fractional seconds, into Unix seconds.
pub fn parse_utc(s: &str) -> Option<u64> {
    let s = s.strip_suffix('Z')?;
    let (date, time) = s.split_once('T')?;
    let mut d = date.splitn(3, '-').map(str::parse::<i64>);
    let (y, m, day) = (d.next()?.ok()?, d.next()?.ok()?, d.next()?.ok()?);
    let time = time.split_once('.').map_or(time, |(whole, _)| whole);
    let mut t = time.splitn(3, ':').map(str::parse::<i64>);
    let (hh, mm, ss) = (t.next()?.ok()?, t.next()?.ok()?, t.next()?.ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&day) || hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    let secs = days_from_civil(y, m, day) * 86_400 + hh * 3600 + mm * 60 + ss;
    u64::try_from(secs).ok()
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::parse_utc;

    #[test]
    fn cloudflared_timestamps_parse_to_unix_seconds() {
        assert_eq!(parse_utc("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_utc("2026-10-29T09:53:19Z"), Some(1_793_267_599));
        assert_eq!(parse_utc("2024-02-29T12:00:00.250Z"), Some(1_709_208_000));
    }

    #[test]
    fn other_forms_are_refused() {
        for s in [
            "2026/10/29 21:42:30",
            "2026-10-29T09:53:19",
            "2026-13-01T00:00:00Z",
            "INF",
            "",
        ] {
            assert_eq!(parse_utc(s), None, "{s}");
        }
    }
}
