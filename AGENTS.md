# Working in this repository

## Layout

npm workspaces: `apps/desktop` (Tauri + Svelte desktop app), `apps/web` (Astro
landing page for glassboard.dev), and `packages/ui` (drawing engine, session adapter,
toolbar and overlay components shared by both). Brand assets live in `assets/brand`.

Root scripts forward to each workspace: `npm run desktop <script>`,
`npm run web <script>`, `npm run ui <script>`. `npm run check`, `npm test`, and
`npm run build` run across all workspaces. Rust checks use
`--manifest-path apps/desktop/src-tauri/Cargo.toml`.

## Text

The English sentence is the key. To add a sentence, add it to
`packages/ui/src/contract/messages.json`, call `t()` in a webview or `t!` in Rust, and
add the translation to each file in `apps/desktop/locales`. To add a language, add one
file named with its tag to that directory. `npm run i18n` lists what is missing.
