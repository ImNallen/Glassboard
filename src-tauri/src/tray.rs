use crate::{commands::perform, state::report};
use std::sync::Mutex;
use tauri::Manager;

pub(crate) const TRAY_ID: &str = "glassboard-tray";
pub(crate) struct TrayAnchor(pub(crate) Mutex<Option<tauri::Rect>>);

pub(crate) fn create_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    let clear = MenuItem::with_id(app, "clear", "Clear screen", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Glassboard", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&clear, &quit])?;
    let mut pixels = vec![0u8; 22 * 22 * 4];
    for y in 3..19 {
        for x in 3..19 {
            if x == y || x == y + 1 || (y == 3 && x > 9) || (x == 18 && y < 12) {
                let i = (y * 22 + x) * 4;
                pixels[i..i + 4].copy_from_slice(&[220, 220, 220, 255]);
            }
        }
    }
    tauri::tray::TrayIconBuilder::with_id(TRAY_ID)
        .icon(tauri::image::Image::new_owned(pixels, 22, 22))
        .icon_as_template(true)
        .tooltip("Glassboard")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
            let open_settings = matches!(
                &event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            );
            let rect = match event {
                TrayIconEvent::Click { rect, .. }
                | TrayIconEvent::DoubleClick { rect, .. }
                | TrayIconEvent::Enter { rect, .. }
                | TrayIconEvent::Move { rect, .. }
                | TrayIconEvent::Leave { rect, .. } => rect,
                _ => return,
            };
            *tray.app_handle().state::<TrayAnchor>().0.lock().unwrap() = Some(rect);
            if open_settings {
                if let Err(error) = perform(tray.app_handle(), "settings") {
                    report(tray.app_handle(), error);
                }
            }
        })
        .on_menu_event(|app, event| {
            if let Err(e) = perform(app, event.id.as_ref()) {
                report(app, e);
            }
        })
        .build(app)?;
    Ok(())
}
