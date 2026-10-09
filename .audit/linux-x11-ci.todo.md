# Fix PR 27 Ubuntu CI

1. Reproduce it yourself on the matching surface via the driver skill (Non-negotiables), even when a debug or instrumentation protocol says to ask the user to reproduce. Ask the user only with a stated, specific reason the control surface cannot reach the target, and only after driving it as far as it goes. If it won't reproduce directly, synthesize the trigger, tighten conditions, or instrument until it fires.
2. Binary-search the cause. Form the candidate hypotheses, then rule them out until one survives. Seed them with `how` over the affected subsystem and the **why** skill for regression history. Each pass, take the split that cuts the most remaining problem space, get runtime evidence, eliminate. When program state is unclear, add instrumentation or logging and read it as the code runs. Don't guess. Drive a long or stubborn hunt with Claude Code's `loop` skill. Confirm the surviving *mechanism* with runtime evidence before the step-3 architect/interrogate fan-out.
3. Plan the fix. If it crosses a function boundary, `architect` first. Delegate implementation to a subagent using your configured bug-fix model (default in poteto-mode's Models section) with a specific scope.
4. Verify on the same surface. The original repro now passes. "Inconclusive" or wrong-surface is not a pass. Flag it. Unit tests show branch behavior, not bug absence.
5. Stage the commits so the failing repro lands before the fix in git history. See the **tdd** skill for the failing-test-first cadence when the bug has a cheap local test path. Skip it when the test would be expensive, integration-heavy, or unclear.
   This is the canonical **sequence-verifiable-units** principle skill, the failing test first and the fix on top.
6. Run **Opening a PR**.

## Scope and overrides

- Fix the one actionable Ubuntu compile failure on the existing PR branch.
- The latest user-provided AGENTS.md reserves Git mutations to the maintainer. Leave the fix uncommitted in the working tree. Do not commit, push, or edit the PR.
- Step 5. skip: This is native system-header integration. Preserve the failing container run before the configuration fix instead of adding an implementation-mirroring unit test.
- Step 6. skip: The latest AGENTS.md prohibits PR edits and Git writes. Leave reviewed changes in the prepared checkout for the maintainer.

## Progress

- [x] Read the actual failed job logs and register PR 27 with T3 Code.
- [x] Reproduce the compiler error with Ubuntu 22.04 system headers.
- [x] Confirm the dependency requirement and select a focused fix.
- [x] Delegate implementation in an isolated detached checkout.
- [x] Verify the corrected environment locally and independently review the diff. Final full run passes and independent gpt-6-sol review approves.
- [x] Report the focused working-tree fix and the local validation. GitHub rerun awaits the maintainer's push.

## Throughput checkpoint

- Blocking first steps. Obtain job logs and reproduce the compiler error before editing source or workflow configuration.
- Independent workstreams. Parent runs native containers while a read-only explainer confirms dependency contracts.
- Shared mutable state. Parent owns the prepared checkout and container target volumes. Any writer gets a detached checkout.
- Smallest safe decomposition. One implementation owner keeps check, release, and documented Linux requirements consistent.
