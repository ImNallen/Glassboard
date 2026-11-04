use crate::{
    commands::REPOSITORY_URL,
    i18n::{t, Text},
};

/// Explains a fatal startup error in a native dialog and exits, since a
/// release build has no console and no Glassboard window may exist to show it.
pub(crate) fn exit(error: &Text) -> ! {
    log::error!("Could not start Glassboard: {error}");
    let title = t!("Glassboard could not start");
    eprintln!("{title}: {error}");
    let advice = t!(
        "Try opening Glassboard again. If this keeps happening, report it at {url} and include Glassboard's log file.",
        url = format!("{REPOSITORY_URL}/issues")
    );
    show_native(
        &title.render(),
        &format!("{}\n\n{}", error.render(), advice.render()),
    );
    std::process::exit(1);
}

#[cfg(target_os = "macos")]
fn show_native(title: &str, details: &str) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSAlert, NSAlertStyle, NSApplication};
    use objc2_foundation::NSString;
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let alert = NSAlert::new(mtm);
    alert.setAlertStyle(NSAlertStyle::Critical);
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(details));
    // A menu-bar app is not activated on launch, so the alert would open
    // behind the frontmost app. `activate` needs macOS 14; this supports 12.
    #[allow(deprecated)]
    NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
    alert.runModal();
}

#[cfg(target_os = "windows")]
fn show_native(title: &str, details: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND,
    };
    let wide = |text: &str| text.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let (text, caption) = (wide(details), wide(title));
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

#[cfg(target_os = "linux")]
fn show_native(title: &str, details: &str) {
    use gtk::prelude::*;
    if gtk::init().is_err() {
        return;
    }
    let dialog = gtk::MessageDialog::new(
        None::<&gtk::Window>,
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Ok,
        title,
    );
    dialog.set_title(title);
    dialog.set_secondary_text(Some(details));
    dialog.run();
    dialog.close();
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn show_native(_title: &str, _details: &str) {}
