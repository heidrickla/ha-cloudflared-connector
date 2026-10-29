//! The panel fed from example fixtures, for screenshots and for trying the page without a tunnel.
//! Nothing in the fixtures belongs to a real tunnel.

use std::sync::{Mutex, PoisonError};

use crate::cloudflared::Readings;
use crate::logbook::LogBook;
use crate::status::{self, Inputs, Phase, Status, Supervision};
use crate::web::Source;

pub const METRICS: &str = include_str!("../fixtures/demo/metrics.txt");
pub const READY: &str = include_str!("../fixtures/demo/ready.json");
pub const CONFIG: &str = include_str!("../fixtures/demo/remote-config.json");
pub const LOG: &str = include_str!("../fixtures/demo/log.txt");

/// The fixtures' present: 2026-10-29T15:00:00Z.
pub const NOW: u64 = 1_793_286_000;
/// cloudflared's start in the fixture log, 2026-10-26T18:53:28Z.
pub const CLOUDFLARED_SINCE: u64 = 1_793_040_808;

pub fn readings() -> Readings {
    Readings {
        metrics: Ok(METRICS.into()),
        ready: Ok(READY.into()),
        config: Ok(CONFIG.into()),
    }
}

pub fn log() -> LogBook {
    let mut book = LogBook::default();
    for line in LOG.lines() {
        book.push(line);
    }
    book
}

pub fn supervision(restarts: u32) -> Supervision {
    Supervision {
        phase: Phase::Running {
            since: CLOUDFLARED_SINCE,
        },
        connector_started: CLOUDFLARED_SINCE - 1,
        restarts,
        last_exit: None,
    }
}

pub fn status(restarts: u32) -> Status {
    status::build(&Inputs {
        now: NOW,
        loglevel: "info",
        supervision: &supervision(restarts),
        log: &log(),
        readings: Some(&readings()),
    })
}

/// A restart in the demo only counts; nothing runs.
#[derive(Default)]
pub struct Demo {
    restarts: Mutex<u32>,
}

impl Source for Demo {
    fn status(&self) -> Status {
        status(*self.restarts.lock().unwrap_or_else(PoisonError::into_inner))
    }

    fn restart(&self) -> Result<(), String> {
        *self.restarts.lock().unwrap_or_else(PoisonError::into_inner) += 1;
        Ok(())
    }
}
