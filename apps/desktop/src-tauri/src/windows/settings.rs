use super::geometry::{beside_icon, Rect, SETTINGS_SIZE};
use crate::{
    tray::{TrayAnchor, TRAY_ID},
    Result,
};
use tauri::Manager;

pub(crate) fn position_settings(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
) -> Result<()> {
    let rect = app
        .tray_by_id(TRAY_ID)
        .and_then(|tray| tray.rect().ok().flatten())
        .or_else(|| *app.state::<TrayAnchor>().0.lock().unwrap());
    // Tray rectangles are in the pixels of the display showing the icon.
    let icon = rect.map(|rect| {
        let p = rect.position.to_physical::<f64>(1.0);
        let s = rect.size.to_physical::<f64>(1.0);
        Rect {
            x: p.x,
            y: p.y,
            width: s.width,
            height: s.height,
        }
    });
    let monitor = icon
        .and_then(|icon| {
            let (x, y) = (icon.x + icon.width / 2.0, icon.y + icon.height / 2.0);
            app.available_monitors()
                .ok()?
                .into_iter()
                .find(|monitor| Rect::monitor(monitor).contains(x, y, 0.0))
        })
        .or(app.primary_monitor()?)
        .ok_or("No display is available for settings")?;
    let scale = monitor.scale_factor();
    let work = Rect::work_area(&monitor);
    let icon = icon.unwrap_or(Rect {
        x: work.x + work.width - 24.0 * scale,
        y: if cfg!(target_os = "macos") {
            work.y
        } else {
            work.y + work.height
        },
        width: 0.0,
        height: 0.0,
    });
    let size = work.fit(SETTINGS_SIZE, scale, 8.0, scale);
    Ok(beside_icon(icon, work, size, scale).place(window, scale)?)
}
