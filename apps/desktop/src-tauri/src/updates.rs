use crate::{
    commands::transition,
    preference_saves::PreferenceSaves,
    session::Transition,
    state::{report, snapshot},
    tray,
};
use serde::Serialize;
use std::{
    io::ErrorKind,
    mem,
    sync::Mutex,
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Error, Update, UpdaterExt};

const FIRST_CHECK: Duration = Duration::from_secs(60);
const TICK: Duration = Duration::from_secs(60 * 60);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const RETRY: Duration = Duration::from_secs(60 * 60);
const TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub(crate) enum UpdateStatus {
    Disabled,
    Idle,
    Checking,
    Downloading {
        version: String,
        percent: Option<u8>,
    },
    Ready {
        version: String,
    },
    Installing {
        version: String,
    },
    Failed {
        message: String,
    },
}

enum Phase<Verified> {
    Disabled,
    Idle,
    Checking,
    Downloading(String, Option<u8>),
    Ready(String, Verified),
    Installing(String),
    Failed(String),
}

impl<Verified> Phase<Verified> {
    fn begin_check(&mut self) -> bool {
        let idle = matches!(self, Phase::Idle | Phase::Failed(_));
        if idle {
            *self = Phase::Checking;
        }
        idle
    }
    fn found(&mut self, version: Option<String>) {
        if matches!(self, Phase::Checking) {
            *self = version.map_or(Phase::Idle, |version| Phase::Downloading(version, None));
        }
    }
    fn progress(&mut self, percent: Option<u8>) {
        if let Phase::Downloading(_, shown) = self {
            *shown = percent;
        }
    }
    fn downloaded(&mut self, download: Verified) {
        if let Phase::Downloading(version, _) = self {
            *self = Phase::Ready(mem::take(version), download);
        }
    }
    fn begin_install(&mut self) -> Option<Verified> {
        match mem::replace(self, Phase::Checking) {
            Phase::Ready(version, download) => {
                *self = Phase::Installing(version);
                Some(download)
            }
            other => {
                *self = other;
                None
            }
        }
    }
    fn fail(&mut self, message: String) {
        if matches!(
            self,
            Phase::Checking | Phase::Downloading(..) | Phase::Installing(_)
        ) {
            *self = Phase::Failed(message);
        }
    }
    fn status(&self) -> UpdateStatus {
        match self {
            Phase::Disabled => UpdateStatus::Disabled,
            Phase::Idle => UpdateStatus::Idle,
            Phase::Checking => UpdateStatus::Checking,
            Phase::Downloading(version, percent) => UpdateStatus::Downloading {
                version: version.clone(),
                percent: *percent,
            },
            Phase::Ready(version, _) => UpdateStatus::Ready {
                version: version.clone(),
            },
            Phase::Installing(version) => UpdateStatus::Installing {
                version: version.clone(),
            },
            Phase::Failed(message) => UpdateStatus::Failed {
                message: message.clone(),
            },
        }
    }
}

struct VerifiedDownload {
    update: Update,
    bytes: Vec<u8>,
}

pub(crate) struct Updates(Mutex<Phase<VerifiedDownload>>);

pub(crate) fn start(app: &tauri::App) {
    let signed_release = !cfg!(debug_assertions);
    #[cfg(target_os = "linux")]
    let signed_release = signed_release && app.env().appimage.is_some();
    app.manage(Updates(Mutex::new(if signed_release {
        Phase::Idle
    } else {
        Phase::Disabled
    })));
    if signed_release {
        let app = app.handle().clone();
        std::thread::spawn(move || schedule(&app));
    }
}

fn schedule(app: &AppHandle) {
    std::thread::sleep(FIRST_CHECK);
    let mut last = None;
    loop {
        let now = SystemTime::now();
        let failed = matches!(status(app), UpdateStatus::Failed { .. });
        if due(last, failed, now) {
            check(app);
            last = Some(now);
        }
        std::thread::sleep(TICK);
    }
}

/// Wall clock, so time asleep counts.
fn due(last: Option<SystemTime>, failed: bool, now: SystemTime) -> bool {
    let wait = if failed { RETRY } else { INTERVAL };
    last.is_none_or(|last| {
        now.duration_since(last)
            .map_or(true, |elapsed| elapsed >= wait)
    })
}

pub(crate) fn status(app: &AppHandle) -> UpdateStatus {
    app.state::<Updates>().0.lock().unwrap().status()
}

#[tauri::command]
pub(crate) fn get_update(app: AppHandle) -> UpdateStatus {
    status(&app)
}

fn advance(app: &AppHandle, change: impl FnOnce(&mut Phase<VerifiedDownload>)) {
    change(&mut app.state::<Updates>().0.lock().unwrap());
    publish(app);
}

fn publish(app: &AppHandle) {
    let status = status(app);
    if let Err(error) = app
        .emit("update", &status)
        .and_then(|()| tray::show_update(app, &status))
    {
        report(app, error);
    }
}

pub(crate) fn check(app: &AppHandle) {
    if !app.state::<Updates>().0.lock().unwrap().begin_check() {
        return;
    }
    publish(app);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = fetch(&app).await {
            log::warn!("Could not update: {error}");
            advance(&app, |phase| phase.fail(explain(&error)));
        }
    });
}

async fn fetch(app: &AppHandle) -> Result<(), Error> {
    let cleanup = app.clone();
    let found = app
        .updater_builder()
        .timeout(TIMEOUT)
        // Windows exits inside `install`, which skips the tray icon's removal.
        .on_before_exit(move || cleanup.cleanup_before_exit())
        .build()?
        .check()
        .await
        .or_else(|error| match error {
            // GitHub has no manifest yet, or none for this system: nothing new.
            Error::ReleaseNotFound | Error::TargetNotFound(_) | Error::TargetsNotFound(_) => {
                Ok(None)
            }
            error => Err(error),
        })?;
    advance(app, |phase| {
        phase.found(found.as_ref().map(|update| update.version.clone()))
    });
    let Some(mut update) = found else {
        return Ok(());
    };
    // `check` does not pass the builder's timeout on to the download.
    update.timeout = Some(TIMEOUT);
    let mut received = 0;
    let mut shown = None;
    let bytes = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                let percent = percent(received, total);
                // Chunks arrive per network read; publish only whole-percent changes.
                if percent != shown {
                    shown = percent;
                    advance(app, |phase| phase.progress(percent));
                }
            },
            || {},
        )
        .await?;
    log::info!("Downloaded Glassboard {}", update.version);
    advance(app, |phase| {
        phase.downloaded(VerifiedDownload { update, bytes })
    });
    Ok(())
}

fn percent(received: u64, total: Option<u64>) -> Option<u8> {
    total
        .filter(|&total| total > 0)
        .map(|total| (received.min(total) * 100 / total) as u8)
}

pub(crate) fn install(app: &AppHandle) {
    let Some(download) = app.state::<Updates>().0.lock().unwrap().begin_install() else {
        return;
    };
    publish(app);
    let app = app.clone();
    // The macOS elevation prompt waits on the main thread, so this cannot run there.
    tauri::async_runtime::spawn_blocking(move || {
        log::info!("Installing Glassboard {}", download.update.version);
        // Windows exits inside `install` without running the exit handlers.
        let installed = app
            .state::<PreferenceSaves>()
            .flush()
            .map_err(|error| {
                log::warn!("Could not save preferences before updating: {error}");
                "Couldn't save preferences, so the update wasn't installed.".to_string()
            })
            .and_then(|()| {
                download.update.install(&download.bytes).map_err(|error| {
                    log::warn!("Could not install the update: {error}");
                    explain(&error)
                })
            });
        match installed {
            Ok(()) => app.request_restart(),
            Err(message) => {
                advance(&app, |phase| phase.fail(message));
                if !snapshot(&app).settings_open {
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        if let Err(error) = transition(&handle, Transition::Settings) {
                            report(&handle, error);
                        }
                    });
                }
            }
        }
    });
}

fn explain(error: &Error) -> String {
    match error {
        Error::Reqwest(_) | Error::Network(_) => "Couldn't reach GitHub to check for updates.",
        Error::Minisign(_) | Error::Base64(_) | Error::SignatureUtf8(_) => {
            "The update's signature didn't match, so it was discarded."
        }
        Error::Io(error)
            if matches!(
                error.kind(),
                ErrorKind::ReadOnlyFilesystem | ErrorKind::CrossesDevices
            ) =>
        {
            "Move Glassboard to Applications to install updates."
        }
        Error::Io(error) if error.kind() == ErrorKind::PermissionDenied => {
            "Glassboard couldn't replace itself. Check that you can change Applications."
        }
        _ => "The update failed. Try again later.",
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every() -> Vec<Phase<()>> {
        vec![
            Phase::Disabled,
            Phase::Idle,
            Phase::Checking,
            Phase::Downloading("0.2.0".into(), Some(40)),
            Phase::Ready("0.2.0".into(), ()),
            Phase::Installing("0.2.0".into()),
            Phase::Failed("offline".into()),
        ]
    }

    fn table(event: impl Fn(&mut Phase<()>)) -> Vec<(UpdateStatus, UpdateStatus)> {
        every()
            .into_iter()
            .map(|mut phase| {
                let before = phase.status();
                event(&mut phase);
                (before, phase.status())
            })
            .collect()
    }

    fn changed(rows: Vec<(UpdateStatus, UpdateStatus)>) -> Vec<(UpdateStatus, UpdateStatus)> {
        rows.into_iter().filter(|(from, to)| from != to).collect()
    }

    fn downloading(percent: Option<u8>) -> UpdateStatus {
        UpdateStatus::Downloading {
            version: "0.2.0".into(),
            percent,
        }
    }
    fn ready() -> UpdateStatus {
        UpdateStatus::Ready {
            version: "0.2.0".into(),
        }
    }
    fn installing() -> UpdateStatus {
        UpdateStatus::Installing {
            version: "0.2.0".into(),
        }
    }
    fn failed(message: &str) -> UpdateStatus {
        UpdateStatus::Failed {
            message: message.into(),
        }
    }

    #[test]
    fn only_idle_and_failed_start_a_check() {
        for mut phase in every() {
            let before = phase.status();
            let started = phase.begin_check();
            let idle = matches!(before, UpdateStatus::Idle | UpdateStatus::Failed { .. });
            assert_eq!(started, idle, "{before:?}");
            assert_eq!(
                phase.status(),
                if idle { UpdateStatus::Checking } else { before }
            );
        }
    }

    #[test]
    fn a_check_finds_nothing_or_a_version_to_download() {
        assert_eq!(
            changed(table(|phase| phase.found(None))),
            [(UpdateStatus::Checking, UpdateStatus::Idle)]
        );
        assert_eq!(
            changed(table(|phase| phase.found(Some("0.2.0".into())))),
            [(UpdateStatus::Checking, downloading(None))]
        );
    }

    #[test]
    fn only_a_download_in_progress_reports_progress() {
        assert_eq!(
            changed(table(|phase| phase.progress(Some(75)))),
            [(downloading(Some(40)), downloading(Some(75)))]
        );
    }

    #[test]
    fn progress_is_a_whole_percent_of_a_known_size() {
        assert_eq!(percent(0, Some(200)), Some(0));
        assert_eq!(percent(199, Some(200)), Some(99));
        assert_eq!(percent(250, Some(200)), Some(100));
        assert_eq!(percent(10, None), None);
        assert_eq!(percent(10, Some(0)), None);
    }

    #[test]
    fn only_a_download_in_progress_becomes_ready() {
        assert_eq!(
            changed(table(|phase| phase.downloaded(()))),
            [(downloading(Some(40)), ready())]
        );
    }

    #[test]
    fn install_takes_the_download_exactly_once() {
        let mut phase = Phase::Ready("0.2.0".into(), ());
        assert_eq!(phase.begin_install(), Some(()));
        assert_eq!(phase.status(), installing());
        assert_eq!(phase.begin_install(), None);
        assert_eq!(phase.status(), installing());
        for mut phase in every() {
            let before = phase.status();
            if before != ready() {
                assert_eq!(phase.begin_install(), None, "{before:?}");
                assert_eq!(phase.status(), before);
            }
        }
    }

    #[test]
    fn only_work_in_progress_can_fail() {
        assert_eq!(
            changed(table(|phase| phase.fail("offline".into())))
                .into_iter()
                .map(|(from, _)| from)
                .collect::<Vec<_>>(),
            [UpdateStatus::Checking, downloading(Some(40)), installing()]
        );
        let mut phase = Phase::Ready("0.2.0".into(), ());
        phase.fail("late".into());
        assert_eq!(phase.status(), ready());
        let mut phase = Phase::<()>::Downloading("0.2.0".into(), None);
        phase.fail("offline".into());
        assert_eq!(phase.status(), failed("offline"));
    }

    #[test]
    fn checks_are_due_daily_or_hourly_after_a_failure() {
        let last = SystemTime::UNIX_EPOCH + INTERVAL;
        let hours = |hours: u64| last + Duration::from_secs(hours * 60 * 60);
        assert!(due(None, false, last));
        assert!(!due(Some(last), false, hours(23)));
        assert!(due(Some(last), false, hours(24)));
        assert!(!due(Some(last), true, last + Duration::from_secs(59 * 60)));
        assert!(due(Some(last), true, hours(1)));
        assert!(due(Some(last), false, last - Duration::from_secs(1)));
    }

    #[test]
    fn status_serializes_as_the_settings_contract() {
        let json = |status: UpdateStatus| serde_json::to_string(&status).unwrap();
        assert_eq!(json(ready()), r#"{"state":"ready","version":"0.2.0"}"#);
        assert_eq!(json(UpdateStatus::Idle), r#"{"state":"idle"}"#);
        assert_eq!(
            json(downloading(Some(40))),
            r#"{"state":"downloading","version":"0.2.0","percent":40}"#
        );
        assert_eq!(
            json(downloading(None)),
            r#"{"state":"downloading","version":"0.2.0","percent":null}"#
        );
        assert_eq!(
            json(failed("offline")),
            r#"{"state":"failed","message":"offline"}"#
        );
    }
}
