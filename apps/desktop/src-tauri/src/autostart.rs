use plist::Value;
use std::{
    error::Error,
    fs::File,
    io::{ErrorKind, Write},
    path::Path,
};

/// Associates an existing legacy LaunchAgent with its app without enabling it.
pub(crate) fn associate_plist(path: &Path, bundle_identifier: &str) -> Result<(), Box<dyn Error>> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let mut agent = Value::from_reader(&mut file)?;
    let dictionary = agent
        .as_dictionary_mut()
        .ok_or("The login item must contain a plist dictionary")?;
    let mut identifiers = match dictionary.get("AssociatedBundleIdentifiers") {
        None => Vec::new(),
        Some(Value::String(identifier)) => vec![Value::String(identifier.clone())],
        Some(Value::Array(identifiers))
            if identifiers.iter().all(|value| value.as_string().is_some()) =>
        {
            identifiers.clone()
        }
        Some(_) => return Err("The login item's bundle associations must be strings".into()),
    };
    if identifiers
        .iter()
        .any(|identifier| identifier.as_string() == Some(bundle_identifier))
    {
        return Ok(());
    }
    identifiers.push(Value::String(bundle_identifier.into()));
    dictionary.insert(
        "AssociatedBundleIdentifiers".into(),
        Value::Array(identifiers),
    );

    let mut bytes = Vec::new();
    agent.to_writer_xml(&mut bytes)?;
    let mut replacement = tempfile::NamedTempFile::new_in(
        path.parent()
            .ok_or("The login item has no parent directory")?,
    )?;
    replacement
        .as_file()
        .set_permissions(file.metadata()?.permissions())?;
    replacement.write_all(&bytes)?;
    replacement.as_file().sync_all()?;
    replacement.persist(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};

    const BUNDLE_ID: &str = "dev.glassboard.desktop";
    const LEGACY_AGENT: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
    <key>Label</key><string>Glassboard</string>
    <key>ProgramArguments</key><array>
        <string>/Applications/Glassboard &amp; Notes.app/Contents/MacOS/glassboard</string>
        <string>--value=&lt;hello&gt;</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>Disabled</key><true/>
    <key>EnvironmentVariables</key><dict><key>CUSTOM</key><string>keep me</string></dict>
</dict></plist>"#;

    fn legacy_agent() -> Value {
        Value::from_reader(std::io::Cursor::new(LEGACY_AGENT)).unwrap()
    }

    #[test]
    fn migration_preserves_existing_job_settings_and_permissions() {
        for binary in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("Glassboard.plist");
            let mut expected = legacy_agent();
            if binary {
                expected.to_file_binary(&path).unwrap();
            } else {
                fs::write(&path, LEGACY_AGENT).unwrap();
            }
            fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();

            associate_plist(&path, BUNDLE_ID).unwrap();

            expected.as_dictionary_mut().unwrap().insert(
                "AssociatedBundleIdentifiers".into(),
                Value::Array(vec![Value::String(BUNDLE_ID.into())]),
            );
            assert_eq!(Value::from_file(&path).unwrap(), expected);
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o640
            );
        }
    }

    #[test]
    fn absent_login_item_stays_disabled_without_creating_directories() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("LaunchAgents/Glassboard.plist");
        associate_plist(&path, BUNDLE_ID).unwrap();
        assert!(!path.parent().unwrap().exists());
    }

    #[test]
    fn already_associated_jobs_are_not_rewritten() {
        use std::os::unix::fs::MetadataExt;

        for association in [
            Value::String(BUNDLE_ID.into()),
            Value::Array(vec![
                Value::String("dev.other.app".into()),
                Value::String(BUNDLE_ID.into()),
            ]),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("Glassboard.plist");
            let mut agent = legacy_agent();
            agent
                .as_dictionary_mut()
                .unwrap()
                .insert("AssociatedBundleIdentifiers".into(), association);
            agent.to_file_binary(&path).unwrap();
            let original = fs::read(&path).unwrap();
            let inode = fs::metadata(&path).unwrap().ino();

            associate_plist(&path, BUNDLE_ID).unwrap();
            associate_plist(&path, BUNDLE_ID).unwrap();

            assert_eq!(fs::read(&path).unwrap(), original);
            assert_eq!(fs::metadata(&path).unwrap().ino(), inode);
        }
    }

    #[test]
    fn migration_keeps_other_valid_bundle_associations() {
        for association in [
            Value::String("dev.other.app".into()),
            Value::Array(vec![Value::String("dev.other.app".into())]),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("Glassboard.plist");
            let mut expected = legacy_agent();
            expected
                .as_dictionary_mut()
                .unwrap()
                .insert("AssociatedBundleIdentifiers".into(), association);
            expected.to_file_xml(&path).unwrap();

            associate_plist(&path, BUNDLE_ID).unwrap();

            expected.as_dictionary_mut().unwrap().insert(
                "AssociatedBundleIdentifiers".into(),
                Value::Array(vec![
                    Value::String("dev.other.app".into()),
                    Value::String(BUNDLE_ID.into()),
                ]),
            );
            assert_eq!(Value::from_file(&path).unwrap(), expected);
        }
    }

    #[test]
    fn malformed_jobs_are_reported_without_modifying_them() {
        let mut invalid_association = legacy_agent();
        invalid_association.as_dictionary_mut().unwrap().insert(
            "AssociatedBundleIdentifiers".into(),
            Value::Array(vec![Value::Integer(7.into())]),
        );
        let mut invalid_bytes = Vec::new();
        invalid_association
            .to_writer_xml(&mut invalid_bytes)
            .unwrap();
        for bytes in [
            b"<plist><dict>broken".to_vec(),
            b"<plist version=\"1.0\"><array/></plist>".to_vec(),
            invalid_bytes,
        ] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("Glassboard.plist");
            fs::write(&path, &bytes).unwrap();

            assert!(associate_plist(&path, BUNDLE_ID).is_err());

            assert_eq!(fs::read(&path).unwrap(), bytes);
            assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        }
    }
}
