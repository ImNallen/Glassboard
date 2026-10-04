use plist::{Dictionary, Value};
use std::{
    error::Error,
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

/// The LaunchAgent that opens Glassboard at login. Enabled means its file exists.
pub(crate) struct LoginItem {
    pub agent: PathBuf,
    /// The agent auto-launch wrote under the package name, parented to the developer.
    pub legacy: PathBuf,
    pub identifier: String,
    pub executable: PathBuf,
}

impl LoginItem {
    pub(crate) fn is_enabled(&self) -> bool {
        self.agent.exists()
    }

    pub(crate) fn set_enabled(&self, enabled: bool) -> Result<(), Box<dyn Error>> {
        if !enabled {
            return remove(&self.agent);
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

    /// Replaces the legacy agent with the identified one. Writes first so a crash
    /// between the two steps converges on the next launch.
    pub(crate) fn migrate(&self) -> Result<(), Box<dyn Error>> {
        if !self.legacy.exists() {
            return Ok(());
        }
        self.set_enabled(true)?;
        remove(&self.legacy)
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

fn remove(path: &Path) -> Result<(), Box<dyn Error>> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(directory: &Path) -> LoginItem {
        let agents = directory.join("Library/LaunchAgents");
        LoginItem {
            agent: agents.join("dev.glassboard.desktop.plist"),
            legacy: agents.join("Glassboard.plist"),
            identifier: "dev.glassboard.desktop".into(),
            executable: "/Applications/Glassboard.app/Contents/MacOS/glassboard".into(),
        }
    }

    fn expected() -> Value {
        Value::from_reader(std::io::Cursor::new(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
    <key>Label</key><string>dev.glassboard.desktop</string>
    <key>ProgramArguments</key><array>
        <string>/Applications/Glassboard.app/Contents/MacOS/glassboard</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>AssociatedBundleIdentifiers</key><array>
        <string>dev.glassboard.desktop</string>
    </array>
</dict></plist>"#,
        ))
        .unwrap()
    }

    #[test]
    fn enabling_writes_the_identified_agent() {
        let directory = tempfile::tempdir().unwrap();
        let item = item(directory.path());
        assert!(!item.is_enabled());

        item.set_enabled(true).unwrap();

        assert!(item.is_enabled());
        assert_eq!(Value::from_file(&item.agent).unwrap(), expected());
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
    fn migrating_replaces_the_legacy_agent() {
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

        item.migrate().unwrap();

        assert!(!item.legacy.exists());
        assert!(item.is_enabled());
        assert_eq!(Value::from_file(&item.agent).unwrap(), expected());
    }

    #[test]
    fn migrating_without_a_legacy_agent_creates_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let item = item(directory.path());

        item.migrate().unwrap();

        assert!(!item.is_enabled());
        assert!(!directory.path().join("Library").exists());
    }

    #[test]
    fn migrating_twice_converges() {
        let directory = tempfile::tempdir().unwrap();
        let item = item(directory.path());
        fs::create_dir_all(item.legacy.parent().unwrap()).unwrap();
        fs::write(&item.legacy, b"<plist version=\"1.0\"><dict/></plist>").unwrap();

        item.migrate().unwrap();
        let first = fs::read(&item.agent).unwrap();
        item.migrate().unwrap();

        assert!(!item.legacy.exists());
        assert_eq!(fs::read(&item.agent).unwrap(), first);
        let agents = item.agent.parent().unwrap();
        assert_eq!(fs::read_dir(agents).unwrap().count(), 1);
    }
}
