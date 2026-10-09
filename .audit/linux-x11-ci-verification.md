# PR 27 Ubuntu CI fix

The failed [Ubuntu 22.04 job](https://github.com/ImNallen/Glassboard/actions/runs/37850597439/job/113562270754) stops while compiling `libspa 0.10.1`, pulled in through XCap and PipeWire. Bindgen reads Ubuntu's installed PipeWire 0.3.48 headers. The Rust dependency references missing metadata helpers, a missing video `flags` field, and an unsigned modifier where the old headers expose a signed field.

## Reproduction before the fix

[The preserved CI log](linux-ubuntu-ci-failure.log) reports seven errors and exit 101. A native ARM64 Ubuntu 22.04 container compiles the same locked project dependency with `cargo check --locked --manifest-path apps/desktop/src-tauri/Cargo.toml -p libspa`. [That local run](linux-ubuntu22-local-repro.log) produces the same seven errors and exits 101. Source and workflow edits begin after this reproduction.

The [Ubuntu 24.04 probe](linux-ubuntu24-local-probe.log) runs that same command against stock PipeWire 1.0.5 headers and exits 0. [Ubuntu's package index](https://packages.ubuntu.com/noble/libpipewire-0.3-dev) confirms that package version. The older Debian 12 feature verification used PipeWire 0.3.65, which already includes the needed header declarations.

## Focused change

Seven files change. The check and release matrices use Ubuntu 24.04. The native Docker driver also uses Ubuntu 24.04, copies the pinned Rust and Node tools into it, and explicitly installs foundation packages formerly inherited from the Rust image. Its new Cargo target cache avoids reusing native build artifacts from Debian. Shared registry, Git, and npm caches retain portable inputs. The Docker image selects WebKit's fallback renderer for Xvfb, and the GUI driver waits for undo to finish rendering before measuring the unchanged pixel threshold.

README and release instructions document the new build baseline. Debian 12 remains a verified source-build option. Compatibility of release binaries built on Ubuntu 24.04 with older distributions remains unverified. This tradeoff applies to the unpublished Linux preview. No application Rust behavior or dependency version changes.

The regression history is the Linux job introduced by `a66ec91` in this PR. The original design trail chose Ubuntu 22.04, but the recorded native verification used Debian. The failing CI log and the two actual header environments explain the difference. A broader history or incident investigation would add no evidence for this newly introduced build configuration.

## Local validation

The [CI-like Ubuntu 24.04 ARM64 run](linux-ubuntu24-ci-local.log) uses exactly the workflow's native dependency list, without fixture-only GUI packages. It passes the desktop check, 59 JavaScript tests, 55 Rust tests, Clippy with warnings denied, formatting, and the Tauri native debug build. This checks for missing build dependencies that the richer GUI image could otherwise hide.

The first updated Docker run [failed drawing at normal scale](linux-ubuntu24-final/run.log). The same native binary [passes all nine groups](linux-ubuntu24-gui-probe/command.log) when only `WEBKIT_DISABLE_DMABUF_RENDERER=1` changes. Mesa software drivers were already installed. This isolates the failed drawing observation to the virtual desktop rendering path. The setting belongs only to the test image.

The next run [passed normal scale but failed undo at double scale](linux-ubuntu24-complete/run.log). A [same-binary double-scale probe](linux-ubuntu24-undo-probe/command.log) observed 12,077 residual pixels immediately, then zero pixels after another 0.5 seconds. The driver now waits for the original fewer-than-20-pixel requirement, bounded by its existing 15-second timeout. An [intermediate run](linux-ubuntu24-verified/run.log) passes undo and reveals the same asynchronous-rendering observation at the next mark. Drawing and clear now also wait for their original pixel thresholds. All strict assertions remain.

The [final-source complete run](linux-ubuntu24-final-verified/run.log) exits 0. It passes the repository checks, tests, builds, native Rust checks, Clippy, Tauri debug build, and all nine native GUI groups at both 96 DPI with GTK scale 1 and 192 DPI with GTK scale 2. The machine-readable [normal-scale result](linux-ubuntu24-final-verified/dpi-96/results.json) and [double-scale result](linux-ubuntu24-final-verified/dpi-192/results.json) both report pass.

The latest user-provided `AGENTS.md` reserves commits, pushes, branches, and PR edits to the maintainer. The fix stays uncommitted in the prepared checkout. GitHub's new AMD64 result awaits the maintainer's push. PR 27 is already registered with T3 Code.

Actionlint, shell syntax, Python syntax, and `git diff --check` pass. Comment review finds zero changed comments and zero flags. An independent gpt-6-sol reviewer approves the seven-file working-tree fix after checking the full diff, dependency parity, chronological reproduction trail, final run artifacts, and documented limits. It found no source blocker.

## Remaining limits

Local reproduction and native checks use ARM64. GitHub checks the AMD64 runner. The updated optimized release packages, signed update path, physical hardware, other desktops, and login-session checks remain untested. The new runner name is `desktop (ubuntu-24.04)`.
