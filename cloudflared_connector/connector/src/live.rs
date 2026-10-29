//! The panel's source in the add-on: the supervisor's shared state and cloudflared's endpoints,
//! read when the page asks.

use std::sync::Arc;
use std::sync::mpsc::Sender;

use crate::clock;
use crate::cloudflared::Readings;
use crate::status::{self, Inputs, Phase, Status};
use crate::supervise::{Control, Shared};
use crate::web::Source;

pub struct Live {
    pub shared: Arc<Shared>,
    pub loglevel: String,
    pub metrics_addr: String,
    pub control: Sender<Control>,
}

impl Source for Live {
    fn status(&self) -> Status {
        let supervision = self.shared.supervision().clone();
        let readings = matches!(supervision.phase, Phase::Running { .. })
            .then(|| Readings::read(&self.metrics_addr));
        let log = self.shared.log();
        status::build(&Inputs {
            now: clock::now(),
            loglevel: &self.loglevel,
            supervision: &supervision,
            log: &log,
            readings: readings.as_ref(),
        })
    }

    fn restart(&self) -> Result<(), String> {
        match self.shared.supervision().phase {
            Phase::Unconfigured => return Err("no tunnel token is set".into()),
            Phase::Stopping => return Err("the add-on is stopping".into()),
            Phase::Running { .. } | Phase::Waiting { .. } => {}
        }
        self.control
            .send(Control::Restart)
            .map_err(|_| "the supervisor has stopped".to_string())
    }
}
