# Public download and installation

- [x] `how` over the affected subsystem.
- [x] `architect` for parallel design exploration.
- [x] Write the throughput checkpoint as four todo items. A dimension that genuinely does not apply (single file, no fan-out) keeps its item with `n/a: <reason>` rather than being dropped:
  - [x] **Blocking first steps.** Gates run before fan-out.
  - [x] **Independent workstreams.** Disjoint files, services, or layers parallelize. Shared writes serialize.
  - [x] **Shared mutable state.** Default to splitting the target (the **separate-before-serializing-shared-state** principle skill). Serialize only for real invariants.
  - [x] **Smallest safe decomposition.** If one worker is best, name why.
- [x] Delegate code-writing to a subagent using your configured feature model with a specific scope.
- [x] Verify on the matching surface. "Inconclusive" or wrong-surface is not a pass. Flag it.
- [ ] Rebase into small, ordered commits. Stack follow-ups.
  skip: AGENTS.md prohibits commits, branches and pushes. Use an isolated filesystem copy for implementation, then copy reviewed changes into this working tree.
- [ ] If the design is contested, `interrogate` before shipping.
  skip: apply only if design exploration reveals an unresolved disagreement.
- [ ] Run **Opening a PR**.
  skip: AGENTS.md prohibits opening or editing PRs.

## Architecture and arena

- [x] Ground.
- [x] Sketch.
- [x] Agree.
- [x] Implement.
- [ ] Scrap.
  skip: apply only if implementation invalidates the chosen shape.
- [x] Frame.
- [x] Fan out.
- [x] Cross-judge.
- [x] Pick.
- [x] Graft.
- [x] Verify.

## Throughput checkpoint

- Blocking first steps. GitHub Pages selected by the user. Confirm draft asset names, then select the release-link model before writing code.
- Independent workstreams. Compare designs read-only while the parent inspects the installers and hosting requirements.
- Shared mutable state. One implementation owner writes an isolated filesystem copy. The parent alone updates this checklist and copies reviewed changes back.
- Smallest safe decomposition. One owner keeps the download table, installation copy, release workflow and Pages deployment contract consistent.

## Scope and acceptance

- [x] Direct macOS universal and Windows x64 download links visible on desktop and mobile.
- [x] Installation guidance, first-use shortcuts and macOS permission recovery.
- [x] GitHub Pages workflow, custom-domain and DNS instructions.
- [x] A repeatable release/download validation command and launch checklist.
- [x] Verify actual draft asset metadata and macOS signing/notarization without publishing.
- [x] Browser verification of production site, mobile actions and demo behavior.
- [x] Document native Windows and clean-machine verification still required.

## Design decision

Candidate A is the base. Both candidates chose an explicit advertised release independent of desktop development versions. The cross-judge selected A for shared link ownership and Pages base-path handling. Graft B's dedicated installation page and anonymous release-by-tag validation followed by exact installer URL checks. Reject runtime or build-time discovery for ordinary local builds. One isolated implementation owner writes the fixed contract.

## Installer inspection

- Downloaded draft DMG and Windows installer SHA-256 values match GitHub asset digests.
- DMG stapler validation passed. Gatekeeper accepted DMG and bundled app as Notarized Developer ID. Strict codesign verification passed.
- Bundled Mach-O contains x86_64 and arm64, with macOS minimum 12.0 declared in both slices and Info.plist. This does not replace a minimum-OS runtime test.
- Windows installer PE Authenticode certificate table is empty. Installation guidance must state that it is unsigned.
- Launched the downloaded macOS app from an extracted temporary bundle. Its native toolbar appeared. Escape hid its annotation windows; Cmd+Shift+A from Finder reopened them. Quit the app and confirmed the process exited. Existing preferences were reused, so no clean-machine, first-launch, permission or Windows smoke pass is claimed.
- Enabled GitHub Pages using the workflow source. No deployment, commit, push or release publication performed.

## Previous investigation

- [x] Route through the **how** skill. For motivation questions, also route through the **why** skill.
  skip: why is not needed for the current-state and next-build question.
- [x] Throughput checkpoint stays one line: `throughput checkpoint: n/a, read-only investigation`.
- [x] Produce the `how`-shaped output (Overview / Key Concepts / How It Works / Where Things Live / Gotchas), or a recommendation with a tradeoffs table if the request is a decision between alternatives.
- [x] Apply the **unslop** skill to the reply.

throughput checkpoint: n/a, read-only investigation

- [x] Inspect implemented product paths and recent repository history.
- [x] Run current checks and tests.
- [x] Check release availability and landing-page download path.
- [x] Recommend the next user outcome with a completion criterion.

## Evidence

- `npm run check` passed with no diagnostics.
- `npm test` passed 228 JavaScript tests.
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` passed 38 Rust tests.
- `npm run build` passed for the desktop frontend and website.
- `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml -- -D warnings` passed.
- Current HEAD GitHub Check run 37083229153 succeeded.
- Authenticated GitHub release lookup found v0.1.0 in draft with macOS DMG and Windows installer assets. Public release listing is empty.
- Release run 37080660903 succeeded on both operating systems, including the macOS DMG notarization and stapling step.
- Live glassboard.dev showed a Coming Soon placeholder in the collaborative browser.
- The repository website points to Build from source and hides the action group on mobile.
- Native packaged installation and OS interaction were not exercised in this investigation.
- The production website build opened in the collaborative browser, and Try it on this page opened the drawing toolbar. Escape exited the demo.

## Implementation verification

- Reviewed implementation copied from the isolated owner into this working tree. Independent review found no blocking issues or comment changes.
- `npm run check` passed across all workspaces with zero diagnostics.
- `npm test` passed 235 tests: 41 desktop, 7 website release checks, and 187 shared UI.
- `npm run build`, Pages workflow actionlint, and `git diff --check` passed.
- Production builds verified at the custom-domain root and GitHub project path; generated canonical, navigation, favicon, and installer URLs were correct.
- Browser checked desktop and mobile home/install pages, including dark mode. Both platform downloads remain available on mobile, with no horizontal overflow or console/network errors.
- The production browser demo opened its annotation toolbar; Escape closed it.
- The anonymous live download verifier correctly stopped with HTTP 404 because v0.1.0 is still a draft.
- Actual draft macOS artifact signing, notarization, architecture, launch and global toggle were checked as recorded above. Windows installation and clean-machine/macOS permission flows remain unverified.

## Remaining launch steps

- [ ] Maintainer reviews, commits, and pushes these changes, per AGENTS.md.
- [ ] Complete the remaining packaged checks in `docs/releases.md`, then publish the complete v0.1.0 draft with release notes.
- [ ] Pages workflow passes its anonymous download check and deploys the site.
- [ ] Replace Squarespace DNS records with GitHub Pages records; wait for certificate provisioning and enforce HTTPS.
- [ ] Verify both public downloads and installation links on the live custom domain.

GitHub Pages workflow hosting and the `glassboard.dev` custom-domain setting are already configured. No source commit, push, PR, release publication, DNS change, or website deployment was performed.
