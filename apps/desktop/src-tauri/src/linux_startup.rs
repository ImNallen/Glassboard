use std::ffi::OsStr;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StartupError {
    WaylandSession,
    MissingX11Display,
}

impl std::fmt::Display for StartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::WaylandSession => "Glassboard's experimental Linux support requires an X11 desktop session. Sign out and select an X11 or Xorg session at the login screen, then open Glassboard again. Wayland and XWayland sessions are not supported.",
            Self::MissingX11Display => "Glassboard could not find an X11 display. Open Glassboard from an X11 desktop session with DISPLAY set.",
        })
    }
}

impl std::error::Error for StartupError {}

pub(crate) fn prepare() -> Result<(), StartupError> {
    validate(
        std::env::var_os("XDG_SESSION_TYPE").as_deref(),
        std::env::var_os("DISPLAY").as_deref(),
        std::env::var_os("WAYLAND_DISPLAY").as_deref(),
    )?;
    // GTK and XCap must use the same desktop backend. This runs before either initializes.
    std::env::set_var("GDK_BACKEND", "x11");
    Ok(())
}

fn validate(
    session_type: Option<&OsStr>,
    display: Option<&OsStr>,
    wayland_display: Option<&OsStr>,
) -> Result<(), StartupError> {
    if session_type
        .is_some_and(|session| session.as_encoded_bytes().eq_ignore_ascii_case(b"wayland"))
        || wayland_display.is_some_and(|display| !display.is_empty())
    {
        return Err(StartupError::WaylandSession);
    }
    if display.is_none_or(OsStr::is_empty) {
        return Err(StartupError::MissingX11Display);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_x11_and_virtual_displays_without_a_session_type() {
        for session in [None, Some(OsStr::new("x11")), Some(OsStr::new("tty"))] {
            assert_eq!(validate(session, Some(OsStr::new(":99")), None), Ok(()));
            assert_eq!(
                validate(session, Some(OsStr::new(":0")), Some(OsStr::new(""))),
                Ok(())
            );
        }
    }

    #[test]
    fn refuses_wayland_even_with_an_xwayland_display() {
        for display in [None, Some(OsStr::new(":0"))] {
            assert_eq!(
                validate(Some(OsStr::new("wayland")), display, None),
                Err(StartupError::WaylandSession)
            );
            assert_eq!(
                validate(
                    Some(OsStr::new("x11")),
                    display,
                    Some(OsStr::new("wayland-0"))
                ),
                Err(StartupError::WaylandSession)
            );
        }
    }

    #[test]
    fn refuses_a_missing_or_empty_display() {
        for display in [None, Some(OsStr::new(""))] {
            assert_eq!(
                validate(None, display, None),
                Err(StartupError::MissingX11Display)
            );
        }
    }
}
