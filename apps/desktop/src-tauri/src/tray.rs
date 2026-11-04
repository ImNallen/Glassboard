use crate::{
    commands::{perform, transition},
    i18n::t,
    session::Transition,
    state::{report, snapshot},
    updates::UpdateStatus,
};
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    AppHandle, Manager,
};

pub(crate) const TRAY_ID: &str = "glassboard-tray";
pub(crate) struct TrayAnchor(pub(crate) Mutex<Option<tauri::Rect>>);

/// How close together a press on the icon and Settings losing focus count as one
/// gesture. The OS delivers them separately and in either order.
const SAME_GESTURE: Duration = Duration::from_millis(500);

/// Tells a click that closes Settings from one that opens it. Pressing the icon
/// takes focus from an open Settings, which closes it before the click finishes.
#[derive(Default)]
pub(crate) struct TrayToggle(Mutex<ToggleState>);
#[derive(Default)]
struct ToggleState {
    settings_blurred: Option<Instant>,
    closing: bool,
}
impl TrayToggle {
    pub(crate) fn settings_blurred(&self, now: Instant) {
        self.0.lock().unwrap().settings_blurred = Some(now);
    }
    fn pressed(&self, settings_open: bool, now: Instant) {
        let mut state = self.0.lock().unwrap();
        state.closing = settings_open
            || state
                .settings_blurred
                .is_some_and(|blurred| now.duration_since(blurred) < SAME_GESTURE);
    }
    fn released(&self) -> Transition {
        if std::mem::take(&mut self.0.lock().unwrap().closing) {
            Transition::CloseSettings
        } else {
            Transition::Settings
        }
    }
}

fn menu(app: &AppHandle, ready: bool) -> tauri::Result<Menu<tauri::Wry>> {
    let quit = MenuItem::with_id(
        app,
        "quit",
        t!("Quit Glassboard").render(),
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&quit])?;
    #[cfg(target_os = "linux")]
    menu.prepend(&MenuItem::with_id(
        app,
        "settings",
        t!("Settings").render(),
        true,
        None::<&str>,
    )?)?;
    if ready {
        let restart = MenuItem::with_id(
            app,
            "install-update",
            t!("Restart to update").render(),
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
        t!("Glassboard: update ready").render()
    } else {
        "Glassboard".into()
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
        .show_menu_on_left_click(cfg!(target_os = "linux"))
        .on_tray_icon_event(|tray, event| {
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
            let app = tray.app_handle();
            let left_button = match &event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state,
                    ..
                } => Some(*button_state),
                _ => None,
            };
            let rect = match event {
                TrayIconEvent::Click { rect, .. }
                | TrayIconEvent::DoubleClick { rect, .. }
                | TrayIconEvent::Enter { rect, .. }
                | TrayIconEvent::Move { rect, .. }
                | TrayIconEvent::Leave { rect, .. } => rect,
                _ => return,
            };
            *app.state::<TrayAnchor>().0.lock().unwrap() = Some(rect);
            let toggle = app.state::<TrayToggle>();
            match left_button {
                Some(MouseButtonState::Down) => {
                    toggle.pressed(snapshot(app).settings_open, Instant::now());
                }
                Some(MouseButtonState::Up) => {
                    if let Err(error) = transition(app, toggle.released()) {
                        report(app, error);
                    }
                }
                None => {}
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

#[cfg(test)]
mod tests {
    use super::*;

    fn click(toggle: &TrayToggle, settings_open: bool, at: Instant) -> Transition {
        toggle.pressed(settings_open, at);
        toggle.released()
    }

    #[test]
    fn a_click_opens_closed_settings_and_closes_open_settings() {
        let now = Instant::now();
        let toggle = TrayToggle::default();
        assert_eq!(click(&toggle, false, now), Transition::Settings);
        assert_eq!(click(&toggle, true, now), Transition::CloseSettings);
    }

    #[test]
    fn focus_lost_just_before_the_press_still_closes_settings() {
        let now = Instant::now();
        let toggle = TrayToggle::default();
        toggle.settings_blurred(now);
        assert_eq!(
            click(&toggle, false, now + SAME_GESTURE / 2),
            Transition::CloseSettings
        );
    }

    #[test]
    fn a_later_click_reopens_settings_after_focus_moved_away() {
        let now = Instant::now();
        let toggle = TrayToggle::default();
        toggle.settings_blurred(now);
        assert_eq!(
            click(&toggle, false, now + SAME_GESTURE),
            Transition::Settings
        );
    }

    #[test]
    fn a_release_without_a_press_opens_settings() {
        assert_eq!(TrayToggle::default().released(), Transition::Settings);
    }
}
