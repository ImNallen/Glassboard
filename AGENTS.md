# Working in this repository

## Layout

npm workspaces: `apps/desktop` (Tauri + Svelte desktop app), `apps/web` (Astro
landing page for glassboard.dev), and `packages/ui` (drawing engine, session adapter,
toolbar and overlay components shared by both). Brand assets live in `assets/brand`.

Root scripts forward to each workspace: `npm run desktop <script>`,
`npm run web <script>`, `npm run ui <script>`. `npm run check`, `npm test`, and
`npm run build` run across all workspaces. Rust checks use
`--manifest-path apps/desktop/src-tauri/Cargo.toml`.
