//! The Cloudflared Connector add-on's one process. It runs cloudflared as its child, keeps what
//! cloudflared prints, and serves the ingress panel from one status model built from cloudflared's
//! local endpoints and its own supervision.

pub mod clock;
pub mod cloudflared;
pub mod demo;
pub mod live;
pub mod logbook;
pub mod metrics;
pub mod options;
pub mod status;
pub mod supervise;
pub mod web;
