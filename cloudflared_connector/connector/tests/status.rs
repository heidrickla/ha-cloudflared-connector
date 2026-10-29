//! The status model fed from fixtures: `recorded/` is cloudflared 2026.10.4 run with a dummy token,
//! `demo/` is a connected tunnel composed in the same formats.

use connector::clock;
use connector::cloudflared::Readings;
use connector::demo;
use connector::logbook::LogBook;
use connector::status::{
    self, Exit, Inputs, Phase, ResponseCount, Route, State, Status, Supervision,
};

const REC_METRICS: &str = include_str!("../fixtures/recorded/metrics.txt");
const REC_READY: &str = include_str!("../fixtures/recorded/ready.json");
const REC_CONFIG: &str = include_str!("../fixtures/recorded/remote-config.json");
const REC_LOG: &str = include_str!("../fixtures/recorded/log.txt");

fn recorded() -> Readings {
    Readings {
        metrics: Ok(REC_METRICS.into()),
        ready: Ok(REC_READY.into()),
        config: Ok(REC_CONFIG.into()),
    }
}

fn book(text: &str) -> LogBook {
    let mut b = LogBook::default();
    for line in text.lines() {
        b.push(line);
    }
    b
}

fn running(since: u64) -> Supervision {
    Supervision {
        phase: Phase::Running { since },
        connector_started: since,
        ..Supervision::default()
    }
}

fn build(now: u64, sup: &Supervision, log: &LogBook, readings: Option<&Readings>) -> Status {
    status::build(&Inputs {
        now,
        loglevel: "info",
        supervision: sup,
        log,
        readings,
    })
}

#[test]
fn the_demo_clock_matches_its_fixture_log() {
    assert_eq!(clock::parse_utc("2026-10-29T15:00:00Z"), Some(demo::NOW));
    assert_eq!(
        clock::parse_utc("2026-10-26T18:53:28Z"),
        Some(demo::CLOUDFLARED_SINCE)
    );
}

#[test]
fn a_connected_tunnel_reports_four_connections_traffic_and_hostnames() {
    let s = demo::status(0);
    assert_eq!(s.state, State::Connected);
    assert_eq!(s.headline, "4 of 4 edge connections registered");
    assert_eq!(s.cloudflared.version.as_deref(), Some("2026.10.4"));
    assert_eq!(s.cloudflared.go_version.as_deref(), Some("go1.27.3"));
    assert_eq!(
        s.cloudflared.connector_id.as_deref(),
        Some("3f6c2b1e-8d4a-4c7e-9b2f-5a1d0e6c7b84")
    );
    assert_eq!(s.cloudflared.config_version, Some(7));
    assert_eq!(s.connections.ready, 4);
    assert!(s.notices.is_empty(), "{:?}", s.notices);

    let links = &s.connections.links;
    assert_eq!(
        links.iter().map(|l| l.index).collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    assert!(links.iter().all(|l| l.up == Some(true)));
    assert!(links.iter().all(|l| l.protocol.as_deref() == Some("quic")));
    // The metrics' current location wins; connection 1 moved to iad08 when it reconnected.
    let locations: Vec<_> = links
        .iter()
        .map(|l| l.location.as_deref().unwrap())
        .collect();
    assert_eq!(locations, ["ord02", "iad08", "ord05", "iad08"]);
    assert_eq!(links[1].since, clock::parse_utc("2026-10-29T13:07:22Z"));
    assert_eq!(links[1].for_s, Some(demo::NOW - links[1].since.unwrap()));
    assert_eq!(links[0].rtt_ms, Some(14.0));
    assert_eq!(links[0].sent_bytes, Some(2_411_950_383));
    assert_eq!(links[3].received_bytes, Some(21_877_409));

    assert_eq!(s.traffic.requests, Some(18_342));
    assert_eq!(s.traffic.errors, Some(27));
    assert_eq!(s.traffic.concurrent, Some(3));
    assert_eq!(s.traffic.responses.len(), 7);
    assert_eq!(
        s.traffic.responses[1],
        ResponseCount {
            code: "200".into(),
            count: 15_873
        }
    );

    assert!(s.routes.received);
    assert_eq!(s.routes.fallback.as_deref(), Some("http_status:404"));
    assert_eq!(s.routes.hostnames.len(), 5);
    assert_eq!(
        s.routes.hostnames[0],
        Route {
            hostname: "ha.example.com".into(),
            path: Some("^/api/webhook/".into()),
            service: "http://homeassistant:8123".into()
        }
    );
    assert_eq!(s.routes.hostnames[4].service, "ssh://homeassistant:22");
    assert_eq!(s.log.len(), demo::LOG.lines().count());
    assert_eq!(
        s.process.uptime_s,
        Some(demo::NOW - demo::CLOUDFLARED_SINCE)
    );
}

#[test]
fn a_rejected_token_reads_as_disconnected_with_no_connections_or_routes() {
    // Recorded with a dummy token: cloudflared is up and serving its endpoints, the edge refuses
    // the registration, and the configuration is the placeholder.
    let log = book(REC_LOG);
    let since = clock::parse_utc("2026-10-29T21:42:29Z").unwrap();
    let s = build(since + 120, &running(since), &log, Some(&recorded()));
    assert_eq!(s.state, State::Disconnected);
    assert_eq!(
        s.headline,
        "0 of 4 edge connections registered; cloudflared keeps retrying."
    );
    assert_eq!(s.cloudflared.version.as_deref(), Some("2026.10.4"));
    assert!(s.cloudflared.connector_id.is_some());
    assert_eq!(s.cloudflared.config_version, None);
    assert_eq!(s.connections.ready, 0);
    assert!(s.connections.links.is_empty());
    assert!(!s.routes.received);
    assert!(s.routes.hostnames.is_empty());
    assert_eq!(s.routes.fallback.as_deref(), Some("http_status:503"));
    assert_eq!(s.traffic.requests, Some(0));
    assert!(s.traffic.responses.is_empty());
    assert!(
        s.log
            .iter()
            .any(|l| l.text.contains("Failed to get tunnel"))
    );
}

#[test]
fn a_fresh_start_is_starting_then_disconnected() {
    let log = LogBook::default();
    let readings = recorded();
    let s = build(1_000 + 5, &running(1_000), &log, Some(&readings));
    assert_eq!(s.state, State::Starting);
    let s = build(1_000 + 60, &running(1_000), &log, Some(&readings));
    assert_eq!(s.state, State::Disconnected);
}

#[test]
fn some_connections_is_degraded() {
    let readings = Readings {
        ready: Ok(r#"{"status":200,"readyConnections":2,"connectorId":"x"}"#.into()),
        ..recorded()
    };
    let s = build(2_000, &running(1_000), &LogBook::default(), Some(&readings));
    assert_eq!(s.state, State::Degraded);
    assert_eq!(s.headline, "2 of 4 edge connections registered");
}

#[test]
fn unreadable_endpoints_are_reported_once_cloudflared_has_settled() {
    let readings = Readings {
        metrics: Err("connection refused".into()),
        ready: Err("connection refused".into()),
        config: Ok(REC_CONFIG.into()),
    };
    let early = build(1_005, &running(1_000), &LogBook::default(), Some(&readings));
    assert!(early.notices.is_empty());
    let late = build(1_100, &running(1_000), &LogBook::default(), Some(&readings));
    assert_eq!(
        late.notices,
        [
            "cloudflared's /metrics did not answer: connection refused",
            "cloudflared's /ready did not answer: connection refused"
        ]
    );
    assert_eq!(late.cloudflared.version, None);
}

#[test]
fn without_a_token_the_panel_says_how_to_set_one() {
    let sup = Supervision::default();
    let s = build(10, &sup, &LogBook::default(), None);
    assert_eq!(s.state, State::Unconfigured);
    assert!(s.headline.contains("tunnel_token"));
    assert!(!s.process.can_restart);
    assert!(!s.process.running);
}

#[test]
fn a_waiting_restart_says_why_and_when() {
    let sup = Supervision {
        phase: Phase::Waiting { until: 1_008 },
        connector_started: 900,
        restarts: 3,
        last_exit: Some(Exit {
            at: 1_000,
            code: Some(1),
            signal: None,
            requested: false,
            error: None,
        }),
    };
    // Readings offered while cloudflared is not running are ignored.
    let s = build(1_002, &sup, &book(demo::LOG), Some(&demo::readings()));
    assert_eq!(s.state, State::Restarting);
    assert_eq!(
        s.headline,
        "cloudflared exited with code 1; starting again in 6 s."
    );
    assert_eq!(s.process.retry_in_s, Some(6));
    assert_eq!(s.process.restarts, 3);
    assert_eq!(
        s.process.last_exit_text.as_deref(),
        Some("cloudflared exited with code 1")
    );
    assert!(s.process.can_restart);
    assert!(s.connections.links.is_empty());
    assert_eq!(s.connections.ready, 0);
    assert!(!s.routes.received);
}
