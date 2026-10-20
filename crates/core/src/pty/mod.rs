use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    env,
    io::{Read, Write},
    path::Path,
    sync::{Arc, Mutex},
    thread,
};

#[derive(Clone)]
pub struct PtyWriter(Arc<Mutex<Box<dyn Write + Send>>>);

impl PtyWriter {
    pub fn write(&self, data: &[u8]) -> Result<(), String> {
        let mut writer = self
            .0
            .lock()
            .map_err(|_| "pty writer lock poisoned".to_string())?;
        writer.write_all(data).map_err(|e| e.to_string())?;
        writer.flush().map_err(|e| e.to_string())
    }
}

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: PtyWriter,
    child: Box<dyn Child + Send + Sync>,
}

impl Drop for PtySession {
    fn drop(&mut self) {
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            let _ = self.child.kill();
        }
    }
}

impl PtySession {
    pub fn spawn(
        shell: &str,
        cols: u16,
        rows: u16,
        mut on_output: impl FnMut(&[u8]) + Send + 'static,
    ) -> Result<Self, String> {
        Self::spawn_with(shell, cols, rows, move |bytes, _writer| on_output(bytes))
    }

    pub fn spawn_with<F>(shell: &str, cols: u16, rows: u16, on_output: F) -> Result<Self, String>
    where
        F: FnMut(&[u8], &PtyWriter) + Send + 'static,
    {
        Self::spawn_with_cwd(shell, cols, rows, None, on_output)
    }

    pub fn spawn_with_cwd<F>(
        shell: &str,
        cols: u16,
        rows: u16,
        cwd: Option<&Path>,
        mut on_output: F,
    ) -> Result<Self, String>
    where
        F: FnMut(&[u8], &PtyWriter) + Send + 'static,
    {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_height: 0,
                pixel_width: 0,
            })
            .map_err(|e| e.to_string())?;

        let mut cmd = CommandBuilder::new(shell);
        cmd.env("TERM", "xterm-256color");
        cmd.env("SHELL", shell);
        cmd.env("TERM_PROGRAM", "tesserae");
        match cwd {
            Some(cwd) => {
                cmd.cwd(cwd);
            }
            None => {
                if let Some(home) = Self::home_dir() {
                    cmd.cwd(home);
                }
            }
        }
        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = PtyWriter(Arc::new(Mutex::new(
            pair.master.take_writer().map_err(|e| e.to_string())?,
        )));
        let thread_writer = writer.clone();

        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => on_output(&buf[..n], &thread_writer),
                }
            }
        });

        Ok(Self {
            master: pair.master,
            writer,
            child,
        })
    }

    pub fn writer(&self) -> PtyWriter {
        self.writer.clone()
    }

    pub fn write(&self, data: &[u8]) -> Result<(), String> {
        self.writer.write(data)
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_height: 0,
                pixel_width: 0,
            })
            .map_err(|e| e.to_string())
    }

    pub fn process_id(&self) -> Option<u32> {
        self.child.process_id()
    }

    fn home_dir() -> Option<String> {
        env::var("HOME").or_else(|_| env::var("USERPROFILE")).ok()
    }

    pub fn default_shell() -> String {
        #[cfg(windows)]
        {
            env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".into())
        }
        #[cfg(not(windows))]
        {
            env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into())
        }
    }

    pub fn child_exited(&mut self) -> Result<bool, String> {
        self.child
            .try_wait()
            .map(|status| status.is_some())
            .map_err(|e| e.to_string())
    }
}
