use crate::{toolbar_position::ToolbarPosition, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{collections::BTreeMap, io::Write, path::Path};
use tauri::Manager;
use tauri_plugin_global_shortcut::{Modifiers, Shortcut};

pub(crate) const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+A";
const FILE_NAME: &str = "preferences.json";
const BACKUP_NAME: &str = "preferences.json.bak";
/// Saved files carry this so upgrades apply only to files written before them.
const FORMAT_VERSION: u64 = 1;

#[derive(Clone, Copy, Default, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ColorMode {
    Solid,
    #[default]
    Rainbow,
    Cycle,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Preferences {
    pub(crate) tool: String,
    pub(crate) color: String,
    #[serde(default)]
    pub(crate) color_mode: ColorMode,
    pub(crate) shortcut: String,
    /// In-app binding overrides by command id; an empty shortcut unbinds the command.
    #[serde(default)]
    pub(crate) keybindings: BTreeMap<String, String>,
    /// The toolbar's solid color swatches, in toolbar order.
    #[serde(default = "default_swatches")]
    pub(crate) swatches: Vec<String>,
    /// Rainbow's gradient colors, in order.
    #[serde(default = "default_sequence")]
    pub(crate) rainbow_colors: Vec<String>,
    /// The colors Shifting steps through, one per shape.
    #[serde(default = "default_sequence")]
    pub(crate) cycle_colors: Vec<String>,
    #[serde(default)]
    pub(crate) toolbar_position: ToolbarPosition,
    #[serde(default)]
    pub(crate) auto_fade_seconds: u8,
    #[serde(default)]
    pub(crate) tutorial_completed: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            tool: "arrow".into(),
            color: "#f46b78".into(),
            color_mode: ColorMode::Rainbow,
            shortcut: DEFAULT_SHORTCUT.into(),
            keybindings: BTreeMap::new(),
            swatches: default_swatches(),
            rainbow_colors: default_sequence(),
            cycle_colors: default_sequence(),
            toolbar_position: ToolbarPosition::Bottom,
            auto_fade_seconds: 0,
            tutorial_completed: false,
        }
    }
}
fn default_swatches() -> Vec<String> {
    [
        "#000000", "#ffffff", "#4dcaa0", "#f2c85b", "#f46b78", "#669df0",
    ]
    .map(String::from)
    .to_vec()
}
fn default_sequence() -> Vec<String> {
    [
        "#f46b78", "#f2c85b", "#4dcaa0", "#4fc5d5", "#669df0", "#a184e8", "#e580b5",
    ]
    .map(String::from)
    .to_vec()
}
fn valid_sequence(colors: &[String]) -> bool {
    (2..=8).contains(&colors.len()) && colors.iter().all(|color| is_hex_color(color))
}
fn is_hex_color(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
}
// Palette colors from before saved files had a version, mapped to their replacements.
fn upgraded_color(color: &str) -> &str {
    match color {
        "#efa5a5" | "#ff3355" => "#f46b78",
        "#efb895" | "#ff8a1f" | "#e8cf91" | "#f5ff00" => "#f2c85b",
        "#c6d99c" | "#a3ff12" | "#9fd5b5" | "#00f58a" => "#4dcaa0",
        "#98d3cf" | "#00f0ff" => "#4fc5d5",
        "#9fc5e8" | "#3388ff" => "#669df0",
        "#afb5e5" | "#7855ff" | "#c9ace0" | "#c43cff" => "#a184e8",
        "#e1a9cf" | "#ff33cc" => "#e580b5",
        _ => color,
    }
}
impl Preferences {
    /// Loads saved preferences, with a message for the user when any had to be reset.
    pub(crate) fn load(app: &tauri::AppHandle) -> (Self, Option<String>) {
        match app.path().app_config_dir() {
            Ok(dir) => load_from(&dir),
            Err(error) => {
                log::error!("Could not find the preferences folder: {error}");
                (Self::default(), None)
            }
        }
    }

    pub(crate) fn save(&self, app: &tauri::AppHandle) -> Result<()> {
        let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        self.save_to(&dir)
    }

    fn save_to(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let mut saved = serde_json::to_value(self).map_err(|e| e.to_string())?;
        saved["version"] = FORMAT_VERSION.into();
        let bytes = serde_json::to_vec_pretty(&saved).map_err(|e| e.to_string())?;
        write_atomically(&dir.join(FILE_NAME), &bytes).map_err(|e| e.to_string())
    }

    pub(crate) fn validate(&self) -> Result<Shortcut> {
        if ![
            "pen",
            "arrow",
            "rectangle",
            "ellipse",
            "highlighter",
            "text",
            "eraser",
        ]
        .contains(&self.tool.as_str())
            || ![0, 3, 5, 10].contains(&self.auto_fade_seconds)
            || !is_hex_color(&self.color)
            || self.swatches.len() != 6
            || !self.swatches.iter().all(|color| is_hex_color(color))
            || !valid_sequence(&self.rainbow_colors)
            || !valid_sequence(&self.cycle_colors)
        {
            return Err("Invalid drawing preferences".into());
        }
        if self.keybindings.len() > 64
            || self.keybindings.iter().any(|(command, shortcut)| {
                command.is_empty()
                    || command.len() > 32
                    || !command
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                    || (!shortcut.is_empty() && shortcut.parse::<Shortcut>().is_err())
            })
        {
            return Err("Invalid keybindings".into());
        }
        let shortcut: Shortcut = self
            .shortcut
            .parse()
            .map_err(|e| format!("Invalid shortcut: {e}"))?;
        if !shortcut
            .mods
            .intersects(Modifiers::CONTROL | Modifiers::SUPER | Modifiers::ALT)
        {
            return Err("Include CommandOrControl, Control, Super, or Alt in the shortcut.".into());
        }
        Ok(shortcut)
    }
}

fn load_from(dir: &Path) -> (Preferences, Option<String>) {
    const SOME_RESET: &str = "Some settings could not be read and were reset to their defaults.";
    const ALL_RESET: &str = "Your settings could not be read, so Glassboard is using the defaults.";
    let path = dir.join(FILE_NAME);
    let backup = dir.join(BACKUP_NAME);
    let (preferences, problem, backed_up) = match std::fs::read(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (Preferences::default(), None)
        }
        Err(error) => {
            log::error!("Could not read {}: {error}", path.display());
            // Move the unreadable file aside so saving the defaults cannot replace it.
            (
                Preferences::default(),
                ALL_RESET,
                std::fs::rename(&path, &backup),
            )
        }
        Ok(bytes) => {
            let (preferences, problem) = match serde_json::from_slice::<Value>(&bytes) {
                Ok(Value::Object(mut saved)) => {
                    upgrade(&mut saved);
                    if let Some(preferences) = parse(&saved) {
                        return (preferences, None);
                    }
                    let (preferences, reset) = recover(saved);
                    log::warn!("Reset unreadable preferences: {reset:?}");
                    (preferences, SOME_RESET)
                }
                _ => {
                    log::warn!(
                        "Reset all preferences: {} is not a JSON object",
                        path.display()
                    );
                    (Preferences::default(), ALL_RESET)
                }
            };
            (preferences, problem, write_atomically(&backup, &bytes))
        }
    };
    if let Err(error) = backed_up {
        log::error!("Could not back up {}: {error}", path.display());
        return (preferences, Some(problem.into()));
    }
    // Replace the damaged file with what was recovered, so the next launch starts clean.
    if let Err(error) = preferences.save_to(dir) {
        log::error!("Could not save recovered preferences: {error}");
    }
    (
        preferences,
        Some(format!(
            "{problem} The original file was kept as {BACKUP_NAME}."
        )),
    )
}

fn upgrade(saved: &mut Map<String, Value>) {
    let version = saved.get("version").and_then(Value::as_u64).unwrap_or(0);
    if version < 1 {
        if let Some(Value::String(color)) = saved.get_mut("color") {
            *color = upgraded_color(color).into();
        }
    }
}

/// Rebuilds preferences one saved field at a time, so a single unreadable
/// field (from a damaged file or a newer version) resets only that field.
/// Returns the fields that were reset.
fn recover(saved: Map<String, Value>) -> (Preferences, Vec<String>) {
    let Ok(Value::Object(mut kept)) = serde_json::to_value(Preferences::default()) else {
        return (
            Preferences::default(),
            saved.into_iter().map(|(field, _)| field).collect(),
        );
    };
    let mut reset = Vec::new();
    for (field, value) in saved {
        let previous = kept.insert(field.clone(), value);
        if parse(&kept).is_none() {
            match previous {
                Some(previous) => kept.insert(field.clone(), previous),
                None => kept.remove(&field),
            };
            reset.push(field);
        }
    }
    (parse(&kept).unwrap_or_default(), reset)
}

fn parse(fields: &Map<String, Value>) -> Option<Preferences> {
    serde_json::from_value::<Preferences>(Value::Object(fields.clone()))
        .ok()
        .filter(|preferences| preferences.validate().is_ok())
}

/// Writes beside the destination and renames into place, so a crash or power
/// loss mid-save leaves the previous file intact instead of a truncated one.
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension("tmp");
    let written = std::fs::File::create(&temporary).and_then(|mut file| {
        file.write_all(bytes)?;
        file.sync_all()
    });
    let result = written.and_then(|()| std::fs::rename(&temporary, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shortcut_validation_rejects_unmodified_and_malformed_keys() {
        let mut p = Preferences::default();
        assert!(p.validate().is_ok());
        for shortcut in ["A", "Shift+A", "not a shortcut", "CommandOrControl+Shift+"] {
            p.shortcut = shortcut.into();
            assert!(p.validate().is_err(), "{shortcut}");
        }
        // Recorded shortcuts use browser key codes, which the native parser accepts.
        for shortcut in [
            "CommandOrControl+Shift+KeyA",
            "Alt+Digit1",
            "Control+Space",
            "CommandOrControl+F5",
            "Alt+Shift+ArrowUp",
            "Super+Comma",
            "CommandOrControl+Shift+S",
        ] {
            p.shortcut = shortcut.into();
            assert!(p.validate().is_ok(), "{shortcut}");
        }
    }
    #[test]
    fn swatches_are_saved_and_validated() {
        let preferences = Preferences {
            swatches: [
                "#123abc", "#FFFFFF", "#000000", "#8b5cf6", "#8b5cf6", "#00c7be",
            ]
            .map(String::from)
            .to_vec(),
            ..Preferences::default()
        };
        assert!(preferences.validate().is_ok());
        let restored: Preferences =
            serde_json::from_slice(&serde_json::to_vec(&preferences).unwrap()).unwrap();
        assert_eq!(restored.swatches, preferences.swatches);
        for swatches in [
            vec!["#000000"; 5],
            vec!["#000000"; 7],
            vec![
                "#000000", "#ffffff", "#4dcaa0", "#f2c85b", "#f46b78", "#fff",
            ],
            vec![
                "#000000", "#ffffff", "#4dcaa0", "#f2c85b", "#f46b78", "#gggggg",
            ],
        ] {
            let invalid = Preferences {
                swatches: swatches.into_iter().map(String::from).collect(),
                ..Preferences::default()
            };
            assert!(invalid.validate().is_err(), "{:?}", invalid.swatches);
        }
    }
    #[test]
    fn rainbow_and_shifting_colors_are_saved_and_validated() {
        let preferences = Preferences {
            rainbow_colors: vec!["#000000".into(), "#FFFFFF".into()],
            cycle_colors: vec!["#123abc".into(); 8],
            ..Preferences::default()
        };
        assert!(preferences.validate().is_ok());
        let restored: Preferences =
            serde_json::from_slice(&serde_json::to_vec(&preferences).unwrap()).unwrap();
        assert_eq!(restored.rainbow_colors, preferences.rainbow_colors);
        assert_eq!(restored.cycle_colors, preferences.cycle_colors);
        let json = serde_json::to_value(&preferences).unwrap();
        assert!(json.get("rainbowColors").is_some() && json.get("cycleColors").is_some());
        for colors in [
            vec![],
            vec!["#000000"],
            vec!["#000000"; 9],
            vec!["#000000", "red"],
        ] {
            let colors: Vec<String> = colors.into_iter().map(String::from).collect();
            for invalid in [
                Preferences {
                    rainbow_colors: colors.clone(),
                    ..Preferences::default()
                },
                Preferences {
                    cycle_colors: colors.clone(),
                    ..Preferences::default()
                },
            ] {
                assert!(invalid.validate().is_err(), "{colors:?}");
            }
        }
    }
    #[test]
    fn keybindings_are_saved_and_validated() {
        let mut preferences = Preferences::default();
        for (command, shortcut) in [
            ("undo", "CommandOrControl+KeyU"),
            ("color-red", "Digit9"),
            ("hide", "Escape"),
            ("tool-pen", "Alt+Shift+KeyP"),
            ("copy", ""),
        ] {
            preferences
                .keybindings
                .insert(command.into(), shortcut.into());
        }
        assert!(preferences.validate().is_ok());
        let restored: Preferences =
            serde_json::from_slice(&serde_json::to_vec(&preferences).unwrap()).unwrap();
        assert_eq!(restored.keybindings, preferences.keybindings);
        for (command, shortcut) in [
            ("undo", "not a shortcut"),
            ("undo", "CommandOrControl+"),
            ("", "KeyA"),
            ("Undo", "KeyA"),
            ("tool_pen", "KeyA"),
        ] {
            let mut invalid = Preferences::default();
            invalid.keybindings.insert(command.into(), shortcut.into());
            assert!(invalid.validate().is_err(), "{command}: {shortcut}");
        }
    }
    #[test]
    fn auto_fade_preferences_are_saved_and_validated() {
        for seconds in [0, 3, 5, 10] {
            let preferences = Preferences {
                auto_fade_seconds: seconds,
                ..Preferences::default()
            };
            assert!(preferences.validate().is_ok());
            let saved = serde_json::to_string(&preferences).unwrap();
            let restored: Preferences = serde_json::from_str(&saved).unwrap();
            assert_eq!(restored.auto_fade_seconds, seconds);
        }
        let invalid = Preferences {
            auto_fade_seconds: 4,
            ..Preferences::default()
        };
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn every_tool_is_accepted_and_unknown_tools_are_rejected() {
        for tool in [
            "pen",
            "arrow",
            "rectangle",
            "ellipse",
            "highlighter",
            "text",
            "eraser",
        ] {
            let preferences = Preferences {
                tool: tool.into(),
                ..Preferences::default()
            };
            assert!(preferences.validate().is_ok(), "{tool}");
        }
        let invalid = Preferences {
            tool: "pencil".into(),
            ..Preferences::default()
        };
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn older_preferences_default_to_rainbow_and_keep_color_and_shortcut() {
        let preferences: Preferences = serde_json::from_str(
            r##"{"tool":"pen","color":"#68aaff","width":7,"shortcut":"CommandOrControl+Shift+B"}"##,
        )
        .unwrap();
        assert_eq!(preferences.color_mode, ColorMode::Rainbow);
        assert_eq!(preferences.toolbar_position, ToolbarPosition::Bottom);
        assert_eq!(preferences.tool, "pen");
        assert_eq!(preferences.auto_fade_seconds, 0);
        assert_eq!(preferences.color, "#68aaff");
        assert_eq!(preferences.shortcut, "CommandOrControl+Shift+B");
        assert!(preferences.keybindings.is_empty());
        assert_eq!(preferences.swatches, default_swatches());
        assert_eq!(preferences.rainbow_colors, default_sequence());
        assert_eq!(preferences.cycle_colors, default_sequence());
        // Retired line-width preferences are ignored without resetting other settings.
        assert!(serde_json::to_value(&preferences)
            .unwrap()
            .get("width")
            .is_none());
        assert!(preferences.validate().is_ok());
    }
    #[test]
    fn custom_colors_are_validated_and_preserved() {
        for color in ["#000000", "#ffffff", "#123aBC", "#ff6259"] {
            let preferences = Preferences {
                color: color.into(),
                ..Preferences::default()
            };
            assert!(preferences.validate().is_ok());
            let loaded: Preferences =
                serde_json::from_slice(&serde_json::to_vec(&preferences).unwrap()).unwrap();
            assert_eq!(loaded.color, color);
        }
        for color in ["", "#", "#fff", "#12345678", "1234567", "#gggggg", "#é1234"] {
            let preferences = Preferences {
                color: color.into(),
                ..Preferences::default()
            };
            assert!(preferences.validate().is_err(), "{color}");
        }
    }
    #[test]
    fn toolbar_positions_survive_saving_and_loading() {
        for position in [
            ToolbarPosition::Left,
            ToolbarPosition::Right,
            ToolbarPosition::Bottom,
        ] {
            let preferences = Preferences {
                toolbar_position: position,
                ..Preferences::default()
            };
            let loaded: Preferences =
                serde_json::from_slice(&serde_json::to_vec(&preferences).unwrap()).unwrap();
            assert_eq!(loaded.toolbar_position, position);
            assert!(loaded.validate().is_ok());
        }
    }
    fn scratch_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "glassboard-preferences-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }
    #[test]
    fn missing_preferences_load_defaults_without_a_warning() {
        let dir = scratch_dir("missing");
        let (preferences, warning) = load_from(&dir);
        assert!(preferences == Preferences::default());
        assert!(warning.is_none());
    }
    #[test]
    fn saved_preferences_replace_the_file_and_load_back() {
        let dir = scratch_dir("saved");
        Preferences::default().save_to(&dir).unwrap();
        let preferences = Preferences {
            tool: "pen".into(),
            ..Preferences::default()
        };
        preferences.save_to(&dir).unwrap();
        let (loaded, warning) = load_from(&dir);
        assert!(loaded == preferences);
        assert!(warning.is_none());
        let files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(files, [FILE_NAME]);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn damaged_preferences_are_backed_up_and_repaired_once() {
        let dir = scratch_dir("truncated");
        std::fs::create_dir_all(&dir).unwrap();
        let truncated = br##"{"tool":"pen","color":"#68a"##;
        std::fs::write(dir.join(FILE_NAME), truncated).unwrap();
        let (preferences, warning) = load_from(&dir);
        assert!(preferences == Preferences::default());
        assert!(warning.unwrap().contains(BACKUP_NAME));
        assert_eq!(std::fs::read(dir.join(BACKUP_NAME)).unwrap(), truncated);
        let (reloaded, warning) = load_from(&dir);
        assert!(reloaded == preferences);
        assert!(warning.is_none(), "the next launch starts clean");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn one_invalid_field_resets_only_that_field() {
        let dir = scratch_dir("field");
        std::fs::create_dir_all(&dir).unwrap();
        let saved = r##"{"tool":"pencil","color":"#123abc","shortcut":"Alt+KeyG",
            "keybindings":{"undo":"CommandOrControl+KeyU"},"toolbarPosition":"left",
            "autoFadeSeconds":4,"tutorialCompleted":true}"##;
        std::fs::write(dir.join(FILE_NAME), saved).unwrap();
        let (preferences, warning) = load_from(&dir);
        assert!(warning.is_some());
        assert_eq!(preferences.tool, Preferences::default().tool);
        assert_eq!(preferences.auto_fade_seconds, 0);
        assert_eq!(preferences.color, "#123abc");
        assert_eq!(preferences.shortcut, "Alt+KeyG");
        assert_eq!(preferences.keybindings["undo"], "CommandOrControl+KeyU");
        assert_eq!(preferences.toolbar_position, ToolbarPosition::Left);
        assert!(preferences.tutorial_completed);
        let (_, reset) = recover(serde_json::from_str(saved).unwrap());
        assert_eq!(reset, ["autoFadeSeconds", "tool"]);
        let (reloaded, warning) = load_from(&dir);
        assert!(reloaded == preferences);
        assert!(warning.is_none(), "the recovered fields were saved");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn unreadable_preferences_are_moved_aside_before_defaults_are_saved() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch_dir("unreadable");
        let custom = Preferences {
            tool: "pen".into(),
            ..Preferences::default()
        };
        custom.save_to(&dir).unwrap();
        let original = std::fs::read(dir.join(FILE_NAME)).unwrap();
        let path = dir.join(FILE_NAME);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read(&path).is_ok() {
            // Running as root, which can read the file anyway.
            return;
        }
        let (preferences, warning) = load_from(&dir);
        assert!(preferences == Preferences::default());
        assert!(warning.unwrap().contains(BACKUP_NAME));
        let backup = dir.join(BACKUP_NAME);
        std::fs::set_permissions(&backup, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(std::fs::read(&backup).unwrap(), original);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn palette_upgrades_apply_only_to_files_saved_before_versions() {
        let dir = scratch_dir("legacy");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(FILE_NAME),
            r##"{"tool":"pen","color":"#3388ff","shortcut":"CommandOrControl+Shift+A"}"##,
        )
        .unwrap();
        let (legacy, warning) = load_from(&dir);
        assert_eq!(legacy.color, "#669df0");
        assert!(warning.is_none());
        // A color the user picks now is kept, both from the webview and from disk.
        let picked: Preferences = serde_json::from_str(
            r##"{"tool":"pen","color":"#3388ff","shortcut":"CommandOrControl+Shift+A"}"##,
        )
        .unwrap();
        assert_eq!(picked.color, "#3388ff");
        picked.save_to(&dir).unwrap();
        assert_eq!(load_from(&dir).0.color, "#3388ff");
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn color_modes_survive_saving_and_loading() {
        for mode in [ColorMode::Solid, ColorMode::Rainbow, ColorMode::Cycle] {
            let preferences = Preferences {
                color_mode: mode,
                ..Preferences::default()
            };
            let bytes = serde_json::to_vec(&preferences).unwrap();
            let loaded: Preferences = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(loaded.color_mode, mode);
            assert!(loaded.validate().is_ok());
        }
    }
}
