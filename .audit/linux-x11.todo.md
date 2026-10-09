# Linux X11 support for 0.1.1

1. `how` over the affected subsystem.
2. `architect` for parallel design exploration.
3. Write the throughput checkpoint as four todo items. A dimension that genuinely does not apply (single file, no fan-out) keeps its item with `n/a: <reason>` rather than being dropped:
   - **Blocking first steps.** Gates run before fan-out.
   - **Independent workstreams.** Disjoint files, services, or layers parallelize. Shared writes serialize.
   - **Shared mutable state.** Default to splitting the target (the **separate-before-serializing-shared-state** principle skill). Serialize only for real invariants.
   - **Smallest safe decomposition.** If one worker is best, name why.
4. Delegate code-writing to a subagent using your configured feature model (default in poteto-mode's Models section) with a specific scope (file paths, named data shape and its organizing structure per **principle-model-the-domain**, a state machine over scattered booleans, a table/registry over branching, a typed model over repeated shape assumptions, chosen before the delegate writes logic, and success criteria). When the implementation admits multiple valid shapes (error handling, abstraction layer, test structure), delegate via the **arena** skill instead so the runners surface the alternatives and the cross-judge guards the pick. Mandatory: no skip-with-reason escape, and Laziness Protocol does not override it (the gain is review separation, not lines saved). A subagent forbidden to spawn satisfies this by owning the diff directly with the same review separation. No "standing by" reply that waits on a nested agent. **Give every file-writing delegate its own worktree** (spawn it with `isolation: "worktree"`, or hand it an exclusive branch), and do not write files or run a suite in a worktree a delegate still holds. On Claude Code, `isolation: "worktree"` branches from the remote default branch unless the `worktree.baseRef` setting is `"head"`. When the delegate builds on commits the default branch lacks, commit them, create its worktree with `git worktree add <path> -b <delegate-branch> HEAD`, and name that base commit in the brief. Fencing a file in the brief's prose is not a lock (**principle-separate-before-serializing-shared-state**). Comments per **Comments**. Surgical edits, re-ground against the source for upstream-derived files. Port shared-primitive improvements to all consumers and verify each. Commit liberally.
5. Verify on the matching surface. "Inconclusive" or wrong-surface is not a pass. Flag it.
6. Rebase into small, ordered commits. Stack follow-ups.
   Use the **sequence-verifiable-units** principle skill, building, verifying, and committing each small unit before the next.
7. If the design is contested, `interrogate` before shipping.
8. Run **Opening a PR**.


## Progress

- [x] Ground the affected app, release, and website behavior.
- [x] Compare two design sketches and obtain an independent judgment.
- [x] Implement in one detached worktree.
- [x] Build and exercise the native app on a virtual Linux X11 desktop.
- [x] Run repository checks and review the final diff.
- [x] Report evidence and remaining limits in `.audit/linux-x11/verification.md`.

## Throughput checkpoint

- Blocking first steps. Choose the X11 support boundary and verify Docker before implementation.
- Independent workstreams. Native app implementation and preparation of an isolated Docker test environment.
- Shared mutable state. The implementation owner has a detached worktree. The parent owns only the audit and Docker environment until the owner returns.
- Smallest safe decomposition. One owner keeps native behavior, packaging, and download metadata consistent.

## Overrides

- Feature step 6. skip: AGENTS.md forbids commits and branch creation. Verify ordered units and retain a working-tree diff.
- Feature step 7. skip unless the design is contested.
- Feature step 8. skip: AGENTS.md forbids opening or editing pull requests.

## Authorized PR delivery

The later direct request to create a PR supersedes the maintainer-only Git restriction for this delivery. Use the existing `t3code/assess-linux-support` branch. Keep audit files and Docker outputs local.

- [x] Re-read instructions, select the PR tool, and verify the target branch.
- [x] Apply the pre-commit cleanup check to the reviewed diff.
- [x] Retain the independent source and comment reviews for the unchanged implementation.
- [x] Commit the product change and rerunnable Docker checks as ordered units.
- [x] Push the feature branch without force.
- [x] Open a ready pull request and register it with T3 Code.
- [x] Confirm the PR contents and report its URL.

## Architect

1. Ground
2. Sketch
3. Agree
4. Implement
5. Scrap

## Arena

1. Frame
2. Fan out
3. Cross-judge
4. Pick
5. Graft
6. Verify
