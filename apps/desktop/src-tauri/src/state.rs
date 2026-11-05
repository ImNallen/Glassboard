use crate::{i18n::Text, session::Session, Result};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

pub(crate) struct AppState(pub(crate) Mutex<Session>);

pub(crate) fn snapshot(app: &tauri::AppHandle) -> Session {
    app.state::<AppState>().0.lock().unwrap().clone()
}
pub(crate) fn publish(app: &tauri::AppHandle) -> Result<()> {
    Ok(app.emit("session", snapshot(app))?)
}
pub(crate) fn report(app: &tauri::AppHandle, error: impl Into<Text>) {
    let error = error.into();
    log::error!("{error}");
    app.state::<AppState>().0.lock().unwrap().error = Some(error);
    let _ = publish(app);
}
