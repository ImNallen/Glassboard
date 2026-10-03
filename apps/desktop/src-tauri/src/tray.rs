use crate::{
    commands::{perform, transition},
    session::Transition,
    state::report,
    updates::UpdateStatus,
};
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    AppHandle, Manager,
};

pub(crate) const TRAY_ID: &str = "glassboard-tray";
pub(crate) struct TrayAnchor(pub(crate) Mutex<Option<tauri::Rect>>);

fn menu(app: &AppHandle, ready: bool) -> tauri::Result<Menu<tauri::Wry>> {
    let clear = MenuItem::with_id(app, "clear", "Clear screen", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Glassboard", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&clear, &quit])?;
    if ready {
        let restart = MenuItem::with_id(
            app,
            "install-update",
            "Restart to update",
            true,
            None::<&str>,
        )?;
        menu.prepend_items(&[&restart, &PredefinedMenuItem::separator(app)?])?;
    }
    Ok(menu)
}

pub(crate) fn show_update(app: &AppHandle, status: &UpdateStatus) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    let ready = matches!(status, UpdateStatus::Ready { .. });
    tray.set_menu(Some(menu(app, ready)?))?;
    tray.set_tooltip(Some(if ready {
        "Glassboard: update ready"
    } else {
        "Glassboard"
    }))
}

pub(crate) fn create_tray(app: &tauri::App) -> tauri::Result<()> {
    // macOS colors the template's alpha mask for the current menu bar appearance.
    #[cfg(target_os = "macos")]
    let icon = tauri::include_image!("icons/tray-template.png");
    #[cfg(not(target_os = "macos"))]
    let icon = tauri::include_image!("icons/tray.png");
    tauri::tray::TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Glassboard")
        .menu(&menu(app.handle(), false)?)
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
                if let Err(error) = transition(tray.app_handle(), Transition::Settings) {
                    report(tray.app_handle(), error);
                }
            }
        })
        .on_menu_event(|app, event| {
            // Menu item IDs are action wire names.
            let action = serde_json::from_value(event.id.as_ref().into());
            if let Err(e) = action
                .map_err(Into::into)
                .and_then(|action| perform(app, action))
            {
                report(app, e);
            }
        })
        .build(app)?;
    Ok(())
}
