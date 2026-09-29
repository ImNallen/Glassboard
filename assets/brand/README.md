# Glassboard logo

The marked pane: an open, rounded screen outline crossed by a drawing stroke.
The geometry is shared across all versions and remains recognizable in one color.

## Assets

- `mark.svg`: editable master; `currentColor` when used inline. Its default is black when loaded as an image.
- `mark-dark.svg`, `mark-white.svg`, `mark-mint.svg`: fixed-color versions for websites, documentation, and images.
- `app-icon.svg`, `.png`, `.icns`, `.ico`: mint tile, dark mark, transparent outer padding. PNG is 1024 × 1024.
- `github-avatar.svg`, `.png`: full square background with enough space for a circular crop. PNG is 1024 × 1024.
- `menu-bar-{16,22,32,44}.png`: black alpha masks. Use as macOS template images so the OS determines the displayed color. The 44 px export supports a 22 pt Retina presentation when the host supplies the correct logical size. On other platforms, choose a contrasting color version or the app icon.
- `favicon.svg`: transparent mark that follows light/dark appearance. PNG and ICO fallbacks use the mint tile.
- `preview.png`: review sheet; the wordmark uses a system sans serif and is a layout example.

Colors: mint `#A3E9D1`, dark green `#143D33`, and white `#FFFFFF`.
Use the dark mark on light backgrounds, mint or white on dark backgrounds.
Keep at least 3 units of clear space around the mark's 24-unit canvas.
Avoid shadows or transparency effects inside the small menu bar mark.

## Regeneration

Edit `mark.svg`, then run `python3 scripts/generate-brand-assets.py` from the repository root.
This optional asset-generation command requires `rsvg-convert` and Python's Pillow package;
neither is a runtime dependency. All other graphics in this folder are generated from the master.

The generator also updates `apps/desktop/src-tauri/icons/icon.{png,icns,ico}`, the macOS
template and Windows tray images, and `apps/desktop/public/favicon.{svg,ico}`. The macOS
template is rendered at 36 px for Tauri's 18 pt presentation on Retina displays.
The tutorial and settings use the SVG master directly as a CSS mask, inheriting
their surrounding text color in both appearances. The GitHub README uses the app
icon; the separate avatar export remains available for an account or organization.
