# Candidate A: extend the existing feature owners

## Problem

Ship desktop 0.1.1 with experimental Linux X11 behavior while the public website continues advertising published 0.1.0. The existing Session owns interaction, windows owns native presentation, capture owns screenshot and clipboard work, and commands owns autostart. Keep these owners. Linux defects are missing platform branches, clipboard lifetime, unavailable tray click events, stacking, and XCap's distinct DPI coordinates—not a missing cross-platform service layer. Grounding: `.audit/linux-x11-grounding.md`, `main.rs`, `commands.rs`, `capture/{mod,screen}.rs`, `windows/{mod,setup,z_order}.rs`, `tray.rs`, workflows and bundle config.

## Usage (caller's view)

The Linux user logs into an X11 desktop with a compositor and tray host, installs a Debian package or runs the AppImage, and uses the existing drawing/capture shortcuts. Settings is available from the tray menu. Copying closes the capture editor; another application can still paste the PNG while Glassboard runs. A Wayland login produces an explanation to switch to an X11 session, before plugins or overlay windows initialize.

Startup call site, before builder/plugin initialization:

```rust
#[cfg(target_os = "linux")]
if let Err(error) = linux_startup::prepare() {
    startup_error::exit(&error.to_string());
}
// Existing builder and setup follow.
```

Clipboard call sites preserve capture's existing command surface:

```rust
// setup: constructing this state does not connect to a clipboard.
app.manage(capture::CaptureClipboard::default());

// Existing capture worker, after decode/crop succeeds:
write_clipboard(&clipboard_app, id, image)?;
// Existing finish_copy closes only the matching capture.
```

Native control restoration keeps the caller's current single operation:

```rust
// Existing windows::focus_drawing and commands::activate_overlay:
raise_toolbar(app);
// main's overlay Focused(true) hook now covers Windows and Linux.
```

## Shape

### Typed startup boundary

Add one Linux-only private root module `linux_startup.rs`, with one callable function. It owns environment interpretation and X11 backend selection, not capture, tray, or window lifetimes.

```rust
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StartupError { WaylandSession, MissingX11Display }
impl std::fmt::Display for StartupError { /* actionable explanation */ }
impl std::error::Error for StartupError {}

pub(crate) fn prepare() -> Result<(), StartupError> { todo!() }
// Private pure policy; owned environment values are read once by prepare.
fn validate(
    session_type: Option<&std::ffi::OsStr>,
    display: Option<&std::ffi::OsStr>,
    wayland_display: Option<&std::ffi::OsStr>,
) -> Result<(), StartupError> { todo!() }
```

Reject `XDG_SESSION_TYPE=wayland` or nonempty `WAYLAND_DISPLAY`, even if `DISPLAY` exists (XWayland is not supported because XCap independently selects its backend). Otherwise require a nonempty DISPLAY, allowing Xvfb and X11 sessions that omit XDG_SESSION_TYPE. After acceptance set `GDK_BACKEND=x11`, before threads/GTK/plugins. Errors instruct the user to select an X11 login; they must not suggest a supported Wayland override. Environment policy is validated once here (boundary-discipline).

Extend `startup_error::show_native(details: &str)` with Linux GTK MessageDialog on the main thread. Initialize GTK using the unmodified environment on refusal so a Wayland GUI can display its refusal. Use the existing fatal title, blocking acknowledgement, and exit status 1. Always print details to stderr as well, since GTK cannot show a dialog without a working display. Add target Linux gtk 0.18 compatible with the locked Tauri stack. No shell dependence on zenity or external dialog programs.

### Clipboard lifetime in capture

```rust
#[derive(Default)]
pub(crate) struct CaptureClipboard(std::sync::Mutex<Option<arboard::Clipboard>>);
// Remains private to capture; API consumers cannot access the native handle.
fn write_clipboard(app: &tauri::AppHandle, id: u32, image: image::RgbaImage)
    -> crate::Result<()> { todo!() }
```

Manage one CaptureClipboard for the whole application on every OS. `write_clipboard` locks it, checks capture currency after acquiring the lock, initializes lazily if absent, then calls set_image. On initialization failure leave None so a later attempt can retry. Keep the object after success and after the capture closes. Errors follow the existing recoverable capture error path. Clipboard state never enters Session or wire serialization. Concurrent copy writers necessarily target one native selection; this small lock is the owner rather than several independently competing handles (model-the-domain). Never retain AppState's lock while calling native clipboard APIs. The sole new crate-visible capture item is an opaque lifetime owner; IPC signatures do not change.

### Platform branches stay beside the operation

- Widen Windows autostart dependency, plugin initialization, import, and command branches to `any(target_os = "windows", target_os = "linux")`; macOS LaunchAgent behavior remains owned by autostart.rs. OS autostart registration is the source of truth.
- In the existing tray menu builder insert Linux `settings` with label Settings, using the existing Action deserialization/Transition path. Keep it when the menu is rebuilt for updates. Enable ordinary left-click menu display on Linux. Do not synthesize TrayAnchor values; settings.rs already handles None.
- Compile `windows/z_order.rs` for Windows and Linux. Preserve `raise_controls(app: &AppHandle)` and its shared visible-surface loop/main-thread scheduling. Put native raising in a private helper selected by cfg: Windows keeps SetWindowPos; Linux obtains `window.gtk_window()?.window()` and invokes GDK raise without presenting/focusing. Expose no X11 handle types or native connection owner. Missing native GDK windows/errors report through the existing report mechanism. Add raising at the end of apply_windows to cover show order, as well as the current drawing activation and widened overlay focus hook. Restore only visible Toolbar, Tutorial, Settings, in that order; never show a hidden control. Repeating restoration preserves bounds, visibility, and keyboard focus. Existing always-on-top setup remains the policy.

### Linux capture coordinates

The newly identified XCap scale behavior requires a Linux branch, not pretending Linux is Windows. Use Tauri physical geometry as the authority for monitor choice and editor placement. In capture::start select the Tauri monitor containing the physical cursor with existing Rect containment, and take its physical position/size. Preserve current macOS/Windows selection logic.

Keep `screen::capture(app, point, position, size) -> Result<RgbaImage>` for this release; add a private Linux helper:

```rust
#[cfg(target_os = "linux")]
fn capture_x11(position: (i32, i32), size: (u32, u32))
    -> crate::Result<image::RgbaImage> { todo!() }
```

Inside the worker enumerate XCap monitors and match their XCap x/y/width/height multiplied by each monitor's scale_factor against requested Tauri physical bounds. Account explicitly for XCap's integer truncation: each converted edge must be within one scale unit; require exactly one matching monitor, otherwise return a recoverable display error. Capture that monitor directly, avoiding a second from_point interpretation. Editor placement and capture_monitor lookup use the same Tauri physical origin and extent. Capture pixels remain native pixels; existing frame scaling and crop code receives the actual image dimensions. No monitor handle crosses the thread boundary. Keep the existing short Linux compositor delay for this experimental release, but drive a real no-overlay-in-screenshot test. A future synchronization change belongs in screen.rs, not in Session.

### Packaging and publication

Use `npm run version:bump -- 0.1.1` to update existing desktop version owners. Add deb and appimage bundle targets (or a Linux Tauri config overriding targets if Tauri target filtering requires it), retaining existing Mac/Windows targets. Add Ubuntu 22.04 x86_64 to check/release workflows with GTK/WebKitGTK 4.1/AppIndicator plus locked XCap prerequisites (pkg-config, clang/libclang, XCB, XRandR, DBus, PipeWire, Wayland, EGL and OpenSSL build dependencies as needed). Checks compile the native app and run Rust tests/clippy, not just web tests. Release builds both deb and AppImage and the updater-manifest check requires a signed linux-x86_64 entry in addition to current targets. AppImage supplies automatic updater support; Debian users install an updated deb. No workflow is dispatched and no release is published during this task.

Extend `apps/web/src/lib/release.mjs`'s asset schema with optional Linux AppImage. Absence remains valid for 0.1.0; presence is validated just as strictly as existing filenames and renders an explicitly experimental X11 Linux link. Leave `apps/web/src/data/release.json` at 0.1.0 unchanged, without an invented asset URL. Developer/release documentation can describe experimental 0.1.1 support and installation without advertising an unpublished public download. Retain mandatory macOS/Windows assets.

Interface depth is high relative to this small extension: no new IPC, service trait, transport object, native-handle API, or feature flag. Each existing feature hides its own platform details. The two new named shapes are a typed startup refusal and opaque clipboard lifetime owner. Existing Surface, Session, physical geometry, and Result remain the vocabulary (minimize-reader-load).

## Synthesis decision

Candidate A only; parent synthesis has not selected a base. This proposal deliberately avoids a consolidated platform service.

## Tradeoffs accepted

- We accept localized cfg branches in exchange for keeping feature behavior beside its current owner.
- We accept rejecting XWayland in exchange for a truthful X11-only promise across GTK and XCap.
- We accept clipboard availability only while the application runs in exchange for correct X11 selection ownership without spawning a daemon.
- We accept a GTK raise implementation and existing compositor delay subject to native GUI evidence in exchange for avoiding a custom X11 connection layer.
- We accept conservative bounds matching with an explicit ambiguity error in exchange for retaining current signatures; if mixed-DPI testing exposes ambiguity, the capture-target representation must be redesigned rather than adding arbitrary matching preferences.

## Alternatives considered

- A Platform service owning startup, tray, clipboard and windows hides platform details but exposes a broad capability interface to unrelated existing owners. It requires migrating Mac/Windows behavior and introduces a second place to inspect every feature. Rejected for this preview scope.
- A Linux-specific event loop and X11 connection with clipboard/stacking commands hides native synchronization but adds request channels, shutdown rules and duplicate clipboard APIs. GTK/GDK and arboard already supply the relevant ownership and native work.
- A fresh clipboard per copy has a tiny interface but hides no lifetime policy and loses Linux selection ownership on return. Rejected as functionally wrong.
- Force GTK onto X11 while permitting a Wayland session: it makes startup look successful while XCap can independently take Wayland. Rejected rather than offering an unreliable opt-in flag.

## Implementation reconciliation

No implementation changes made. Parent provided the DPI finding after initial grounding; the capture coordinate section incorporates it. Signatures are a sketch, with native API availability to be confirmed by the actual Linux compile.

## Open questions and risks

Can GTK/GDK raise preserve visible controls above repeatedly focused overlays under the tested window manager without taking focus? Does XCap's scaled integer geometry normalize uniquely at Xft.dpi=192 and at fractional DPI? Does AppIndicator activation work in the supplied test tray host? These are implementation evidence gates, not permission requests; unresolved real-display and compositor differences belong in the experimental support statement.

## Success criteria

1. Native Linux build, tests and clippy pass alongside existing platform checks; desktop versions agree at 0.1.1 and public release metadata stays 0.1.0.
2. X11/Xvfb starts; Wayland with and without DISPLAY refuses before plugins/windows, shows a GUI explanation when GTK can connect, and exits nonzero. No display emits the reason on stderr and exits.
3. Native tray menu reaches Settings both before and after menu rebuild; autostart toggle is reflected by the OS registration.
4. Real pointer drawing, click-through, hotkeys, tutorial and Settings remain reachable; repeated overlay activation leaves controls above overlays without moving focus or bounds.
5. Native capture excludes overlays; region and full-image copy produce valid image/png readable by an independent process after editor closure, including a second capture/copy.
6. Xft.dpi=192 capture chooses and covers the correct monitor, preserves image pixels and positions editor/toolbar correctly. Fractional/negative-origin coordinate tests cover normalization and ambiguity refusal.
7. deb/AppImage builds are configured; updater manifest validation includes linux-x86_64. Optional Linux website metadata tests cover absent/valid/invalid assets without changing today's advertised downloads.

Parent's repeatable Docker GUI scripts supply the native evidence; no new testing framework is proposed. Physical displays, real GPU stacks and untested desktops remain explicitly unverified.

## Next implementation step

Implement the Linux startup boundary and dependency branches, then obtain the first real Linux compile before adding clipboard and presentation behavior.

## Accepted synthesis

The parent and independent gpt-6-luna judge chose candidate A. The feature-owned extension avoids migrating established macOS and Windows code for a Linux preview. Graft candidate B's explicit capture coordinate resolver boundary, kept private within capture, and packaged AppImage smoke verification. Retain A's direct matching of XCap monitor physical bounds instead of calling from_point again on Linux. Reject the broad platform service migration because it changes existing native owners without requiring that change for X11 support. Native GTK compilation and GUI tests resolve API and compositor questions during implementation. The parent owns scripts/linux and the Docker runtime; the implementation owner works in its detached checkout.

## Accepted implementation reconciliation

The implementation owner found that XCap truncates origin and extent independently. Parent accepted matching physical x, y, width, and height independently within one scale unit, rather than matching summed right and bottom edges within one scale unit. Unique-match refusal stays mandatory. The saved Linux capture contract now uses independent fields. Negative and fractional geometry tests must exercise the truncation tolerance.

## Native runtime reconciliation

Native execution required three changes to the initial GTK sketch. Hidden overlays need realization before Tao applies their input shape. Nonresizable GTK windows need a requested size and native resize together, owned by `windows::set_size` for all existing callers. Plain GDK raise and sibling restack do not cross Openbox's fullscreen stacking layer. Visible controls now form transient relationships with their nearest lower visible Glassboard window, then restack without activation.

The complete Docker command passes all nine native groups at GTK scale 1 and 2. The rebuilt AppImage and installed Debian debug package pass the same checks. Physical hardware, other desktops, and final x86_64 release execution remain separate coverage limits.
