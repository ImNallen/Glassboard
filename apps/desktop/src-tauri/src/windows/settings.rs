use crate::{
    settings_position::{anchored_position, Bounds},
    tray::{TrayAnchor, TRAY_ID},
    Result,
};
use tauri::{Manager, PhysicalPosition, PhysicalSize};

pub(super) const SETTINGS_WIDTH: f64 = 380.0;
pub(super) const SETTINGS_HEIGHT: f64 = 620.0;

pub(crate) fn position_settings(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
) -> Result<()> {
    let rect = app
        .tray_by_id(TRAY_ID)
        .and_then(|tray| tray.rect().ok().flatten())
        .or_else(|| *app.state::<TrayAnchor>().0.lock().unwrap());
    // Tray rectangles are physical coordinates on both macOS and Windows.
    let icon = rect.map(|rect| {
        let p = rect.position.to_physical::<f64>(1.0);
        let s = rect.size.to_physical::<f64>(1.0);
        Bounds {
            x: p.x,
            y: p.y,
            width: s.width,
            height: s.height,
        }
    });
    let monitor = icon
        .and_then(|icon| {
            app.monitor_from_point(icon.x + icon.width / 2.0, icon.y + icon.height / 2.0)
                .ok()
                .flatten()
        })
        .or(app.primary_monitor().map_err(|e| e.to_string())?)
        .ok_or("No display is available for settings")?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let work = Bounds {
        x: area.position.x as f64,
        y: area.position.y as f64,
        width: area.size.width as f64,
        height: area.size.height as f64,
    };
    let icon = icon.unwrap_or(Bounds {
        x: work.x + work.width - 24.0 * scale,
        y: if cfg!(target_os = "macos") {
            work.y
        } else {
            work.y + work.height
        },
        width: 0.0,
        height: 0.0,
    });
    let size = (
        (SETTINGS_WIDTH * scale).min((work.width - 8.0 * scale).max(scale)),
        (SETTINGS_HEIGHT * scale).min((work.height - 8.0 * scale).max(scale)),
    );
    let (x, y) = anchored_position(icon, work, size, scale);
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    window
        .set_size(PhysicalSize::new(
            size.0.round() as u32,
            size.1.round() as u32,
        ))
        .map_err(|e| e.to_string())
}
