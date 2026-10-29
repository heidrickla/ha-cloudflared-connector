use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use connector::clock;
use connector::demo::Demo;
use connector::live::Live;
use connector::options::Options;
use connector::supervise::{Control, Shared, Supervisor, say};
use connector::web::{self, Peers};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

const OPTIONS: &str = "/data/options.json";
const CLOUDFLARED: &str = "/usr/local/bin/cloudflared";
/// cloudflared's local endpoints, on loopback so nothing else in the container network reaches them.
const METRICS: &str = "127.0.0.1:20241";
/// The ingress port in config.yaml.
const PANEL: &str = "0.0.0.0:8099";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] => run(),
        ["--version"] => {
            println!("connector {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        ["--demo"] => demo("127.0.0.1:8099"),
        ["--demo", addr] => demo(addr),
        _ => {
            eprintln!("usage: connector [--demo [ADDR] | --version]");
            ExitCode::from(2)
        }
    }
}

fn run() -> ExitCode {
    let options = match Options::read(Path::new(OPTIONS)) {
        Ok(o) => o,
        Err(e) => {
            say(&format!("cannot read {OPTIONS}: {e}"));
            return ExitCode::FAILURE;
        }
    };
    let shared = Arc::new(Shared::new(clock::now(), !options.tunnel_token.is_empty()));
    let (tx, rx) = mpsc::channel();

    match Signals::new([SIGTERM, SIGINT]) {
        Ok(mut signals) => {
            let tx = tx.clone();
            thread::spawn(move || {
                for _ in signals.forever() {
                    let _ = tx.send(Control::Stop);
                }
            });
        }
        Err(e) => say(&format!("cannot watch for stop signals: {e}")),
    }

    let live = Live {
        shared: Arc::clone(&shared),
        loglevel: options.loglevel.clone(),
        metrics_addr: METRICS.into(),
        control: tx.clone(),
    };
    // The tunnel matters more than the panel, so a panel that cannot listen is reported, not fatal.
    match web::serve(PANEL, Peers::Ingress, Arc::new(live)) {
        Ok(_) => say(&format!("panel listening on {PANEL} for the ingress proxy")),
        Err(e) => say(&format!("panel cannot listen on {PANEL}: {e}")),
    }

    say(&format!(
        "starting connector, loglevel={}",
        options.loglevel
    ));
    Supervisor {
        program: PathBuf::from(CLOUDFLARED),
        args: [
            "--no-autoupdate",
            "--loglevel",
            &options.loglevel,
            "--metrics",
            METRICS,
            "tunnel",
            "run",
        ]
        .map(String::from)
        .to_vec(),
        token: options.tunnel_token,
        shared,
        tx,
        rx,
        backoff: (Duration::from_secs(1), Duration::from_secs(60)),
        healthy_run: Duration::from_secs(60),
        grace: Duration::from_secs(10),
    }
    .run();
    say("stopped");
    ExitCode::SUCCESS
}

fn demo(addr: &str) -> ExitCode {
    match web::serve(addr, Peers::Loopback, Arc::new(Demo::default())) {
        Ok(handle) => {
            say(&format!("demo panel on http://{addr}/"));
            let _ = handle.join();
            ExitCode::SUCCESS
        }
        Err(e) => {
            say(&format!("cannot listen on {addr}: {e}"));
            ExitCode::FAILURE
        }
    }
}
