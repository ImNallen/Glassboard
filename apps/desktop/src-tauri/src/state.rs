use crate::{session::Session, Result};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

pub(crate) struct AppState(pub(crate) Mutex<Session>);

pub(crate) fn snapshot(app: &tauri::AppHandle) -> Session {
    app.state::<AppState>().0.lock().unwrap().clone()
}
pub(crate) fn publish(app: &tauri::AppHandle) -> Result<()> {
    app.emit("session", snapshot(app))
        .map_err(|e| e.to_string())
}
pub(crate) fn report(app: &tauri::AppHandle, error: String) {
    log::error!("{error}");
    app.state::<AppState>().0.lock().unwrap().error = Some(error);
    let _ = publish(app);
}
