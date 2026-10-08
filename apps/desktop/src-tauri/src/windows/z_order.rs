use super::Surface;
use crate::state::report;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
};

/// Restore the controls above the full-display overlays without moving keyboard focus.
pub(super) fn raise_controls(app: &tauri::AppHandle) {
    let handle = app.clone();
    // Keep native ordering on the window thread, after any queued focus/show requests.
    if let Err(error) = app.run_on_main_thread(move || {
        for surface in [Surface::Toolbar, Surface::Tutorial, Surface::Settings] {
            let Some(window) = surface.window(&handle) else {
                continue;
            };
            let result = (|| -> crate::Result<()> {
                if !window.is_visible()? {
                    return Ok(());
                }
                raise_window(&window)
            })();
            if let Err(error) = result {
                report(
                    &handle,
                    format!("Could not raise {}: {error}", surface.label()),
                );
            }
        }
    }) {
        report(
            app,
            format!("Could not restore control window order: {error}"),
        );
    }
}

#[cfg(target_os = "linux")]
fn raise_window(window: &tauri::WebviewWindow) -> crate::Result<()> {
    use gtk::prelude::*;
    use tauri::Manager;
    let app = window.app_handle();
    let native_window = window.gtk_window()?;
    let native = native_window
        .window()
        .ok_or("The control window is unavailable")?;
    let below = [
        Surface::Settings,
        Surface::Tutorial,
        Surface::Toolbar,
        crate::state::snapshot(app).active_overlay,
    ];
    for surface in below
        .into_iter()
        .skip_while(|surface| surface.label() != window.label())
        .skip(1)
    {
        let Some(sibling) = surface.window(app) else {
            continue;
        };
        if sibling.is_visible()? {
            let sibling_window = sibling.gtk_window()?;
            let sibling = sibling_window
                .window()
                .ok_or("The sibling window is unavailable")?;
            // Transients inherit the focused full-screen overlay's WM stacking layer.
            native_window.set_transient_for(Some(&sibling_window));
            // An explicit sibling asks the WM to restack reparented client windows.
            native.restack(Some(&sibling), true);
            return Ok(());
        }
    }
    native_window.set_transient_for(gtk::Window::NONE);
    native.raise();
    Ok(())
}

#[cfg(target_os = "windows")]
fn raise_window(window: &tauri::WebviewWindow) -> crate::Result<()> {
    let hwnd = window.hwnd()?;
    // SAFETY: Tauri owns this live HWND and this runs on its window thread.
    Ok(unsafe { raise_without_activation(hwnd.0) }?)
}

/// `hwnd` must be live and belong to the calling thread.
#[cfg(target_os = "windows")]
unsafe fn raise_without_activation(hwnd: HWND) -> std::io::Result<()> {
    // Repeating set_always_on_top(true) is a no-op in Tao when the flag is
    // already set. SetWindowPos must run each time an overlay is raised.
    let raised = unsafe {
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    };
    if raised == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use std::ptr::null_mut;
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, GetForegroundWindow, GetWindow, GetWindowRect,
            IsWindowVisible, GW_HWNDPREV, WS_EX_TOPMOST, WS_POPUP,
        },
    };

    struct TestWindow(HWND);
    impl TestWindow {
        fn new() -> Self {
            // Hidden native windows exercise Z order without interrupting the desktop.
            let hwnd = unsafe {
                CreateWindowExW(
                    WS_EX_TOPMOST,
                    windows_sys::core::w!("STATIC"),
                    windows_sys::core::w!("Glassboard Z-order test"),
                    WS_POPUP,
                    10,
                    20,
                    100,
                    80,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                )
            };
            assert!(!hwnd.is_null(), "{}", std::io::Error::last_os_error());
            Self(hwnd)
        }

        fn bounds(&self) -> [i32; 4] {
            let mut rect = RECT::default();
            assert_ne!(unsafe { GetWindowRect(self.0, &mut rect) }, 0);
            [rect.left, rect.top, rect.right, rect.bottom]
        }
    }
    impl Drop for TestWindow {
        fn drop(&mut self) {
            unsafe { DestroyWindow(self.0) };
        }
    }

    fn above(top: HWND, bottom: HWND) -> bool {
        let mut previous = unsafe { GetWindow(bottom, GW_HWNDPREV) };
        while !previous.is_null() {
            if previous == top {
                return true;
            }
            previous = unsafe { GetWindow(previous, GW_HWNDPREV) };
        }
        false
    }

    #[test]
    fn repeatedly_raises_an_already_topmost_control_without_activation_or_layout_changes() {
        let toolbar = TestWindow::new();
        let overlay = TestWindow::new();
        let bounds = toolbar.bounds();
        let active = unsafe { GetForegroundWindow() };
        for _ in 0..3 {
            unsafe { raise_without_activation(overlay.0) }.unwrap();
            assert!(above(overlay.0, toolbar.0));
            unsafe { raise_without_activation(toolbar.0) }.unwrap();
            assert!(above(toolbar.0, overlay.0));
            assert_eq!(unsafe { GetForegroundWindow() }, active);
            assert_eq!(toolbar.bounds(), bounds);
            assert_eq!(unsafe { IsWindowVisible(toolbar.0) }, 0);
        }
    }
}
