use color_eyre::{Result, eyre::eyre};
use tesserae_core::terminal::{Snapshot, TerminalSession};

pub struct App {
    session: TerminalSession,
    pub exit: bool,
}

impl App {
    pub fn new(shell: &str, cols: u16, rows: u16) -> Result<Self> {
        let session =
            TerminalSession::spawn(shell, cols.max(1), rows.max(1)).map_err(|e| eyre!("{e}"))?;

        Ok(Self {
            session,
            exit: false,
        })
    }

    pub fn write(&self, bytes: &[u8]) -> Result<()> {
        self.session.write(bytes).map_err(|e| eyre!("{e}"))
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.session
            .resize(cols.max(1), rows.max(1))
            .map_err(|e| eyre!("{e}"))
    }

    pub fn scroll(&self, delta: i32) {
        self.session.scroll(delta);
    }

    pub fn snapshot(&self) -> Snapshot {
        self.session.snapshot()
    }

    pub fn quit(&mut self) {
        self.exit = true;
    }

    pub fn child_exited(&mut self) -> Result<bool> {
        self.session.child_exited().map_err(|e| eyre!("{e}"))
    }
}
