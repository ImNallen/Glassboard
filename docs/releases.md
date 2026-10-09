# Publish a release

The maintainer reviews, commits, and pushes repository changes. Agents leave edits
in the working tree, as required by [AGENTS.md](../AGENTS.md).

## CI/CD flow

Pull requests and pushes to `main` run **Check**. The active Main ruleset requires
`web`, `desktop (macos-latest)`, and `desktop (windows-latest)` before merging.
The workflow also runs `desktop (ubuntu-24.04)` for experimental Linux X11 support.
Add that job to the repository ruleset before requiring Linux checks for merges.
Those jobs check and test the shared UI, website, and desktop app, run the native
Rust checks and desktop builds on all three platforms, and lint the workflows.

**Release** reruns the same checks at the selected dispatch commit. Only after
they pass does it align the draft to that commit, build installers, sign updater
packages, and verify the updater manifest. The maintainer tests the draft
installers and publishes manually. A published release or a relevant `main`
push can start **Pages**. Its public-download check stops deployment until the
published installers are accessible.

## Keep the updater key

Release builds on macOS, Windows, and Linux AppImages check GitHub once a day for a
newer release. They install an update only if its signature matches the public key in
[`tauri.conf.json`](../apps/desktop/src-tauri/tauri.conf.json). The first public
release must include the updater. Copies installed without it never update themselves.
Linux Debian packages and unpackaged binaries disable automatic updates. Install a
newer Debian package manually.

The private key is `~/.tauri/glassboard-updater.key`. Its password is in
`~/.tauri/glassboard-updater.key.password`. Back up both offline. If either is
lost, no new release can reach existing installs, and every user must reinstall
by hand. To rotate the key, ship a release signed with the old key that carries
the new public key.

The Release workflow reads both from repository secrets. To set them:

```sh
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/glassboard-updater.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD < ~/.tauri/glassboard-updater.key.password
```

Only the workflow creates update packages. It passes
[`tauri.release.conf.json`](../apps/desktop/src-tauri/tauri.release.conf.json),
which turns them on. A local `tauri build` without that file needs no key.

## Prepare the installers

For a new app version, run `npm run version:bump -- patch`, `minor`, `major`, or
an exact stable or prerelease version such as `0.2.0-rc.1`. The command updates
the desktop npm package, its lock entry, and the Rust package and lock versions. Run
`npm run version:check` to confirm that all four copies agree. Review the changed
files before committing. Rebuilding the current draft at the same version does
not need a bump. Keep the website's advertised version on the last public
release until the final installers are ready.

1. Review and commit the intended desktop release changes, then push them to GitHub.
2. Run the **Release** workflow from the intended revision.
   Its **Check** job must pass for that exact commit before the workflow changes
   the draft. The workflow then uses the same SHA for all builds and aligns the
   draft's source target automatically. Start a new run when changing revisions;
   rerun the same workflow only to retry that revision. Do not rerun an older
   revision after starting a newer one for the same version. An existing version tag
   pointing at another commit or an already published release stops the run.
3. Wait for the entire workflow to succeed. Check the summary for the expected
   version tag and full build commit SHA. The macOS job
   must finish notarizing and stapling the DMG, then replace the uploaded draft asset.
   The `updater-manifest` job must also pass. It confirms that the draft's
   `latest.json` names this version and signs an update for Apple silicon, Intel,
   Windows, and Linux x86_64, and that the draft still targets the build commit. Do not publish
   while the workflow is running or publish a draft from a failed run.
4. Open the draft release and confirm it contains every installer. For v0.1.1, these are:
   - `Glassboard_0.1.1_universal.dmg`
   - `Glassboard_0.1.1_x64-setup.exe`
   - `Glassboard_0.1.1_amd64.AppImage`
   - `Glassboard_0.1.1_amd64.deb`
   The signed updater manifest must contain `linux-x86_64` alongside both macOS
   architectures and Windows. The AppImage and its `.sig` supply the Linux update.
5. Download the final draft installers and complete the packaged checks below.
6. Add release notes, then publish the complete draft.

The v0.1.0 draft inspected during launch preparation had a signed, notarized,
stapled macOS DMG. Gatekeeper accepted both the DMG and extracted app, strict
code-signature verification passed, and the app contained arm64 and x86_64 binaries.
Its configured and binary minimum macOS version was 12.0. That is not a substitute
for testing on macOS 12. The Windows installer had no PE signing certificate and
is currently unsigned. A local launch of the extracted macOS app also verified that
the toolbar appeared, Escape hid it, Cmd+Shift+A reopened it from Finder, and Cmd+Q
exited the process. That check reused existing preferences. It did not verify a
clean first launch, screenshot permissions, capture, or Windows runtime behavior.
Recheck these facts if you replace either asset.

## Check the packaged apps

Use the final downloaded installers on clean machines or user accounts. Build and
unit-test success does not cover these checks. Leave each box unchecked until you
have performed it and record the OS version and architecture alongside the result.

- [ ] macOS Apple silicon: install from the DMG into Applications and launch normally.
- [ ] macOS Intel: install and launch the universal app.
- [ ] macOS minimum target: verify launch, drawing, capture, and clipboard on macOS 12 before advertising that minimum as supported.
- [ ] macOS: verify the final DMG with `xcrun stapler validate` and Gatekeeper.
- [ ] Windows x64: run setup, record any unsigned-publisher or SmartScreen prompt, and launch the installed app.
- [ ] All supported platforms: finish the tutorial and toggle drawing with Cmd+Shift+A or Ctrl+Shift+A.
- [ ] All supported platforms: draw, undo, clear, and return to the underlying app.
- [ ] All supported platforms: capture a region, annotate it, copy it, and paste it into another app.
- [ ] macOS: test the Screen Recording permission prompt, denial, and recovery.
- [ ] All supported platforms: open settings and quit from the menu bar or system tray.
- [ ] All supported platforms: test multiple displays and different display scales. Record remaining limitations in the release notes.
- [ ] macOS, Windows, and Linux AppImage: complete the updater rehearsal below before advertising automatic updates.
- [ ] macOS, Windows, and Linux AppImage: after each later release, update the previous public release from Settings and confirm the new version starts.

## Check the Linux preview

Linux support in 0.1.1 is experimental and requires an X11 desktop session, a
compositor, and a system tray host. Wayland sessions, including XWayland, must show
an explanation before Glassboard creates windows. CI checks and release packages
build on Ubuntu 24.04 x86_64 because the locked Rust dependencies require
PipeWire 0.3.65 or newer. Ubuntu 22.04's stock PipeWire is too old. This changes
the build baseline for the unpublished Linux preview. Compatibility of the release
binaries with older distributions is unverified, even where source builds work.
Run `bash scripts/linux/verify.sh` for Ubuntu 24.04 Docker checks on the host's
native CPU architecture. A virtual X11 desktop checks application behavior but
does not cover real GPUs, physical monitors, or every Linux desktop.

- [ ] Linux x86_64: make the final AppImage executable, launch it on X11, and finish the tutorial.
- [ ] Linux x86_64: install the Debian package with `sudo apt install ./Glassboard_0.1.1_amd64.deb` and launch it.
- [ ] Linux: toggle autostart, confirm the desktop autostart entry, and test a new login.
- [ ] Linux: open Settings from the tray menu before and after an update becomes ready.
- [ ] Linux: repeatedly activate the drawing overlay and confirm that visible controls remain above it without moving or taking keyboard focus.
- [ ] Linux: capture the screen and confirm that Glassboard windows and drawings do not appear in the screenshot.
- [ ] Linux: copy a region and a full annotated capture. Paste each in another application after the capture editor closes. Repeat the capture and copy.
- [ ] Linux: test X11 at normal, double, and fractional display scales. Record physical multi-display coverage separately.
- [ ] Linux: launch in a Wayland login with and without an XWayland display. Confirm the explanation and nonzero exit.
- [ ] Linux: launch without a display and confirm the explanation on stderr.

Keep Glassboard running until the copied image is pasted. X11 clipboard ownership
ends when the application exits. AppImage automatic updates require the AppImage
to remain writable. Debian package users install new packages manually.
The current autostart dependency uses `~/.config/autostart` regardless of
`XDG_CONFIG_HOME`. Test login execution separately for custom configuration paths.

## Rehearse an update

Rehearse on each platform with a throwaway key and a local server. The real key
stays in CI.

1. Create a test key:

   ```sh
   npm run desktop tauri signer generate -- -w /tmp/gb-test.key -p test
   ```

2. Write `/tmp/gb-test.conf.json`. It replaces the public key and endpoint, and
   turns on update packages. The separate identifier keeps the rehearsal away from
   your real preferences, logs, and running copy:

   ```json
   {
     "identifier": "dev.glassboard.rehearsal",
     "bundle": { "createUpdaterArtifacts": true },
     "plugins": { "updater": {
       "pubkey": "<contents of /tmp/gb-test.key.pub>",
       "endpoints": ["http://127.0.0.1:8765/latest.json"],
       "dangerousInsecureTransportProtocol": true
     } }
   }
   ```

3. Build the current version. Install it into Applications on macOS, run its setup on Windows, or launch its AppImage on Linux X11:

   ```sh
   export TAURI_SIGNING_PRIVATE_KEY=/tmp/gb-test.key TAURI_SIGNING_PRIVATE_KEY_PASSWORD=test
   npm run desktop tauri build -- --config /tmp/gb-test.conf.json
   ```

4. Record the current version, run `npm run version:bump -- patch`, and build again.
   Do not commit the change. Copy the update package and its `.sig` file into
   `/tmp/gb-serve`. On macOS the package is `Glassboard.app.tar.gz`. On Windows it
   is the `-setup.exe`. On Linux it is the `.AppImage`.
5. Write `/tmp/gb-serve/latest.json` with the new version and one platform entry,
   such as `darwin-aarch64`, `windows-x86_64`, or `linux-x86_64`. Its `url` points at the served
   package, and its `signature` is the contents of the `.sig` file. Then serve it:

   ```sh
   python3 -m http.server 8765 -d /tmp/gb-serve
   ```

6. Change one character of the signature in `latest.json`. Open the installed app,
   open Settings, and select **Check for updates** next to the GitHub icon. The
   status line reports a signature mismatch. Restore the signature.
7. Stop the server and select **Check for updates** again. The status line reports
   that GitHub is unreachable, without the red error style. Start the server again.
8. On macOS, quit the installed app and open the old build from its mounted DMG.
   Select **Check for updates**, then the highlighted install icon.
   The status line asks you to move Glassboard to Applications. Quit that copy.
9. Open the installed app and select **Check for updates**. The icon becomes a
   progress ring while the update downloads, then a highlighted install icon. The
   tray menu shows **Restart to update**.
10. Change the toolbar position, then select **Restart to update** in the tray.
    Confirm that the new version starts once, keeps the toolbar position, and
    responds to the drawing shortcut. On Windows, confirm that no stale tray icon
    remains.
11. Restore the recorded version with `npm run version:bump -- <previous-version>`.

## Deploy the website

The website advertises the version in
[`apps/web/src/data/release.json`](../apps/web/src/data/release.json). Keep that
record on the last public release while the next desktop version is in development.
Changing the desktop package version does not change website downloads.

1. Update the advertised version and exact asset filenames only when all final
   installers are ready. Linux uses an optional `linux` asset ending in `.AppImage`.
   Add it only after publishing that installer. Older releases keep only their
   macOS and Windows assets. Have the maintainer review, commit, and push the website changes.
2. Publish the complete draft before deployment. A push before publication can run
   the Pages workflow, but its public-download check must fail and stop deployment.
3. From the repository root, run the public check:

   ```sh
   npm run web verify:downloads
   ```

   The command requests release metadata and every advertised installer URL without credentials.
   It rejects a draft, a wrong tag, a missing or incomplete asset, or an inaccessible
   download. A GitHub API outage or rate limit also stops deployment. Retry after the
   cause is resolved rather than bypassing the check.
4. Check **Settings → Pages**. Use **GitHub Actions** as the source. This was already
   selected during launch preparation; recheck it if the repository settings change.
5. Run **Pages** from `main`, or let a relevant push or published-release event run it.
   A release event checks out the default branch so a desktop tag does not deploy
   stale website code. The workflow installs dependencies from the workspace root,
   checks and tests the UI and website, builds the website, verifies public downloads,
   and uploads `apps/web/dist` for deployment.
6. Open the deployed URL. Check every download link, `/install/`, icons, and release
   notes. Test a narrow screen and disable JavaScript to confirm all downloads remain
   available. With JavaScript enabled, test the browser drawing demo.

Local website builds do not fetch GitHub release metadata. Use `npm run web build`
to preview unpublished website changes. To check a project-site path before DNS
cutover, build with:

```sh
PAGES_BASE_URL=https://imnallen.github.io/Glassboard npm run web build
```

In Actions, `actions/configure-pages` supplies `PAGES_BASE_URL`. Astro uses that
URL for the site origin and base path. Check the deployed canonical URLs after a
domain change. See [Astro's GitHub Pages deployment guide](https://docs.astro.build/en/guides/deploy/github/).

## Connect glassboard.dev

At launch preparation, GitHub Pages already had `glassboard.dev` configured as the
custom domain, but no website deployment had completed and DNS still pointed at
Squarespace. Recheck the current state before changing records.

1. Deploy the website and confirm that the Pages workflow succeeds.
2. In **Settings → Pages**, confirm the custom domain is `glassboard.dev`.
3. At the DNS provider, replace the Squarespace website records for the apex with
   these four GitHub Pages A records:

   ```text
   185.199.108.153
   185.199.109.153
   185.199.110.153
   185.199.111.153
   ```

4. Point the `www` CNAME at `imnallen.github.io`, without a repository path. Remove
   conflicting website records for those names, including old apex AAAA records if
   they point elsewhere. Preserve unrelated MX and TXT records used for mail,
   verification, and other services.
5. Wait for DNS checks and the HTTPS certificate to complete in GitHub Pages.
   Enable **Enforce HTTPS** when available.
6. Verify `https://glassboard.dev`, `https://glassboard.dev/install/`, and the `www`
   redirect. Re-run Pages if its configured base URL changed after the previous build.

GitHub Actions deployments do not require a `public/CNAME` file. Follow GitHub's
[custom-domain guide](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site)
for current DNS and HTTPS instructions. Verify domain ownership as described in
[GitHub's domain verification guide](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/verifying-your-custom-domain-for-github-pages).

## Draft v0.1.0 release notes

Review this copy against the packaged checks before publishing it.

> Glassboard lets you draw over your screen, then clear your drawings with a shortcut.
> You can also capture a region, annotate it, and copy the result to the clipboard.
> Everything runs on your machine, with no account or server.
>
> Download the universal DMG for Apple silicon or Intel Macs, or the x64 setup EXE
> for Windows. On macOS, open the DMG and drag Glassboard into Applications. On
> Windows, run the installer. The macOS app is signed and notarized. The Windows
> installer is currently unsigned and may show an Unknown publisher or SmartScreen
> warning.
>
> Press Cmd+Shift+A on macOS or Ctrl+Shift+A on Windows to start drawing. While
> drawing, press Cmd+S or Ctrl+S to capture a region, then Cmd+C or Ctrl+C to copy it.
> macOS asks for Screen Recording permission the first time you capture.
>
> Known limitations: restart Glassboard after connecting or rearranging displays.
> Drawings stay at a fixed screen position. For video calls, share the whole screen
> to include annotations. Windows, fullscreen apps, Stage Manager, and mixed-DPI
> setups still need more testing.
