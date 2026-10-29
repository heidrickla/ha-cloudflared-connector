//! The panel's HTTP server. Home Assistant's ingress proxy is the only peer it answers; the page
//! uses relative URLs so it works under the ingress path.

use std::io::{self, Cursor};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use tiny_http::{Header, Method, Request, Response, Server};

use crate::status::Status;

/// The Supervisor's ingress proxy, per Home Assistant's add-on documentation.
pub const INGRESS_PROXY: IpAddr = IpAddr::V4(Ipv4Addr::new(172, 30, 32, 2));
/// Header the page sends with a restart, which a cross-site form cannot set.
pub const ACTION_HEADER: &str = "X-Connector-Action";

const INDEX: &str = include_str!("../web/index.html");
const SCRIPT: &str = include_str!("../web/app.js");
const STYLE: &str = include_str!("../web/style.css");

pub trait Source: Send + Sync {
    fn status(&self) -> Status;
    fn restart(&self) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Peers {
    /// The add-on: the ingress proxy only.
    Ingress,
    /// The demo: loopback only.
    Loopback,
}

impl Peers {
    pub fn allows(self, peer: Option<&SocketAddr>) -> bool {
        let Some(ip) = peer.map(SocketAddr::ip) else {
            return false;
        };
        let ip = match ip {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(ip, IpAddr::V4),
            v4 => v4,
        };
        match self {
            Peers::Ingress => ip == INGRESS_PROXY,
            Peers::Loopback => ip.is_loopback(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Reply {
    pub code: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

impl Reply {
    fn new(code: u16, content_type: &'static str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            code,
            content_type,
            body: body.into(),
        }
    }

    fn text(code: u16, body: &str) -> Self {
        Self::new(code, "text/plain; charset=utf-8", body)
    }

    fn json(code: u16, value: &impl serde::Serialize) -> Self {
        match serde_json::to_vec(value) {
            Ok(body) => Self::new(code, "application/json", body),
            Err(e) => Self::text(500, &e.to_string()),
        }
    }
}

/// Answers one request. `action_header` is the value of [`ACTION_HEADER`], if sent.
pub fn respond(
    method: &Method,
    url: &str,
    action_header: Option<&str>,
    source: &dyn Source,
) -> Reply {
    let path = url.split(['?', '#']).next().unwrap_or("/");
    match (method, path) {
        (Method::Get, "/" | "/index.html") => Reply::new(200, "text/html; charset=utf-8", INDEX),
        (Method::Get, "/app.js") => Reply::new(200, "text/javascript; charset=utf-8", SCRIPT),
        (Method::Get, "/style.css") => Reply::new(200, "text/css; charset=utf-8", STYLE),
        (Method::Get, "/api/status") => Reply::json(200, &source.status()),
        (Method::Post, "/api/restart") if action_header == Some("restart") => {
            match source.restart() {
                Ok(()) => Reply::json(202, &serde_json::json!({"restarting": true})),
                Err(e) => Reply::json(409, &serde_json::json!({"error": e})),
            }
        }
        (Method::Post, "/api/restart") => Reply::text(403, "missing action header"),
        (_, "/" | "/index.html" | "/app.js" | "/style.css" | "/api/status" | "/api/restart") => {
            Reply::text(405, "method not allowed")
        }
        _ => Reply::text(404, "not found"),
    }
}

/// Binds the panel's listener and serves it on its own thread, so a fault in the panel cannot stop
/// the supervisor.
pub fn serve(addr: &str, peers: Peers, source: Arc<dyn Source>) -> io::Result<JoinHandle<()>> {
    let server = Server::http(addr).map_err(io::Error::other)?;
    Ok(thread::spawn(move || {
        for request in server.incoming_requests() {
            handle(request, peers, source.as_ref());
        }
    }))
}

fn handle(request: Request, peers: Peers, source: &dyn Source) {
    let reply = if peers.allows(request.remote_addr()) {
        let action = request
            .headers()
            .iter()
            .find(|h| h.field.equiv(ACTION_HEADER))
            .map(|h| h.value.as_str().to_string());
        let (method, url) = (request.method().clone(), request.url().to_string());
        catch_unwind(AssertUnwindSafe(|| {
            respond(&method, &url, action.as_deref(), source)
        }))
        .unwrap_or_else(|_| Reply::text(500, "the panel failed to answer"))
    } else {
        Reply::text(403, "only Home Assistant's ingress proxy is answered")
    };
    let _ = request.respond(to_response(reply));
}

fn to_response(reply: Reply) -> Response<Cursor<Vec<u8>>> {
    let headers = [
        ("Content-Type", reply.content_type),
        ("Cache-Control", "no-store"),
        ("X-Content-Type-Options", "nosniff"),
        (
            "Content-Security-Policy",
            "default-src 'self'; img-src 'self' data:; base-uri 'none'; form-action 'none'",
        ),
    ];
    let mut response = Response::from_data(reply.body).with_status_code(reply.code);
    for (k, v) in headers {
        if let Ok(h) = Header::from_bytes(k.as_bytes(), v.as_bytes()) {
            response.add_header(h);
        }
    }
    response
}
