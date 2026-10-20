use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use tesserae_core::pty::PtySession;

#[test]
fn streams_shell_output() {
    let (tx, rx) = mpsc::channel();
    let mut session = PtySession::spawn("/bin/sh", 80, 24, move |b| {
        let _ = tx.send(b.to_vec());
    })
    .unwrap();

    session.write(b"printf hello\n").unwrap();

    let mut seen = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Ok(chunk) = rx.recv_timeout(Duration::from_millis(100)) {
            seen.extend_from_slice(&chunk);
            if seen.windows(5).any(|w| w == b"hello") {
                break;
            }
        }
    }
    assert!(
        seen.windows(5).any(|w| w == b"hello"),
        "output was: {seen:?}"
    );
}
