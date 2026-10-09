# Linux 0.1.1 verification

The experimental X11 implementation passes native GUI checks in Debian Bookworm ARM64. The rebuilt ARM64 debug AppImage and installed Debian package pass the same checks. These results support an experimental release, with the release checks below still pending.

## Native execution

Docker runs Xvfb, Openbox, xcompmgr, and an XFCE tray host. The driver operates the actual Tauri application through native keyboard, pointer, and accessibility input. An independent GTK application provides the drawing and capture reference. Independent `xclip` reads verify clipboard images after the capture editor closes.

| Run | Display and GTK scale | Result |
| --- | --- | --- |
| [Complete Docker command](final-native/run.log) | 1280 by 800 at 96 DPI, GTK 1x | [All nine groups pass](final-native/dpi-96/results.json) |
| Same command | 2560 by 1600 at 192 DPI, GTK 2x | [All nine groups pass](final-native/dpi-192/results.json) |
| [Rebuilt debug AppImage](packaged-appimage-run.log) | 2560 by 1600 at 192 DPI, GTK 2x | [All nine groups pass](packaged-appimage/dpi-192/results.json) |
| [Installed debug Debian package](packaged-deb-run.log) | 1280 by 800 at 96 DPI, GTK 1x | [All nine groups pass](packaged-deb/dpi-96/results.json) |

The groups cover startup refusals, tutorial dismissal, drawing and pointer interception, undo, toolbar order and click-through, clear, plain capture, annotated capture, and tray Settings with autostart registration. Three focus and visibility cycles preserve keyboard focus and toolbar bounds. The toolbar measures 744 by 76 pixels at GTK 1x and 1488 by 152 pixels at GTK 2x.

The packaged run copies a 780 by 240 image with zero pixels different from the independent reference. Its second, annotated 1100 by 300 image differs in 16810 pixels. Both images remain readable after the capture editor closes. The packaged autostart entry names the AppImage executable. Enabling and disabling autostart changes the actual desktop registration.

The driver simulates Wayland and XWayland environment variables on X11. Each refusal exits with code 1 and an explanation. This checks the startup guard and its X11 dialog, not the dialog in a real Wayland login.

## Build and static checks

- The [complete Docker run](final-native/run.log) passes `npm ci`, `npm run check`, `npm test`, and `npm run build`. It runs 269 JavaScript tests, 55 Linux Rust tests, all-target Clippy with warnings denied, and a native debug build.
- [macOS Rust tests](macos-test.log) pass all 58 tests. [macOS all-target Clippy](macos-clippy.log) passes with warnings denied.
- Rust formatting, `git diff --check`, desktop version consistency, and [actionlint 1.7.12](actionlint.log) pass.
- The [native package build](native-package-build.log) produces `Glassboard_0.1.1_arm64.deb` and `Glassboard_0.1.1_aarch64.AppImage`. The AppImage executes through extraction mode because the container lacks FUSE.
- The [Debian installation](debian-install.log) succeeds with the test environment's dependencies already present. Its installed `/usr/bin/glassboard` passes all nine native groups. Its autostart entry names that installed executable.
- The website retains published 0.1.0 metadata. Its optional future Linux download renders during a temporary production-build check. The public metadata was restored before the final build.

Run the complete native command from the repository root on a Docker host.

```sh
bash scripts/linux/verify.sh
```

It builds in an isolated container from a read-only source mount and saves results under `.audit/linux-x11`. `GLASSBOARD_LINUX_EVIDENCE` selects another evidence directory. `GLASSBOARD_LINUX_PLATFORM=linux/amd64` selects x86_64 when the Docker host supports it. The observed run uses native ARM64, without x86_64 emulation.

## Remaining release checks

The observed native checks use Debian ARM64. Ubuntu 22.04 x86_64 CI and release results need separate validation. The local packages are debug ARM64 builds. Installation on a minimal desktop, optimized x86_64 package execution, FUSE launch, signed update installation, and the final release manifest need release validation.

Physical GPUs, multiple monitors, negative monitor origins, fractional GUI scaling, other window managers, and other tray hosts remain untested. Rust tests cover fractional monitor-coordinate conversion and negative origins. Full-monitor copy, real Wayland login refusal, autostart during a new login, and tray rebuilding after an update remain untested. macOS and Windows GUI regression checks remain pending.

The locked autostart dependency writes to `~/.config/autostart` and ignores `XDG_CONFIG_HOME`. Registration at that default location passes. Custom config locations require a manual desktop entry, as documented in the README.

## Decisions and delivery

The implementation keeps Linux behavior in the existing feature modules. A broad platform abstraction was rejected during the [design comparison](../linux-x11-design.md). Native testing found and fixed a hidden GTK window crash, nonresizable GTK window sizing, and controls below the fullscreen overlay's window-manager layer.

Model the Domain and Type System Discipline shaped the typed startup refusal and unique monitor selection. Separate Before Serializing Shared State isolated writing delegates. Build the Lever produced the rerunnable Docker driver. Test Behavior Not Implementation and Prove It Works required independent pixels, clipboard reads, and packaged execution. Fix Root Causes and Attack the Premise led to GTK sizing and transient-window relationships after direct raise and restack attempts failed.

The source changes were uncommitted during verification. The later request to create a PR authorized two ordered commits and a push of the existing feature branch. [PR 27](https://github.com/ImNallen/Glassboard/pull/27) contains the implementation and Docker checks. Local audit files and Docker artifacts remain untracked. No release was published. The [decision trail](../linux-x11.tsv) records accepted choices, rejected experiments, actual verification results, and the later delivery authorization.

The [independent final review](../linux-x11-review.md) approves this experimental scope and confirms that the transcript and decision trail agree. It retains the release and hardware coverage limits above.
