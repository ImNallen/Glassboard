<img src="assets/brand/app-icon.png" alt="Glassboard logo" width="96" height="96" />

# Glassboard

**Draw on your screen, then get back to work.**

Glassboard is a small annotation tool for macOS and Windows. Press a shortcut to draw
arrows, shapes, and text over anything on screen. Press it again and the drawings are
cleared. You can also capture part of the screen, mark it up, and copy it to the
clipboard, ready to paste into a chat or a coding agent.

Everything runs on your machine. There's no account, no server, no video recording,
and nothing is sent over the network.

[glassboard.dev](https://glassboard.dev) has a live demo you can draw on in the browser.

## Download and install

Download Glassboard for **macOS** or **Windows** at [glassboard.dev](https://glassboard.dev).
The macOS download supports Apple silicon and Intel. The Windows installer is for
64-bit Intel and AMD PCs.

- **macOS:** open the DMG, drag Glassboard into Applications, then open it from Applications.
- **Windows:** run the setup EXE, finish installation, then launch Glassboard.

The macOS app is signed and notarized. The Windows installer is currently unsigned,
so Windows may show an **Unknown publisher** or SmartScreen warning. Download only
from the Glassboard release linked on the site. Follow your organization's policy
if installation is blocked.

On first launch, the tutorial walks you through drawing and switching back to work.
Press **Cmd+Shift+A** on macOS or **Ctrl+Shift+A** on Windows to start drawing.
While drawing, use **Cmd+S** or **Ctrl+S** to capture a region, then **Cmd+C** or
**Ctrl+C** to copy it.

See the [installation guide](https://glassboard.dev/install/) for screenshot
permissions and first-use instructions, or browse the
[GitHub releases](https://github.com/ImNallen/Glassboard/releases).

## Features

- **Draw over anything:** arrow, pen, square, circle, text, highlighter, and eraser, with undo and redo.
- **Starts clean every time:** leaving annotation mode clears every display. You can also
  set drawings to fade out after 3, 5, or 10 seconds.
- **Screenshot and annotate:** freeze the screen, select a region, draw on it, and copy
  the result. Nothing is saved to disk.
- **Out of the way:** the toolbar shrinks to a small pill on the left, right, or bottom
  edge of the screen and follows your system's light or dark mode.
- **Multiple displays:** each display gets its own drawing surface.
- **Keyboard-friendly:** every tool and color has a shortcut.

## Shortcuts

On Windows, use **Ctrl** wherever **Cmd** appears.

| Action | Shortcut |
| --- | --- |
| Turn annotation mode on or off (global, configurable) | Cmd+Shift+A |
| Capture a screenshot region | Cmd+S |
| Copy the capture and close | Cmd+C |
| Arrow / Pen / Square / Circle | Cmd+1 / 2 / 3 / 4 |
| Eraser / Text / Highlighter | Cmd+E / T / H |
| Choose a color | 1–8 |
| Undo / Redo | Cmd+Z / Cmd+Shift+Z |
| Hide the overlay or cancel a capture | Esc |

Hold **Shift** while dragging to snap arrows to 45° or draw perfect squares and circles.
To open settings, click the menu bar or system tray icon.

## Build from source

For development or a local build, you'll need:

- Node.js 22.12 or newer
- Rust (stable)
- The [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS

Run it in development mode:

```sh
npm ci
npm run desktop tauri dev
```

Or build the app and its installer:

```sh
npm run desktop tauri build
```

On macOS this creates `Glassboard.app` and a `.dmg`; on Windows, a setup `.exe`. You'll
find them in `apps/desktop/src-tauri/target/release/bundle/`.

> **macOS screenshot permission:** screen capture needs the **Screen Recording**
> permission, which Glassboard requests the first time you take a screenshot. If you
> rebuild the app and it stops working, remove Glassboard from the list in System
> Settings, add the new `.app` again, and restart it.

## Known limitations

- Glassboard only sees displays that were connected at launch. Restart it after
  connecting, rearranging, or rescaling displays.
- Drawings stay at a fixed spot on screen. They don't move with windows or scroll with content.
- To show your drawings in a video call, share your whole screen. Sharing a single window may leave them out.
- Windows support, fullscreen apps, Stage Manager, and mixed-DPI setups still need more testing.
- The macOS build uses private APIs for transparency, so it can't be distributed through the Mac App Store.

## Troubleshooting

Glassboard keeps a log of errors, which is helpful to attach when you
[report an issue](https://github.com/ImNallen/Glassboard/issues):

- macOS: `~/Library/Logs/dev.glassboard.desktop/`
- Windows: `%LOCALAPPDATA%\dev.glassboard.desktop\logs\`

If your settings can't be read, Glassboard keeps the ones it can, resets the rest, and
saves the original file as `preferences.json.bak` next to `preferences.json`.

## Development

The repository is an npm workspace:

| Path | Contents |
| --- | --- |
| `apps/desktop` | The desktop app (Tauri 2, Rust, Svelte 5) |
| `apps/web` | The [glassboard.dev](https://glassboard.dev) landing page (Astro) |
| `packages/ui` | The drawing engine, toolbar, overlay, and capture editor, shared by both apps |
| `assets/brand` | Logo and icons ([guidelines](assets/brand/README.md)) |

Common commands:

```sh
npm run desktop dev   # browser preview of the UI at http://127.0.0.1:1420
npm run web dev       # landing page
npm run check         # type checks
npm test              # unit tests
npm run build         # build everything

cargo test   --manifest-path apps/desktop/src-tauri/Cargo.toml
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml -- -D warnings
```

The browser preview is good for UI work, but the overlay, global shortcut, and native
screenshots only work in the Tauri app. Check native changes on both macOS and Windows.

For the release and website deployment steps, see [Publish a release](docs/releases.md).

## License

Glassboard is free and open source under the [MIT License](LICENSE). Use it, fork it,
and change it however you like.
