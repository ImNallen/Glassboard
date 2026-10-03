# In-app updater

- [x] `how` over the affected subsystem.
  Traced by hand: Rust state and `publish`, `Action` and the tray menu, autostart commands, Settings General tab, release workflow and draft flow. Grounding in /tmp/arena-updater/grounding.md.
- [x] `architect` for parallel design exploration.
  Base c1 (Opus). The cross-judge agreed. Grafts and rejections are in /tmp/arena-updater/synthesis.md.
- [x] Write the throughput checkpoint as four todo items. A dimension that genuinely does not apply (single file, no fan-out) keeps its item with `n/a: <reason>` rather than being dropped:
  - [x] **Blocking first steps.** Generate the signing keypair so the pubkey can go into config. Pick the base design. Both done.
  - [x] **Independent workstreams.** n/a: Rust module, tray, Settings row, and release workflow share one wire contract (`UpdateStatus` and two `Action` names). Splitting them would make each worker guess the other side.
  - [x] **Shared mutable state.** One implementation owner writes the working tree. The parent only reviews until the owner reports.
  - [x] **Smallest safe decomposition.** One owner, because the wire contract couples every file.
- [x] Delegate code-writing to a subagent using your configured feature model with a specific scope.
- [x] Verify on the matching surface. "Inconclusive" or wrong-surface is not a pass. Flag it.
  macOS arm64 packaged rehearsal against a localhost feed with a throwaway key. 0.1.0 found 0.1.1 60 s after launch, then downloaded and verified it. The tray showed Restart to update, and the click installed it and relaunched one 0.1.1 process. The next check found nothing, and the tray item was gone. Not verified: Windows, the universal build, the real key in CI, and the updater-manifest job.
- [ ] Rebase into small, ordered commits. Stack follow-ups.
  skip: AGENTS.md prohibits commits, branches and pushes.
- [ ] If the design is contested, `interrogate` before shipping.
  skip: the cross-judge and the three runners converged on Rust ownership, auto-download, and click-to-restart.
- [ ] Run **Opening a PR**.
  skip: AGENTS.md prohibits opening or editing PRs.

## Architecture and arena

- [x] Ground.
- [x] Sketch.
- [ ] Agree.
  skip: no checkpoint requested.
- [x] Implement.
- [ ] Scrap.
  skip: implementation didn't invalidate the shape.
- [x] Frame.
- [x] Fan out.
- [x] Cross-judge.
- [x] Pick.
- [x] Graft.
- [x] Verify.

## Rubric

1. Correct against tauri-plugin-updater v2 and this release pipeline (draft releases, universal macOS, notarization post-step, unsigned NSIS).
2. Fits a quiet menu-bar tool. No nagging, no surprise restart mid-drawing, clear state and recovery.
3. Small surface. Follows the autostart precedent. No ripple into the shared web session model without a reason.
4. One state type for the update lifecycle. Repeated or concurrent checks and installs converge.
5. Verifiable end to end before a public release.
