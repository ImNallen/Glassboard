# Glassboard

A small desktop annotation tool for macOS and Windows. Draw over your screen, leave annotations visible while interacting with other apps, and hide everything with a shortcut.

Built with Rust, Tauri 2, Svelte 5, TypeScript, and Canvas 2D. Everything runs locally. There is no account, server, screen recording, or network service in the built app.

## Run locally

Install Node.js 22.12+ (or a newer supported LTS), Rust stable, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS. macOS needs the Xcode Command Line Tools. Windows needs the C++ Build Tools and WebView2.

```sh
npm ci
npm run tauri dev
```

The app starts in drawing mode with the toolbar tucked away as a thin pill at the edge of the display. Move the cursor near the pill to open the toolbar; it collapses again shortly after the cursor leaves. It stays open while an error is showing or a control has keyboard focus. The toolbar and settings follow the system light/dark appearance and update when it changes. The toolbar is docked: choose **Left**, **Right**, or **Bottom** under **Toolbar position** in settings. It stays centered along the selected edge of the active display’s usable area. Side toolbars are vertical; the bottom toolbar is horizontal. The selection saves automatically. Click the menu-bar/system-tray icon to open settings. Right-click it for **Clear screen** and **Quit Glassboard**.

For a browser-only UI preview, run `npm run dev` and open http://127.0.0.1:1420. The preview draws within that tab; desktop overlay behavior requires Tauri. Shortcut preferences in the preview are illustrative; its show/hide binding stays at the default.

## Controls

| Action | macOS | Windows |
| --- | --- | --- |
| Show / hide annotations (global, configurable) | Cmd+Shift+A | Ctrl+Shift+A |
| Draw / interact with other apps | V | V |
| Hide while drawing | Escape | Escape |
| Undo / redo | Cmd+Z / Cmd+Shift+Z | Ctrl+Z / Ctrl+Shift+Z |

Tools, left to right: **Pen**, **Arrow**, **Square**, **Circle**, **Highlight**, **Text**, and **Eraser**, on **⌘1–7** (**Ctrl+1–7** on Windows). Press **V** to switch between drawing and interaction. Pen draws a freehand stroke in the current color and width. Text places an inline editor where you click: type, press **Enter** to commit, **Shift+Enter** for a new line, or **Escape** to discard; clicking elsewhere or switching tools commits. Text size follows the line-width setting. The eraser removes any shape you click or drag across, as a single undo step. The show/hide shortcut is the only global one and the only way to summon annotation mode; tool shortcuts, **V**, and undo/redo work when a Glassboard window has keyboard focus. The toolbar's Interact button also switches modes. The aim is for the toolbar to be optional: every control on it should eventually have a shortcut. With any tool, hover over a shape’s stroke or edge and press **X** to erase it. At overlaps, the topmost shape is removed. Each press removes one shape; holding X does not repeat. Erasing supports undo/redo and retains the original auto-fade deadline.

Hold **Shift** while dragging to constrain arrows to 45-degree increments or draw squares and circles. Text uses the same color, color mode, and auto-fade as other shapes. Colors sit directly on the toolbar in this order: **Rainbow**, **Shifting**, **Black**, **White**, **Green**, **Yellow**, **Red**, and **Blue**. Rainbow is the default for new preferences; an explicitly saved mode is remembered. Rainbow gives every tool a gradient blending coral red, warm yellow, mint green, aqua, sky blue, violet, and pink, with a random starting color for each new shape. Its hues shift with your drag and freeze on release. Shifting steps through the same seven colors with each completed shape, across displays. Cancelled strokes, undo, redo, and clear do not advance the cycle. The line-width button cycles through Thin, Regular, and Bold with each click; its line preview shows the current width. The auto-fade button cycles **∞ → 3s → 5s → 10s → ∞** and remembers your selection. New shapes use the duration selected when drawing starts; their timer begins on release, with a soft fade during the final half-second. ∞ keeps drawings until cleared. Changing the duration leaves existing drawings unchanged, and expired drawings stay removed through undo/redo. Click the menu bar / system tray icon to open a separate settings window beside the icon. The window opens below a top menu bar or above a bottom taskbar, stays within the display, and dismisses on Escape or when it loses focus. It contains the show/hide shortcut, toolbar position, clear-all, and quit controls. To change the global show/hide shortcut, click the shortcut field and press the new combination; it saves immediately. Escape cancels recording, and the reset button restores the default. The shortcut must include Command/Control, Control, Super, or Alt. Conflicting registrations show an error without replacing the working shortcut.

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
- Whole-display sharing is the intended way to include annotations. Sharing only another application's window may omit them. The toolbar and its collapsed pill may be captured.
- macOS window flags support all Spaces and fullscreen auxiliary windows. Fullscreen applications, Stage Manager, mixed-DPI displays, and receiving-end Zoom/Teams/Meet captures still need hands-on compatibility testing. Windows runtime behavior is not yet verified.
- Protected system surfaces and exclusive fullscreen applications are outside this first version's scope.

## Structure

- `src-tauri/src/main.rs`: native windows, menu-bar/tray controls, the global shortcut, preferences, cursor proximity for the toolbar, and authoritative interaction state.
- `src/Toolbar.svelte`: floating drawing controls that collapse to a pill when the cursor is away.
- `src/Settings.svelte`: separate settings window.
- `src/Overlay.svelte`: pointer capture, stroke lifecycle, and demand-driven canvas rendering.
- `src/lib/drawing.ts`: framework-independent shape rendering and per-display history.
- `src/lib/session.ts`: typed native bridge and browser preview adapter.

Pointer movements and rendering stay in the webview. Only settings, mode changes, and discrete drawing commands cross the Rust bridge. Each display owns its annotation history, while the separate toolbar can stay interactive when overlay windows pass clicks through.
