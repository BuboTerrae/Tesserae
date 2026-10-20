use crate::{
    pty::PtySession,
    terminal::{AlacrittyEmulator, Emulator, Snapshot, osc::Osc7Scanner},
};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub struct TerminalSession {
    pty: PtySession,
    emulator: Arc<Mutex<Box<dyn Emulator>>>,
    reported_cwd: Arc<Mutex<Option<PathBuf>>>,
}

impl TerminalSession {
    pub fn spawn(shell: &str, cols: u16, rows: u16) -> Result<Self, String> {
        Self::spawn_in(shell, cols, rows, None)
    }

    pub fn spawn_in(shell: &str, cols: u16, rows: u16, cwd: Option<&Path>) -> Result<Self, String> {
        let emulator: Arc<Mutex<Box<dyn Emulator>>> =
            Arc::new(Mutex::new(Box::new(AlacrittyEmulator::new(cols, rows))));
        let reported_cwd = Arc::new(Mutex::new(None));

        let emulator_for_read = Arc::clone(&emulator);
        let cwd_for_read = Arc::clone(&reported_cwd);
        let mut osc = Osc7Scanner::default();

        let pty = PtySession::spawn_with_cwd(shell, cols, rows, cwd, move |bytes, writer| {
            if let Some(path) = osc.feed(bytes) {
                if let Ok(mut current) = cwd_for_read.lock() {
                    *current = Some(path);
                }
            }

            let responses = match emulator_for_read.lock() {
                Ok(mut emulator) => {
                    emulator.feed(bytes);
                    emulator.take_responses()
                }
                Err(_) => Vec::new(),
            };

            if !responses.is_empty() {
                let _ = writer.write(&responses);
            }
        })?;

        Ok(Self {
            pty,
            emulator,
            reported_cwd,
        })
    }

    pub fn write(&self, data: &[u8]) -> Result<(), String> {
        self.pty.write(data)
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        self.emulator
            .lock()
            .map_err(|_| "emulator lock poisoned".to_string())?
            .resize(cols, rows);
        self.pty.resize(cols, rows)
    }

    pub fn scroll(&self, delta: i32) {
        if let Ok(mut emulator) = self.emulator.lock() {
            emulator.scroll(delta);
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.emulator
            .lock()
            .expect("emulator lock poisoned")
            .snapshot()
    }

    pub fn process_id(&self) -> Option<u32> {
        self.pty.process_id()
    }

    pub fn reported_cwd(&self) -> Option<PathBuf> {
        self.reported_cwd.lock().ok().and_then(|cwd| cwd.clone())
    }

    pub fn child_exited(&mut self) -> Result<bool, String> {
        self.pty.child_exited()
    }
}
