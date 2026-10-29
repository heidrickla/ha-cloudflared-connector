//! The supervisor with a shell script standing in for cloudflared.

use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use connector::options::Token;
use connector::status::Phase;
use connector::supervise::{Control, Shared, Supervisor};

// The supervisor writes the child's output to this process's stdout; tests share one stdout.
static SERIAL: Mutex<()> = Mutex::new(());

const TOKEN: &str = "test-token-a1b2c3";

fn serial() -> MutexGuard<'static, ()> {
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn start(script: &str, token: &str) -> (Arc<Shared>, Sender<Control>, JoinHandle<()>) {
    let shared = Arc::new(Shared::new(0, !token.is_empty()));
    let (tx, rx) = mpsc::channel();
    let sup = Supervisor {
        program: PathBuf::from("/bin/sh"),
        args: vec!["-c".into(), script.into()],
        token: Token::new(token.into()),
        shared: Arc::clone(&shared),
        tx: tx.clone(),
        rx,
        backoff: (Duration::from_millis(50), Duration::from_millis(200)),
        healthy_run: Duration::from_secs(60),
        grace: Duration::from_millis(300),
    };
    (shared, tx, thread::spawn(move || sup.run()))
}

fn until(what: &str, mut f: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !f() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(20));
    }
}

fn texts(shared: &Shared) -> Vec<String> {
    shared.log().lines().map(|l| l.text.clone()).collect()
}

#[test]
fn the_token_reaches_the_child_environment_and_not_its_arguments() {
    let _g = serial();
    let (shared, tx, handle) = start(
        "echo \"len=${#TUNNEL_TOKEN}\"; echo \"argv=$0 $*\"; exec sleep 30",
        TOKEN,
    );
    until("the child's output", || {
        texts(&shared).iter().any(|t| t.starts_with("argv="))
    });
    let lines = texts(&shared);
    assert!(lines.contains(&format!("len={}", TOKEN.len())), "{lines:?}");
    assert!(lines.iter().all(|t| !t.contains(TOKEN)), "{lines:?}");
    tx.send(Control::Stop).unwrap();
    handle.join().unwrap();
}

#[test]
fn an_exiting_child_is_started_again_with_backoff() {
    let _g = serial();
    let (shared, tx, handle) = start("echo run; exit 3", TOKEN);
    until("three restarts", || shared.supervision().restarts >= 3);
    let sup = shared.supervision().clone();
    let exit = sup.last_exit.unwrap();
    assert_eq!(exit.code, Some(3));
    assert!(!exit.requested);
    assert!(texts(&shared).iter().filter(|t| *t == "run").count() >= 3);
    tx.send(Control::Stop).unwrap();
    handle.join().unwrap();
}

#[test]
fn a_restart_request_ends_the_child_and_starts_it_at_once() {
    let _g = serial();
    let (shared, tx, handle) = start("echo up; exec sleep 30", TOKEN);
    until("the first run", || {
        matches!(shared.supervision().phase, Phase::Running { .. })
    });
    until("the child's output", || {
        texts(&shared).contains(&"up".to_string())
    });
    tx.send(Control::Restart).unwrap();
    until("the second run", || {
        texts(&shared).iter().filter(|t| *t == "up").count() == 2
    });
    let sup = shared.supervision().clone();
    assert_eq!(sup.restarts, 1);
    let exit = sup.last_exit.unwrap();
    assert!(exit.requested);
    assert_eq!(exit.signal, Some(libc::SIGTERM));
    tx.send(Control::Stop).unwrap();
    handle.join().unwrap();
}

#[test]
fn a_child_that_ignores_sigterm_is_killed_after_the_grace_time() {
    let _g = serial();
    let (shared, tx, handle) = start("trap '' TERM; echo up; while :; do sleep 1; done", TOKEN);
    until("the child's output", || {
        texts(&shared).contains(&"up".to_string())
    });
    let asked = Instant::now();
    tx.send(Control::Stop).unwrap();
    handle.join().unwrap();
    assert!(asked.elapsed() < Duration::from_secs(5));
    let exit = shared.supervision().last_exit.clone().unwrap();
    assert_eq!(exit.signal, Some(libc::SIGKILL));
    assert_eq!(shared.supervision().phase, Phase::Stopping);
}

#[test]
fn without_a_token_nothing_starts_and_stop_still_returns() {
    let _g = serial();
    let (shared, tx, handle) = start("echo should-not-run", "  ");
    thread::sleep(Duration::from_millis(200));
    assert_eq!(shared.supervision().phase, Phase::Unconfigured);
    let lines = texts(&shared);
    assert!(lines.iter().all(|t| t != "should-not-run"), "{lines:?}");
    assert!(lines.iter().any(|t| t.contains("tunnel_token is empty")));
    tx.send(Control::Restart).unwrap();
    tx.send(Control::Stop).unwrap();
    handle.join().unwrap();
}
