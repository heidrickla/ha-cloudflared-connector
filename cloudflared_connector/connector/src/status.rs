//! The one status model the panel renders. It is built from plain inputs (the supervision record,
//! the log book and cloudflared's endpoint readings), so a test can feed it recorded fixtures.

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::Value;

use crate::cloudflared::Readings;
use crate::logbook::{Line, LogBook};
use crate::metrics::Metrics;

/// cloudflared's default `--ha-connections`.
pub const EXPECTED_CONNECTIONS: u32 = 4;
/// Time cloudflared gets to open its metrics listener before a failed reading is reported.
pub const SETTLE_S: u64 = 15;
/// What the panel and the add-on log say while the token option is empty.
pub const NO_TOKEN: &str = "No tunnel token. In Cloudflare Zero Trust, Networks > Tunnels, copy the \
    tunnel's token, or the whole install command it shows, into tunnel_token on the add-on's \
    Configuration tab, then restart the add-on.";

/// What the supervisor knows about cloudflared, independent of what cloudflared reports.
#[derive(Debug, Clone, Default)]
pub struct Supervision {
    pub phase: Phase,
    pub connector_started: u64,
    pub restarts: u32,
    pub last_exit: Option<Exit>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum Phase {
    /// No tunnel token, so cloudflared is never started.
    #[default]
    Unconfigured,
    Running {
        since: u64,
    },
    /// cloudflared exited and starts again at `until`.
    Waiting {
        until: u64,
    },
    Stopping,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Exit {
    pub at: u64,
    pub code: Option<i32>,
    pub signal: Option<i32>,
    /// Ended by a restart from the panel, not by cloudflared itself.
    pub requested: bool,
    /// Set when cloudflared could not be started at all.
    pub error: Option<String>,
}

pub struct Inputs<'a> {
    pub now: u64,
    pub loglevel: &'a str,
    pub supervision: &'a Supervision,
    pub log: &'a LogBook,
    /// None while cloudflared is not running.
    pub readings: Option<&'a Readings>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Unconfigured,
    Starting,
    Connected,
    Degraded,
    Disconnected,
    Restarting,
    Stopping,
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub addon_version: &'static str,
    pub generated_at: u64,
    pub state: State,
    pub headline: String,
    pub cloudflared: Cloudflared,
    pub process: Process,
    pub connections: Connections,
    pub traffic: Traffic,
    pub routes: Routes,
    pub log: Vec<Line>,
    /// Inputs the status lacked, each in a sentence.
    pub notices: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct Cloudflared {
    pub version: Option<String>,
    pub go_version: Option<String>,
    pub connector_id: Option<String>,
    pub config_version: Option<i64>,
    pub loglevel: String,
}

#[derive(Debug, Serialize)]
pub struct Process {
    pub running: bool,
    pub uptime_s: Option<u64>,
    pub connector_uptime_s: u64,
    pub restarts: u32,
    pub last_exit: Option<Exit>,
    pub last_exit_text: Option<String>,
    pub retry_in_s: Option<u64>,
    pub can_restart: bool,
}

#[derive(Debug, Serialize)]
pub struct Connections {
    pub ready: u32,
    pub expected: u32,
    pub links: Vec<Connection>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Connection {
    pub index: u32,
    /// None when the log level hides cloudflared's connection events.
    pub up: Option<bool>,
    pub location: Option<String>,
    pub protocol: Option<String>,
    pub since: Option<u64>,
    pub for_s: Option<u64>,
    pub rtt_ms: Option<f64>,
    pub sent_bytes: Option<u64>,
    pub received_bytes: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct Traffic {
    pub requests: Option<u64>,
    pub errors: Option<u64>,
    pub concurrent: Option<u64>,
    pub responses: Vec<ResponseCount>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ResponseCount {
    pub code: String,
    pub count: u64,
}

#[derive(Debug, Default, Serialize)]
pub struct Routes {
    /// False until cloudflared has received its configuration from the dashboard.
    pub received: bool,
    pub hostnames: Vec<Route>,
    /// The service of the final catch-all rule.
    pub fallback: Option<String>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Route {
    pub hostname: String,
    pub path: Option<String>,
    pub service: String,
}

pub fn build(i: &Inputs<'_>) -> Status {
    let sup = i.supervision;
    let mut notices = Vec::new();
    let uptime = match sup.phase {
        Phase::Running { since } => Some(i.now.saturating_sub(since)),
        _ => None,
    };
    let readings = i.readings.filter(|_| uptime.is_some());
    if let (Some(r), Some(up)) = (readings, uptime)
        && up >= SETTLE_S
    {
        for (path, error) in r.failures() {
            notices.push(format!("cloudflared's {path} did not answer: {error}"));
        }
    }

    let metrics = readings
        .and_then(|r| r.metrics.as_ref().ok())
        .map_or_else(Metrics::default, |t| Metrics::parse(t));
    let ready = readings
        .and_then(|r| r.ready.as_ref().ok())
        .and_then(|t| serde_json::from_str::<Value>(t).ok());
    let config = readings
        .and_then(|r| r.config.as_ref().ok())
        .and_then(|t| serde_json::from_str::<Value>(t).ok());

    let links = if uptime.is_some() {
        connections(i, &metrics)
    } else {
        Vec::new()
    };
    let ready_count = ready
        .as_ref()
        .and_then(|v| v["readyConnections"].as_u64())
        .map_or_else(
            || links.iter().filter(|l| l.up == Some(true)).count() as u32,
            |n| n as u32,
        );

    let state = match sup.phase {
        Phase::Unconfigured => State::Unconfigured,
        Phase::Waiting { .. } => State::Restarting,
        Phase::Stopping => State::Stopping,
        Phase::Running { .. } if ready_count >= EXPECTED_CONNECTIONS => State::Connected,
        Phase::Running { .. } if ready_count > 0 => State::Degraded,
        Phase::Running { since } if i.now.saturating_sub(since) < SETTLE_S * 2 => State::Starting,
        Phase::Running { .. } => State::Disconnected,
    };
    let retry_in_s = match sup.phase {
        Phase::Waiting { until } => Some(until.saturating_sub(i.now)),
        _ => None,
    };
    let build_info = metrics.series("build_info").next();

    Status {
        addon_version: env!("CARGO_PKG_VERSION"),
        generated_at: i.now,
        state,
        headline: headline(state, ready_count, retry_in_s, sup.last_exit.as_ref()),
        cloudflared: Cloudflared {
            version: build_info
                .and_then(|b| b.label("version"))
                .map(String::from),
            go_version: build_info
                .and_then(|b| b.label("goversion"))
                .map(String::from),
            connector_id: ready
                .as_ref()
                .and_then(|v| v["connectorId"].as_str())
                .map(String::from),
            config_version: config
                .as_ref()
                .and_then(|v| v["version"].as_i64())
                .filter(|v| *v >= 0),
            loglevel: i.loglevel.to_string(),
        },
        process: Process {
            running: uptime.is_some(),
            uptime_s: uptime,
            connector_uptime_s: i.now.saturating_sub(sup.connector_started),
            restarts: sup.restarts,
            last_exit: sup.last_exit.clone(),
            last_exit_text: sup.last_exit.as_ref().map(describe_exit),
            retry_in_s,
            can_restart: matches!(sup.phase, Phase::Running { .. } | Phase::Waiting { .. }),
        },
        connections: Connections {
            ready: ready_count,
            expected: EXPECTED_CONNECTIONS,
            links,
        },
        traffic: traffic(&metrics),
        routes: config.as_ref().map(routes).unwrap_or_default(),
        log: i.log.lines().cloned().collect(),
        notices,
    }
}

fn headline(state: State, ready: u32, retry_in_s: Option<u64>, exit: Option<&Exit>) -> String {
    let of = format!("{ready} of {EXPECTED_CONNECTIONS} edge connections registered");
    match state {
        State::Unconfigured => NO_TOKEN.into(),
        State::Connected | State::Degraded => of,
        State::Starting => "cloudflared is starting.".into(),
        State::Disconnected => format!("{of}; cloudflared keeps retrying."),
        State::Stopping => "Stopping.".into(),
        State::Restarting => {
            let how = exit.map_or_else(|| "cloudflared exited".into(), describe_exit);
            format!("{how}; starting again in {} s.", retry_in_s.unwrap_or(0))
        }
    }
}

pub fn describe_exit(e: &Exit) -> String {
    match (&e.error, e.code, e.signal) {
        (Some(err), _, _) => format!("cloudflared could not start: {err}"),
        _ if e.requested => "Restarted from the panel".into(),
        (None, Some(code), _) => format!("cloudflared exited with code {code}"),
        (None, None, Some(sig)) => format!("cloudflared ended by signal {sig}"),
        (None, None, None) => "cloudflared exited".into(),
    }
}

/// Every connection either source knows of: the log's events say up or down and the protocol,
/// the metrics hold the current edge location, RTT and byte counts per `conn_index`.
fn connections(i: &Inputs<'_>, m: &Metrics) -> Vec<Connection> {
    let located: Vec<(u32, String)> = m
        .series("cloudflared_tunnel_server_locations")
        .filter(|s| s.value >= 1.0)
        .filter_map(|s| {
            let index = s.label("connection_id")?.parse().ok()?;
            Some((index, s.label("edge_location")?.to_string()))
        })
        .collect();
    let per_conn = |name: &str, index: u32| {
        m.series(name)
            .find(|s| s.label("conn_index") == Some(index.to_string().as_str()))
            .map(|s| s.value)
    };
    let indexes: BTreeSet<u32> = i
        .log
        .links()
        .keys()
        .copied()
        .chain(located.iter().map(|(n, _)| *n))
        .collect();
    indexes
        .into_iter()
        .map(|index| {
            let link = i.log.links().get(&index);
            let up = link.map(|l| l.up);
            let location = located
                .iter()
                .find(|(n, _)| *n == index)
                .map(|(_, loc)| loc.clone())
                .or_else(|| link.and_then(|l| l.location.clone()));
            let since = link.and_then(|l| l.since);
            Connection {
                index,
                up,
                location,
                protocol: link.and_then(|l| l.protocol.clone()),
                since,
                for_s: since.map(|s| i.now.saturating_sub(s)),
                rtt_ms: per_conn("quic_client_smoothed_rtt", index)
                    .filter(|v| *v > 0.0 && up != Some(false)),
                sent_bytes: per_conn("quic_client_sent_bytes", index).map(|v| v as u64),
                received_bytes: per_conn("quic_client_receive_bytes", index).map(|v| v as u64),
                error: link.and_then(|l| l.error.clone()),
            }
        })
        .collect()
}

fn traffic(m: &Metrics) -> Traffic {
    let count = |name: &str| m.value(name).map(|v| v as u64);
    let mut responses: Vec<ResponseCount> = m
        .series("cloudflared_tunnel_response_by_code")
        .filter_map(|s| {
            Some(ResponseCount {
                code: s.label("status_code")?.to_string(),
                count: s.value as u64,
            })
        })
        .collect();
    responses.sort_by(|a, b| a.code.cmp(&b.code));
    Traffic {
        requests: count("cloudflared_tunnel_total_requests"),
        errors: count("cloudflared_tunnel_request_errors"),
        concurrent: count("cloudflared_tunnel_concurrent_requests_per_tunnel"),
        responses,
    }
}

/// The ingress rules as the dashboard configured them. Version -1 is cloudflared's placeholder
/// before the first configuration arrives.
fn routes(config: &Value) -> Routes {
    let received = config["version"].as_i64().is_some_and(|v| v >= 0);
    let rules = config["config"]["ingress"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let text = |v: &Value| v.as_str().filter(|s| !s.is_empty()).map(String::from);
    let mut hostnames = Vec::new();
    let mut fallback = None;
    for (n, rule) in rules.iter().enumerate() {
        let hostname = text(&rule["hostname"]);
        let path = text(&rule["path"]);
        let service = text(&rule["service"]).unwrap_or_default();
        if hostname.is_none() && path.is_none() && n + 1 == rules.len() {
            fallback = Some(service);
        } else {
            hostnames.push(Route {
                hostname: hostname.unwrap_or_else(|| "*".into()),
                path,
                service,
            });
        }
    }
    Routes {
        received,
        hostnames,
        fallback,
    }
}
