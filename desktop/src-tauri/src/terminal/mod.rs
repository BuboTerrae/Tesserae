use tauri::{
    State,
    ipc::{Channel, Response},
};
use tesserae_core::terminal::open_terminal;

#[derive(Default)]
pub struct AppState(Mutex<Option<PtySession>>);

#[tauri::command]
pub fn open_term() {
    open_terminal()
}
