use objc2::runtime::AnyClass;
use objc2_foundation::{NSString, NSURL};
use objc2_service_management::{SMAppService, SMAppServiceStatus};
use plist::{Dictionary, Value};
use std::{
    error::Error,
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

/// The LaunchAgent that opens Glassboard at login. Enabled means its file names
/// an executable that still exists and macOS has not switched it off.
pub(crate) struct LoginItem {
    pub agent: PathBuf,
    /// The agent auto-launch wrote under the package name, parented to the developer.
    pub legacy: PathBuf,
    pub identifier: String,
    pub executable: PathBuf,
}

impl LoginItem {
    pub(crate) fn is_enabled(&self) -> bool {
        recorded_program(&self.agent).is_some_and(|program| program.exists())
            && approved(&self.agent)
    }

    pub(crate) fn set_enabled(&self, enabled: bool) -> Result<(), Box<dyn Error>> {
        if !enabled {
            return remove(&self.agent);
        }
        if is_transient(&self.executable) {
            return Err(
                "Move Glassboard to the Applications folder and open it from there first".into(),
            );
        }
        let directory = self
            .agent
            .parent()
            .ok_or("The login item has no parent directory")?;
        fs::create_dir_all(directory)?;
        let mut bytes = Vec::new();
        Value::Dictionary(self.plist()).to_writer_xml(&mut bytes)?;
        // Background Task Management reads the developer attribution only when
        // an agent first appears, so the finished file must arrive in one rename.
        let mut file = tempfile::NamedTempFile::new_in(directory)?;
        file.write_all(&bytes)?;
        file.as_file().sync_all()?;
        file.persist(&self.agent)?;
        Ok(())
    }

    /// Converges the agent at launch: replaces the legacy agent and rewrites one
    /// whose executable has gone away. Writes first so a crash between the two
    /// steps converges on the next launch. A transient copy leaves the agent
    /// alone so the installed copy keeps it.
    pub(crate) fn repair(&self) -> Result<(), Box<dyn Error>> {
        if is_transient(&self.executable) {
            return Ok(());
        }
        if self.legacy.exists() {
            self.set_enabled(true)?;
            return remove(&self.legacy);
        }
        if recorded_program(&self.agent).is_some_and(|program| !program.exists()) {
            self.set_enabled(true)?;
        }
        Ok(())
    }

    fn plist(&self) -> Dictionary {
        let string = |value: &str| Value::String(value.into());
        let mut plist = Dictionary::new();
        plist.insert("Label".into(), string(&self.identifier));
        plist.insert(
            "ProgramArguments".into(),
            Value::Array(vec![string(&self.executable.to_string_lossy())]),
        );
        plist.insert("RunAtLoad".into(), Value::Boolean(true));
        plist.insert(
            "AssociatedBundleIdentifiers".into(),
            Value::Array(vec![string(&self.identifier)]),
        );
        plist
    }
}

/// Whether the user switched the agent off under System Settings > General >
/// Login Items & Extensions. macOS remembers that choice across removing and
/// rewriting the file, and only macOS 13 and later ask.
pub(crate) fn approved(agent: &Path) -> bool {
    if AnyClass::get(c"SMAppService").is_none() {
        return true;
    }
    let url = NSURL::fileURLWithPath(&NSString::from_str(&agent.to_string_lossy()));
    let status = unsafe { SMAppService::statusForLegacyURL(&url) };
    status != SMAppServiceStatus::RequiresApproval
}

/// A disk image unmounts and Gatekeeper's translocation mount for a quarantined
/// copy vanishes when it quits, so neither path exists at the next login.
fn is_transient(executable: &Path) -> bool {
    executable.starts_with("/Volumes")
        || executable
            .iter()
            .any(|component| component == "AppTranslocation")
}

fn recorded_program(agent: &Path) -> Option<PathBuf> {
    let agent = Value::from_file(agent).ok()?;
    let arguments = agent.as_dictionary()?.get("ProgramArguments")?.as_array()?;
    Some(arguments.first()?.as_string()?.into())
}

fn remove(path: &Path) -> Result<(), Box<dyn Error>> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRANSLOCATED: &str = "/private/var/folders/rf/kpqj3jpn7qvf82sv18g4ppwr0000gn/T/AppTranslocation/387095D9-9F57-4774-BB63-DF30E56892A5/d/Glassboard.app/Contents/MacOS/glassboard";
    const MOUNTED: &str = "/Volumes/Glassboard/Glassboard.app/Contents/MacOS/glassboard";

    fn executable(directory: &Path, relative: &str) -> PathBuf {
        let path = directory.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"").unwrap();
        path
    }

    fn item(directory: &Path) -> LoginItem {
        let agents = directory.join("Library/LaunchAgents");
        LoginItem {
            agent: agents.join("dev.glassboard.desktop.plist"),
            legacy: agents.join("Glassboard.plist"),
            identifier: "dev.glassboard.desktop".into(),
            executable: executable(
                directory,
                "Applications/Glassboard.app/Contents/MacOS/glassboard",
            ),
        }
    }

    fn agent(program: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
    <key>Label</key><string>dev.glassboard.desktop</string>
    <key>ProgramArguments</key><array>
        <string>{program}</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>AssociatedBundleIdentifiers</key><array>
        <string>dev.glassboard.desktop</string>
    </array>
</dict></plist>"#
        )
    }

    fn expected(item: &LoginItem) -> Value {
        Value::from_reader(std::io::Cursor::new(agent(
            item.executable.to_str().unwrap(),
        )))
        .unwrap()
    }

    fn write_agent(item: &LoginItem, program: &str) {
        fs::create_dir_all(item.agent.parent().unwrap()).unwrap();
        fs::write(&item.agent, agent(program)).unwrap();
    }

    #[test]
    fn enabling_writes_the_identified_agent() {
        let directory = tempfile::tempdir().unwrap();
        let item = item(directory.path());
        assert!(!item.is_enabled());

        item.set_enabled(true).unwrap();

        assert!(item.is_enabled());
        assert_eq!(Value::from_file(&item.agent).unwrap(), expected(&item));
        let agents = item.agent.parent().unwrap();
        assert_eq!(fs::read_dir(agents).unwrap().count(), 1);
    }

    #[test]
    fn disabling_removes_the_agent_and_tolerates_absence() {
        let directory = tempfile::tempdir().unwrap();
        let item = item(directory.path());
        item.set_enabled(true).unwrap();

        item.set_enabled(false).unwrap();
        assert!(!item.is_enabled());
        assert!(!item.agent.exists());

        item.set_enabled(false).unwrap();
        assert!(!item.is_enabled());
    }

    #[test]
    fn enabling_from_a_transient_copy_is_refused() {
        for transient in [MOUNTED, TRANSLOCATED] {
            let directory = tempfile::tempdir().unwrap();
            let mut item = item(directory.path());
            item.executable = transient.into();

            let error = item.set_enabled(true).unwrap_err().to_string();

            assert_eq!(
                error,
                "Move Glassboard to the Applications folder and open it from there first"
            );
            assert!(!item.agent.exists());
            assert!(!item.is_enabled());
        }
    }

    #[test]
    fn an_agent_whose_executable_is_gone_reads_as_disabled() {
        let directory = tempfile::tempdir().unwrap();
        let item = item(directory.path());
        write_agent(&item, TRANSLOCATED);

        assert!(!item.is_enabled());

        write_agent(&item, item.executable.to_str().unwrap());
        assert!(item.is_enabled());
    }

    #[test]
    fn classifies_transient_executables() {
        for (path, transient) in [
            (
                "/Applications/Glassboard.app/Contents/MacOS/glassboard",
                false,
            ),
            (
                "/Users/me/Applications/Glassboard.app/Contents/MacOS/glassboard",
                false,
            ),
            (
                "/Users/me/Volumes/Glassboard.app/Contents/MacOS/glassboard",
                false,
            ),
            (
                "/VolumesBackup/Glassboard.app/Contents/MacOS/glassboard",
                false,
            ),
            (MOUNTED, true),
            (TRANSLOCATED, true),
        ] {
            assert_eq!(is_transient(Path::new(path)), transient, "{path}");
        }
    }

    #[test]
    fn reads_the_recorded_program() {
        let directory = tempfile::tempdir().unwrap();
        let item = item(directory.path());
        assert_eq!(recorded_program(&item.agent), None);

        write_agent(&item, MOUNTED);
        assert_eq!(recorded_program(&item.agent), Some(PathBuf::from(MOUNTED)));

        fs::write(&item.agent, b"<plist version=\"1.0\"><dict/></plist>").unwrap();
        assert_eq!(recorded_program(&item.agent), None);
    }

    #[test]
    fn repairing_replaces_the_legacy_agent() {
        let directory = tempfile::tempdir().unwrap();
        let item = item(directory.path());
        fs::create_dir_all(item.legacy.parent().unwrap()).unwrap();
        fs::write(
            &item.legacy,
            br#"<plist version="1.0"><dict>
    <key>Label</key><string>Glassboard</string>
    <key>ProgramArguments</key><array><string>/old/glassboard</string></array>
    <key>RunAtLoad</key><true/>
</dict></plist>"#,
        )
        .unwrap();

        item.repair().unwrap();

        assert!(!item.legacy.exists());
        assert!(item.is_enabled());
        assert_eq!(Value::from_file(&item.agent).unwrap(), expected(&item));
    }

    #[test]
    fn repairing_without_an_agent_creates_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let item = item(directory.path());

        item.repair().unwrap();

        assert!(!item.is_enabled());
        assert!(!directory.path().join("Library").exists());
    }

    #[test]
    fn repairing_rewrites_an_agent_whose_executable_is_gone() {
        let directory = tempfile::tempdir().unwrap();
        let item = item(directory.path());
        write_agent(&item, TRANSLOCATED);

        item.repair().unwrap();

        assert!(item.is_enabled());
        assert_eq!(Value::from_file(&item.agent).unwrap(), expected(&item));
    }

    #[test]
    fn repairing_keeps_an_agent_whose_executable_exists() {
        let directory = tempfile::tempdir().unwrap();
        let item = item(directory.path());
        let installed = executable(directory.path(), "elsewhere/glassboard");
        write_agent(&item, installed.to_str().unwrap());
        let before = fs::read(&item.agent).unwrap();

        item.repair().unwrap();

        assert_eq!(fs::read(&item.agent).unwrap(), before);
        assert_eq!(recorded_program(&item.agent), Some(installed));
    }

    #[test]
    fn repairing_from_a_transient_copy_touches_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let mut item = item(directory.path());
        item.executable = MOUNTED.into();
        fs::create_dir_all(item.legacy.parent().unwrap()).unwrap();
        fs::write(&item.legacy, b"<plist version=\"1.0\"><dict/></plist>").unwrap();
        write_agent(&item, TRANSLOCATED);
        let before = fs::read(&item.agent).unwrap();

        item.repair().unwrap();

        assert!(item.legacy.exists());
        assert_eq!(fs::read(&item.agent).unwrap(), before);
    }

    #[test]
    fn repairing_twice_converges() {
        let directory = tempfile::tempdir().unwrap();
        let item = item(directory.path());
        fs::create_dir_all(item.legacy.parent().unwrap()).unwrap();
        fs::write(&item.legacy, b"<plist version=\"1.0\"><dict/></plist>").unwrap();

        item.repair().unwrap();
        let first = fs::read(&item.agent).unwrap();
        item.repair().unwrap();

        assert!(!item.legacy.exists());
        assert_eq!(fs::read(&item.agent).unwrap(), first);
        let agents = item.agent.parent().unwrap();
        assert_eq!(fs::read_dir(agents).unwrap().count(), 1);
    }
}
