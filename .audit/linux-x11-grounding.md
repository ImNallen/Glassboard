# Linux X11 grounding

The application owns one transparent always-on-top overlay per monitor and separate toolbar, settings, tutorial, and capture windows. Rust Session in session.rs owns drawing/capture transitions. windows/setup.rs constructs windows. windows/mod.rs applies focus and click-through and selects the cursor monitor. windows/toolbar.rs polls the desktop pointer to reveal controls. Capture identifies xcap monitors from global coordinates, hides overlays before capture, and copies through capture/mod.rs.

Current Linux blockers are empty cfg-only autostart bodies in commands.rs, Windows-only autostart dependency/initialization, TITLE unused because Linux startup_error display is a no-op, and app/dmg/nsis-only bundle targets. Linux should reuse the desktop autostart plugin for Windows and Linux. macOS keeps its LaunchAgent implementation.

Locked arboard 3.6.1 README Clipboard Ownership requires the clipboard object to live while Linux clients paste. Current capture/mod.rs write_clipboard constructs and drops a temporary. Hold one lazily created clipboard for the app lifetime. Initialization failure must remain a recoverable capture error. Verify an independent process reads image/png after the capture window closes.

Locked tray-icon 0.25.1 src/lib.rs documents no Linux TrayIconEvent. Settings currently relies on left click. Add Settings to the Linux tray menu using the existing settings action. windows/settings.rs already falls back to the primary monitor when there is no tray rect.

Linux uses X11 only for this preview. Decide the startup boundary before installing plugins or opening windows. Native Wayland lacks the application's global pointer, window placement, topmost behavior, and global-hotkey backend. Explicitly reject unsupported native Wayland rather than silently presenting incomplete functionality. Consider how a launched GUI user receives the reason.

Cargo.lock owns exact native dependency versions. xcap 0.9.8 pulls PipeWire, libwayshot, and XCB on Linux even for an X11 application. Its documented build prerequisites include pkg-config, libclang-dev, libxcb1-dev, libxrandr-dev, libdbus-1-dev, libpipewire-0.3-dev, libwayland-dev, and libegl-dev. Tauri also needs WebKitGTK 4.1 and AppIndicator.

Root scripts/bump-version.mjs owns desktop npm, npm lock, Cargo manifest and Cargo lock version alignment. Use npm run version:bump -- 0.1.1. apps/web/src/data/release.json owns published website version independently. Per docs/releases.md keep advertised 0.1.0 and current download links until 0.1.1 installers are public. release.mjs currently requires exactly macOS and Windows assets; make Linux download optional so older public releases remain valid. Add a truthful experimental X11 label and installation instructions without advertising an unpublished installer.

Check workflow runs native desktop macOS/Windows. Add Linux build/test and dependencies. Release workflow builds only Mac/Windows; add Linux packages and verify linux-x86_64 in latest.json. Updater currently supports all desktop targets through the Tauri updater plugin; Linux automatic installation is AppImage-based. Keep Debian installation guidance distinct from AppImage updater behavior. No release publication, secrets, commits, branches, pushes, or PR changes are authorized.

Parent is preparing Debian Bookworm ARM64 Docker with Xvfb, Openbox, xcompmgr, GTK, XCap, Rust 1.93, Node 24. Tests must drive the native binary, not a browser preview or mocked IPC. Real PC graphics, physical multiple displays, and Wayland remain unverified.
