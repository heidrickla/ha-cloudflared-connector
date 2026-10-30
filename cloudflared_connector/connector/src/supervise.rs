//! cloudflared as a child process: started with the token in its environment, its output passed
//! through unchanged and kept in the log book, started again with backoff when it exits, and
//! stopped when the add-on stops.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::clock;
use crate::logbook::LogBook;
use crate::options::Token;
use crate::status::{Exit, NO_TOKEN, Phase, Supervision};

pub enum Control {
    Restart,
    Stop,
    Exited(Option<ExitStatus>),
}

/// State the supervisor writes and the panel reads.
#[derive(Default)]
pub struct Shared {
    supervision: Mutex<Supervision>,
    log: Mutex<LogBook>,
}

impl Shared {
    pub fn new(now: u64, configured: bool) -> Self {
        let phase = if configured {
            Phase::Waiting { until: now }
        } else {
            Phase::Unconfigured
        };
        Self {
            supervision: Mutex::new(Supervision {
                phase,
                connector_started: now,
                ..Supervision::default()
            }),
            log: Mutex::new(LogBook::default()),
        }
    }

    // A panic elsewhere must not stop the tunnel's supervision, so poisoned locks are entered.
    pub fn supervision(&self) -> MutexGuard<'_, Supervision> {
        self.supervision
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub fn log(&self) -> MutexGuard<'_, LogBook> {
        self.log.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub struct Supervisor {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub token: Token,
    pub shared: Arc<Shared>,
    pub tx: Sender<Control>,
    pub rx: Receiver<Control>,
    /// First and longest wait before starting cloudflared again.
    pub backoff: (Duration, Duration),
    /// A run this long resets the backoff.
    pub healthy_run: Duration,
    /// Time from SIGTERM to SIGKILL.
    pub grace: Duration,
}

#[derive(Clone, Copy, PartialEq)]
enum Ending {
    ByItself,
    Requested,
    Stopped,
}

pub const PREFIX: &str = "[cloudflared-connector]";

pub fn say(message: &str) {
    println!("{PREFIX} {message}");
}

impl Supervisor {
    /// Prints a message of the connector's own to the add-on log and keeps it for the panel.
    fn note(&self, message: &str) {
        say(message);
        self.shared.log().push(&format!("{PREFIX} {message}"));
    }

    /// Runs until a stop request; cloudflared is stopped before it returns.
    pub fn run(self) {
        if self.token.is_empty() {
            self.note(NO_TOKEN);
            self.shared.supervision().phase = Phase::Unconfigured;
            while !matches!(self.rx.recv(), Ok(Control::Stop) | Err(_)) {}
            return;
        }
        let mut wait = self.backoff.0;
        let mut first = true;
        loop {
            if !first {
                self.shared.supervision().restarts += 1;
            }
            first = false;
            let started = Instant::now();
            let ending = match self.spawn() {
                Ok(pid) => {
                    self.shared.supervision().phase = Phase::Running {
                        since: clock::now(),
                    };
                    let (status, ending) = self.wait_for_exit(pid);
                    self.record_exit(status, ending == Ending::Requested, None);
                    ending
                }
                Err(e) => {
                    self.note(&format!("cannot start cloudflared: {e}"));
                    self.record_exit(None, false, Some(e.to_string()));
                    Ending::ByItself
                }
            };
            match ending {
                Ending::Stopped => return,
                Ending::Requested => {
                    self.note("restarting cloudflared, as asked from the panel");
                    wait = self.backoff.0;
                    continue;
                }
                Ending::ByItself => {}
            }
            if started.elapsed() >= self.healthy_run {
                wait = self.backoff.0;
            }
            self.note(&format!(
                "cloudflared exited; starting it again in {} s",
                wait.as_secs()
            ));
            self.shared.supervision().phase = Phase::Waiting {
                until: clock::now() + wait.as_secs(),
            };
            if !self.pause(wait) {
                return;
            }
            wait = (wait * 2).min(self.backoff.1);
        }
    }

    fn spawn(&self) -> io::Result<u32> {
        self.note(&format!("starting cloudflared {}", self.args.join(" ")));
        let mut child = Command::new(&self.program)
            .args(&self.args)
            .env("TUNNEL_TOKEN", self.token.expose())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        self.shared.log().new_run();
        if let Some(out) = child.stdout.take() {
            pump(out, io::stdout, Arc::clone(&self.shared));
        }
        if let Some(err) = child.stderr.take() {
            pump(err, io::stderr, Arc::clone(&self.shared));
        }
        let pid = child.id();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let _ = tx.send(Control::Exited(child.wait().ok()));
        });
        Ok(pid)
    }

    /// Waits for cloudflared to exit, acting on restart and stop requests meanwhile.
    fn wait_for_exit(&self, pid: u32) -> (Option<ExitStatus>, Ending) {
        let mut ending = Ending::ByItself;
        let mut kill_at: Option<Instant> = None;
        loop {
            let message = match kill_at {
                Some(at) => self
                    .rx
                    .recv_timeout(at.saturating_duration_since(Instant::now())),
                None => self.rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            match message {
                Ok(Control::Exited(status)) => return (status, ending),
                Ok(Control::Restart) if ending == Ending::ByItself => {
                    ending = Ending::Requested;
                    signal(pid, libc::SIGTERM);
                    kill_at = Some(Instant::now() + self.grace);
                }
                Ok(Control::Stop) | Err(RecvTimeoutError::Disconnected)
                    if ending != Ending::Stopped =>
                {
                    ending = Ending::Stopped;
                    self.shared.supervision().phase = Phase::Stopping;
                    signal(pid, libc::SIGTERM);
                    kill_at.get_or_insert(Instant::now() + self.grace);
                }
                Err(RecvTimeoutError::Timeout) => {
                    signal(pid, libc::SIGKILL);
                    kill_at = None;
                }
                Ok(_) | Err(RecvTimeoutError::Disconnected) => {}
            }
        }
    }

    /// Waits out the backoff. False when a stop request came meanwhile.
    fn pause(&self, wait: Duration) -> bool {
        let until = Instant::now() + wait;
        loop {
            match self
                .rx
                .recv_timeout(until.saturating_duration_since(Instant::now()))
            {
                Ok(Control::Stop) | Err(RecvTimeoutError::Disconnected) => return false,
                Ok(Control::Restart) | Err(RecvTimeoutError::Timeout) => return true,
                Ok(Control::Exited(_)) => {}
            }
        }
    }

    fn record_exit(&self, status: Option<ExitStatus>, requested: bool, error: Option<String>) {
        use std::os::unix::process::ExitStatusExt;
        self.shared.supervision().last_exit = Some(Exit {
            at: clock::now(),
            code: status.and_then(|s| s.code()),
            signal: status.and_then(|s| s.signal()),
            requested,
            error,
        });
    }
}

fn signal(pid: u32, sig: libc::c_int) {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return;
    };
    // SAFETY: kill(2) takes plain integers and touches no memory of this process.
    unsafe {
        libc::kill(pid, sig);
    }
}

/// Copies a child stream to ours byte for byte, keeping each line in the log book.
fn pump<R, W, F>(from: R, to: F, shared: Arc<Shared>)
where
    R: Read + Send + 'static,
    W: Write,
    F: Fn() -> W + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = BufReader::new(from);
        let mut line = Vec::new();
        loop {
            line.clear();
            match reader.read_until(b'\n', &mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) => {
                    let mut out = to();
                    let _ = out.write_all(&line).and_then(|()| out.flush());
                    let text = String::from_utf8_lossy(&line);
                    shared.log().push(text.trim_end_matches(['\n', '\r']));
                }
            }
        }
    });
}
