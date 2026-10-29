//! The panel's routes and who may reach them.

use std::net::SocketAddr;

use connector::demo::Demo;
use connector::web::{Peers, Source, respond};
use tiny_http::Method;

fn addr(s: &str) -> SocketAddr {
    s.parse().unwrap()
}

#[test]
fn only_the_ingress_proxy_is_answered_in_the_add_on() {
    let ingress = Peers::Ingress;
    assert!(ingress.allows(Some(&addr("172.30.32.2:51000"))));
    assert!(ingress.allows(Some(&addr("[::ffff:172.30.32.2]:51000"))));
    for other in ["172.30.32.1:1", "172.30.33.9:1", "127.0.0.1:1", "[::1]:1"] {
        assert!(!ingress.allows(Some(&addr(other))), "{other}");
    }
    assert!(!ingress.allows(None));
}

#[test]
fn the_demo_answers_loopback_only() {
    assert!(Peers::Loopback.allows(Some(&addr("127.0.0.1:1"))));
    assert!(Peers::Loopback.allows(Some(&addr("[::1]:1"))));
    assert!(!Peers::Loopback.allows(Some(&addr("172.30.32.2:1"))));
}

#[test]
fn the_page_and_its_assets_are_served_under_relative_paths() {
    let demo = Demo::default();
    let page = respond(&Method::Get, "/", None, &demo);
    assert_eq!(page.code, 200);
    let html = String::from_utf8(page.body).unwrap();
    for asset in ["href=\"style.css\"", "src=\"app.js\""] {
        assert!(html.contains(asset), "{asset}");
    }
    assert!(!html.contains("href=\"/") && !html.contains("src=\"/"));
    let script = respond(&Method::Get, "/app.js", None, &demo);
    let script = String::from_utf8(script.body).unwrap();
    assert!(script.contains("fetch('api/status'") && script.contains("fetch('api/restart'"));
    assert_eq!(respond(&Method::Get, "/style.css", None, &demo).code, 200);
    assert_eq!(respond(&Method::Get, "/nope", None, &demo).code, 404);
}

#[test]
fn status_is_json_from_the_model() {
    let demo = Demo::default();
    let reply = respond(&Method::Get, "/api/status?t=1", None, &demo);
    assert_eq!(reply.code, 200);
    assert_eq!(reply.content_type, "application/json");
    let v: serde_json::Value = serde_json::from_slice(&reply.body).unwrap();
    assert_eq!(v["state"], "connected");
    assert_eq!(v["connections"]["links"].as_array().unwrap().len(), 4);
}

#[test]
fn a_restart_needs_post_and_the_action_header() {
    let demo = Demo::default();
    assert_eq!(respond(&Method::Get, "/api/restart", None, &demo).code, 405);
    assert_eq!(
        respond(&Method::Post, "/api/restart", None, &demo).code,
        403
    );
    assert_eq!(
        respond(&Method::Post, "/api/restart", Some("other"), &demo).code,
        403
    );
    assert_eq!(demo.status().process.restarts, 0);
    assert_eq!(
        respond(&Method::Post, "/api/restart", Some("restart"), &demo).code,
        202
    );
    assert_eq!(demo.status().process.restarts, 1);
}
