#![cfg(unix)]

use std::{
    thread,
    time::{Duration, Instant},
};

use tesserae_core::terminal::TerminalSession;

fn contains(session: &TerminalSession, needle: &str) -> bool {
    let snapshot = session.snapshot();
    (0..snapshot.rows).any(|y| {
        snapshot
            .line(y)
            .iter()
            .map(|cell| cell.ch)
            .collect::<String>()
            .contains(needle)
    })
}

fn wait_for(session: &TerminalSession, needle: &str) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if contains(session, needle) {
            return true;
        }
        thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn pty_output_reaches_emulator() {
    let session = TerminalSession::spawn("/bin/cat", 20, 5).unwrap();
    session.write(b"hello\n").unwrap();
    assert!(wait_for(&session, "hello"), "timed out waiting for output");
}

#[test]
fn resize_updates_snapshot() {
    let session = TerminalSession::spawn("/bin/cat", 20, 5).unwrap();
    session.resize(40, 10).unwrap();
    let snapshot = session.snapshot();
    assert_eq!(snapshot.cols, 40);
    assert_eq!(snapshot.rows, 10);
}

#[test]
fn detects_child_exit() {
    let mut session = TerminalSession::spawn("/bin/sh", 20, 5).unwrap();
    session.write(b"exit\n").unwrap();

    let mut exited = false;
    for _ in 0..200 {
        if session.child_exited().unwrap() {
            exited = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert!(exited);
}
