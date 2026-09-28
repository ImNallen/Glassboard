# Glassboard

A small desktop annotation tool for macOS and Windows. Draw over your screen, leave annotations visible while interacting with other apps, and hide everything with a shortcut.

Built with Rust, Tauri 2, Svelte 5, TypeScript, and Canvas 2D. Everything runs locally. There is no account, server, screen recording, or network service in the built app.

## Run locally

Install Node.js 22.12+ (or a newer supported LTS), Rust stable, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS. macOS needs the Xcode Command Line Tools. Windows needs the C++ Build Tools and WebView2.

```sh
npm ci
npm run tauri dev
```

The app starts in drawing mode with a floating toolbar. The toolbar and settings follow the system light/dark appearance and update when it changes. The toolbar is fixed: choose **Left**, **Right**, or **Bottom** under **Toolbar position** in settings. It stays centered along the selected edge of the active display’s usable area. Side toolbars are vertical; the bottom toolbar is horizontal. The selection saves automatically. Click the menu-bar/system-tray icon to open settings and controls directly. Right-click it for **Clear screen** and **Quit Glassboard**.

For a browser-only UI preview, run `npm run dev` and open http://127.0.0.1:1420. The preview draws within that tab; desktop overlay behavior requires Tauri. Shortcut preferences in the preview are illustrative; its keyboard bindings stay at their defaults.

## Controls

| Action | macOS | Windows |
| --- | --- | --- |
| Show / hide annotations | Cmd+Shift+A | Ctrl+Shift+A |
| Draw / interact with other apps | Cmd+Shift+I | Ctrl+Shift+I |
| Show / hide just the toolbar | Cmd+Shift+H | Ctrl+Shift+H |
| Hide while drawing | Escape | Escape |
| Undo / redo | Cmd+Z / Cmd+Shift+Z | Ctrl+Z / Ctrl+Shift+Z |

Use **⌘1** for Arrow, **⌘2** for Square, **⌘3** for Circle, and **⌘4** for Highlight (**Ctrl+1–4** on Windows), and **V** to switch between drawing and interaction. Tool shortcuts and undo/redo work when a Glassboard window has keyboard focus; the show/hide, interaction, and toolbar shortcuts work globally. Interaction and toolbar shortcuts do nothing while the overlay is hidden. In drawing mode, hover over a shape’s stroke or edge and press **X** to erase it. At overlaps, the topmost shape is removed. Each press removes one shape; holding X does not repeat. Erasing supports undo/redo and retains the original auto-fade deadline.

Hold **Shift** while dragging to constrain arrows to 45-degree increments or draw squares and circles. Colors sit directly on the toolbar in this order: **Rainbow**, **Shifting**, **Black**, **White**, **Green**, **Yellow**, **Red**, and **Blue**. Rainbow is the default for new preferences; an explicitly saved mode is remembered. Rainbow gives every tool a gradient blending coral red, warm yellow, mint green, aqua, sky blue, violet, and pink, with a random starting color for each new shape. Its hues shift with your drag and freeze on release. Shifting steps through the same seven colors with each completed shape, across displays. Cancelled strokes, undo, redo, and clear do not advance the cycle. The line-width button cycles through Thin, Regular, and Bold with each click; its line preview shows the current width. The auto-fade button cycles **∞ → 3s → 5s → 10s → ∞** and remembers your selection. New shapes use the duration selected when drawing starts; their timer begins on release, with a soft fade during the final half-second. ∞ keeps drawings until cleared. Changing the duration leaves existing drawings unchanged, and expired drawings stay removed through undo/redo. Click the menu bar / system tray icon to open a separate settings window beside the icon and change the global show/hide shortcut. The window opens below a top menu bar or above a bottom taskbar, stays within the display, and dismisses on Escape or when it loses focus. The settings window also contains annotation visibility, drawing mode, toolbar visibility and position, clear-all, and quit controls. It must include Control, Command/Super, or Alt. Conflicting registrations show an error without replacing the working shortcut.

Hiding preserves drawings. Clear is undoable. Undo/redo/clear act on the display you most recently drew on (or the display under the cursor when summoned). **Clear all drawings** is in settings; **Clear screen** in the right-click menu clears the active annotation display. The toolbar remains usable in interaction mode. Summoning the overlay positions it on the display under the cursor.

## Local builds

No store, installer, signing service, or distribution setup is required for personal development.

```sh
# macOS: a standalone local .app
npm run tauri build -- --bundles app
# Open src-tauri/target/release/bundle/macos/Glassboard.app

# Windows: a standalone executable, using the installed WebView2 runtime
npm run tauri build -- --no-bundle
# Run src-tauri/target/release/glassboard.exe
```

Use `--debug` for a faster development build. The app uses Tauri's macOS private-API transparency feature; this configuration is not intended for the Mac App Store.

## Checks

```sh
npm run check
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

The tests cover undo/redo branches, undoable clear, immutable stroke history, constrained geometry, mode transitions, and shortcut validation. Native behavior must also be checked on each OS.

## Current boundaries

- Annotation windows are created for displays connected at launch. Restart after connecting, disconnecting, rearranging, or changing the scaling of displays. Strokes use logical coordinates; canvases render at the display's pixel density.
- Drawings are attached to screen positions; they do not follow a window when it moves or content when it scrolls.
- Drawings remain in memory until quitting; tool, color, color mode, width, and shortcut preferences are saved to `preferences.json` in Tauri's app configuration directory.
- Whole-display sharing is the intended way to include annotations. Sharing only another application's window may omit them. The toolbar may be captured; hide it independently when needed.
- macOS window flags support all Spaces and fullscreen auxiliary windows. Fullscreen applications, Stage Manager, mixed-DPI displays, and receiving-end Zoom/Teams/Meet captures still need hands-on compatibility testing. Windows runtime behavior is not yet verified.
- Protected system surfaces and exclusive fullscreen applications are outside this first version's scope.

## Structure

- `src-tauri/src/main.rs`: native windows, menu-bar/tray controls, global shortcuts, preferences, and authoritative interaction state.
- `src/Toolbar.svelte`: floating drawing controls.
- `src/Settings.svelte`: separate settings window.
- `src/Overlay.svelte`: pointer capture, stroke lifecycle, and demand-driven canvas rendering.
- `src/lib/drawing.ts`: framework-independent shape rendering and per-display history.
- `src/lib/session.ts`: typed native bridge and browser preview adapter.

Pointer movements and rendering stay in the webview. Only settings, mode changes, and discrete drawing commands cross the Rust bridge. Each display owns its annotation history, while the separate toolbar can stay interactive when overlay windows pass clicks through.
