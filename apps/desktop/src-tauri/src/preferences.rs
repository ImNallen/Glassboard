use crate::{windows::geometry::ToolbarPosition, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{collections::BTreeMap, io::Write, path::Path, sync::LazyLock};
use tauri::Manager;
use tauri_plugin_global_shortcut::{Modifiers, Shortcut};

const FILE_NAME: &str = "preferences.json";
const BACKUP_NAME: &str = "preferences.json.bak";
/// Saved files carry this so upgrades apply only to files written before them.
const FORMAT_VERSION: u64 = 1;
const INVALID_DRAWING: &str = "Invalid drawing preferences";

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Tool {
    Pen,
    Arrow,
    Rectangle,
    Ellipse,
    Highlighter,
    Text,
    Eraser,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ColorMode {
    Solid,
    Rainbow,
    Cycle,
}

/// `#rrggbb`, kept in the case the user typed.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "String")]
pub(crate) struct HexColor(String);
impl TryFrom<String> for HexColor {
    type Error = &'static str;
    fn try_from(color: String) -> std::result::Result<Self, Self::Error> {
        let valid = color.len() == 7
            && color.starts_with('#')
            && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit);
        valid.then_some(Self(color)).ok_or(INVALID_DRAWING)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "Vec<HexColor>")]
pub(crate) struct ColorSequence(Vec<HexColor>);
impl TryFrom<Vec<HexColor>> for ColorSequence {
    type Error = &'static str;
    fn try_from(colors: Vec<HexColor>) -> std::result::Result<Self, Self::Error> {
        (2..=8)
            .contains(&colors.len())
            .then_some(Self(colors))
            .ok_or(INVALID_DRAWING)
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "u8")]
pub(crate) struct AutoFadeSeconds(u8);
impl TryFrom<u8> for AutoFadeSeconds {
    type Error = &'static str;
    fn try_from(seconds: u8) -> std::result::Result<Self, Self::Error> {
        [0, 3, 5, 10]
            .contains(&seconds)
            .then_some(Self(seconds))
            .ok_or(INVALID_DRAWING)
    }
}

/// In-app binding overrides by command id; an empty shortcut unbinds the command.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "BTreeMap<String, String>")]
pub(crate) struct Keybindings(BTreeMap<String, String>);
impl TryFrom<BTreeMap<String, String>> for Keybindings {
    type Error = &'static str;
    fn try_from(bindings: BTreeMap<String, String>) -> std::result::Result<Self, Self::Error> {
        let invalid = bindings.len() > 64
            || bindings.iter().any(|(command, shortcut)| {
                command.is_empty()
                    || command.len() > 32
                    || !command
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                    || (!shortcut.is_empty() && shortcut.parse::<Shortcut>().is_err())
            });
        (!invalid)
            .then_some(Self(bindings))
            .ok_or("Invalid keybindings")
    }
}

/// A system-wide shortcut, saved as the user recorded it.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct GlobalShortcut {
    text: String,
    parsed: Shortcut,
}
impl GlobalShortcut {
    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }
    pub(crate) fn parsed(&self) -> Shortcut {
        self.parsed
    }
}
impl TryFrom<String> for GlobalShortcut {
    type Error = String;
    fn try_from(text: String) -> std::result::Result<Self, Self::Error> {
        let parsed: Shortcut = text.parse().map_err(|e| format!("Invalid shortcut: {e}"))?;
        if !parsed
            .mods
            .intersects(Modifiers::CONTROL | Modifiers::SUPER | Modifiers::ALT)
        {
            return Err("Include CommandOrControl, Control, Super, or Alt in the shortcut.".into());
        }
        Ok(Self { text, parsed })
    }
}
impl From<GlobalShortcut> for String {
    fn from(shortcut: GlobalShortcut) -> Self {
        shortcut.text
    }
}

/// The global screenshot shortcut; an empty string leaves it unbound.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(try_from = "String", into = "String")]
pub(crate) struct CaptureShortcut(Option<GlobalShortcut>);
impl CaptureShortcut {
    pub(crate) fn get(&self) -> Option<&GlobalShortcut> {
        self.0.as_ref()
    }
}
impl TryFrom<String> for CaptureShortcut {
    type Error = String;
    fn try_from(text: String) -> std::result::Result<Self, Self::Error> {
        if text.is_empty() {
            return Ok(Self(None));
        }
        text.try_into().map(|shortcut| Self(Some(shortcut)))
    }
}
impl From<CaptureShortcut> for String {
    fn from(shortcut: CaptureShortcut) -> Self {
        shortcut.0.map(String::from).unwrap_or_default()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Preferences {
    pub(crate) tool: Tool,
    pub(crate) color: HexColor,
    pub(crate) color_mode: ColorMode,
    /// Toggles drawing. Saved files keep the name from when it was the only global shortcut.
    pub(crate) shortcut: GlobalShortcut,
    pub(crate) capture_shortcut: CaptureShortcut,
    pub(crate) keybindings: Keybindings,
    /// The toolbar's solid color swatches, in toolbar order.
    pub(crate) swatches: [HexColor; 6],
    /// Rainbow's gradient colors, in order.
    pub(crate) rainbow_colors: ColorSequence,
    /// The colors Shifting steps through, one per shape.
    pub(crate) cycle_colors: ColorSequence,
    pub(crate) toolbar_position: ToolbarPosition,
    pub(crate) auto_fade_seconds: AutoFadeSeconds,
    pub(crate) tutorial_completed: bool,
}
/// The webviews import the same file, so the defaults have one source.
const DEFAULTS_JSON: &str =
    include_str!("../../../../packages/ui/src/contract/preference-defaults.json");
static DEFAULTS: LazyLock<Preferences> = LazyLock::new(|| {
    serde_json::from_str(DEFAULTS_JSON).expect("preference-defaults.json holds valid preferences")
});
impl Default for Preferences {
    fn default() -> Self {
        DEFAULTS.clone()
    }
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
    /// Parses a saved file or webview payload. A missing field takes the shared default.
    pub(crate) fn parse(saved: Value) -> serde_json::Result<Self> {
        let Value::Object(fields) = saved else {
            return serde_json::from_value(saved);
        };
        let mut merged = serde_json::to_value(Self::default())?;
        for (field, value) in fields {
            merged[field] = value;
        }
        let preferences: Self = serde_json::from_value(merged)?;
        if preferences
            .capture_shortcut
            .get()
            .map(GlobalShortcut::parsed)
            == Some(preferences.shortcut.parsed())
        {
            return Err(serde::de::Error::custom(
                "Toggling Glassboard and taking a screenshot need different shortcuts.",
            ));
        }
        Ok(preferences)
    }
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
        let dir = app.path().app_config_dir()?;
        self.save_to(&dir)
    }

    fn save_to(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        let mut saved = serde_json::to_value(self)?;
        saved["version"] = FORMAT_VERSION.into();
        let bytes = serde_json::to_vec_pretty(&saved)?;
        Ok(write_atomically(&dir.join(FILE_NAME), &bytes)?)
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
    // Files from before the screenshot shortcut keep a toggle shortcut that took its default.
    if !saved.contains_key("captureShortcut") {
        let toggle = saved.get("shortcut").and_then(Value::as_str);
        let default = Preferences::default().capture_shortcut;
        if toggle.and_then(|text| text.parse::<Shortcut>().ok())
            == default.get().map(GlobalShortcut::parsed)
        {
            saved.insert("captureShortcut".into(), "".into());
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
    Preferences::parse(Value::Object(fields.clone())).ok()
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
    use serde_json::json;
    const SHORTCUT_GRAMMAR: &str =
        include_str!("../../../../packages/ui/src/contract/toggle-shortcuts.json");
    fn hex(color: &str) -> HexColor {
        HexColor(color.into())
    }
    const CUSTOM_JSON: &str = r##"{"tool":"highlighter","color":"#123aBC","colorMode":"cycle","shortcut":"Alt+Shift+KeyG","captureShortcut":"","keybindings":{"color-red":"Digit9","copy":"","undo":"CommandOrControl+KeyU"},"swatches":["#111111","#eeeeee","#4dcaa0","#f2c85b","#F46B78","#669df0"],"rainbowColors":["#000000","#ffffff"],"cycleColors":["#f46b78","#f2c85b","#4dcaa0","#4fc5d5","#669df0","#a184e8","#e580b5","#123456"],"toolbarPosition":"left","autoFadeSeconds":5,"tutorialCompleted":true}"##;
    fn shortcut(text: &str) -> GlobalShortcut {
        String::from(text).try_into().unwrap()
    }
    fn with(field: &str, value: Value) -> serde_json::Result<Preferences> {
        let mut fields = serde_json::to_value(Preferences::default()).unwrap();
        fields[field] = value;
        serde_json::from_value(fields)
    }
    #[test]
    fn saved_and_sent_preferences_keep_their_exact_json() {
        let default_json = serde_json::to_string(&Preferences::default()).unwrap();
        // Every field in the shared file is one the struct knows; an unknown key would be dropped here.
        assert_eq!(
            serde_json::from_str::<Value>(&default_json).unwrap(),
            serde_json::from_str::<Value>(DEFAULTS_JSON).unwrap()
        );
        for (name, json) in [("default", default_json.as_str()), ("custom", CUSTOM_JSON)] {
            let sent: Preferences = serde_json::from_str(json).unwrap();
            assert_eq!(serde_json::to_string(&sent).unwrap(), json, "{name}");
            let mut file: Value = serde_json::from_str(json).unwrap();
            file["version"] = FORMAT_VERSION.into();
            let stored = serde_json::to_vec_pretty(&file).unwrap();
            let dir = scratch_dir(&format!("exact-{name}"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(FILE_NAME), &stored).unwrap();
            let (loaded, warning) = load_from(&dir);
            assert!(warning.is_none(), "{name}");
            loaded.save_to(&dir).unwrap();
            assert_eq!(
                std::fs::read(dir.join(FILE_NAME)).unwrap(),
                stored,
                "{name}"
            );
            std::fs::remove_dir_all(dir).unwrap();
        }
    }
    #[test]
    fn toggle_shortcuts_follow_the_shared_grammar() {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Grammar {
            recordable_keys: Vec<String>,
            toggle: Toggle,
        }
        #[derive(Deserialize)]
        struct Toggle {
            valid: Vec<String>,
            unmodified: Vec<String>,
            malformed: Vec<String>,
        }
        let grammar: Grammar = serde_json::from_str(SHORTCUT_GRAMMAR).unwrap();
        for key in &grammar.recordable_keys {
            assert!(
                with("shortcut", format!("Alt+{key}").into()).is_ok(),
                "{key}"
            );
        }
        for text in &grammar.toggle.valid {
            assert_eq!(
                with("shortcut", text.as_str().into())
                    .unwrap()
                    .shortcut
                    .as_str(),
                text
            );
        }
        // Settings shows this message as is when a recorded shortcut has no modifier.
        for text in &grammar.toggle.unmodified {
            assert_eq!(
                with("shortcut", text.as_str().into())
                    .unwrap_err()
                    .to_string(),
                "Include CommandOrControl, Control, Super, or Alt in the shortcut.",
                "{text}"
            );
        }
        for text in &grammar.toggle.malformed {
            assert!(with("shortcut", text.as_str().into()).is_err(), "{text}");
        }
    }
    #[test]
    fn out_of_range_drawing_preferences_are_rejected() {
        for (field, value) in [
            ("tool", json!("pencil")),
            ("colorMode", json!("neon")),
            ("toolbarPosition", json!("top")),
            ("autoFadeSeconds", json!(4)),
            ("color", json!("")),
            ("color", json!("#fff")),
            ("color", json!("#12345678")),
            ("color", json!("1234567")),
            ("color", json!("#gggggg")),
            ("color", json!("#é1234")),
            ("swatches", json!(vec!["#000000"; 5])),
            ("swatches", json!(vec!["#000000"; 7])),
            (
                "swatches",
                json!(["#000000", "#ffffff", "#4dcaa0", "#f2c85b", "#f46b78", "#fff"]),
            ),
            ("rainbowColors", json!([])),
            ("rainbowColors", json!(["#000000"])),
            ("cycleColors", json!(vec!["#000000"; 9])),
            ("cycleColors", json!(["#000000", "red"])),
            ("keybindings", json!({"undo": "not a shortcut"})),
            ("keybindings", json!({"undo": "CommandOrControl+"})),
            ("keybindings", json!({"": "KeyA"})),
            ("keybindings", json!({"Undo": "KeyA"})),
            ("keybindings", json!({"tool_pen": "KeyA"})),
        ] {
            assert!(with(field, value.clone()).is_err(), "{field}: {value}");
        }
    }
    #[test]
    fn older_preferences_default_to_rainbow_and_keep_color_and_shortcut() {
        let preferences = Preferences::parse(
            serde_json::from_str(
                r##"{"tool":"pen","color":"#68aaff","width":7,"shortcut":"CommandOrControl+Shift+B"}"##,
            )
            .unwrap(),
        )
        .unwrap();
        // Retired line-width preferences are ignored without resetting other settings.
        assert_eq!(
            preferences,
            Preferences {
                tool: Tool::Pen,
                color: hex("#68aaff"),
                shortcut: shortcut("CommandOrControl+Shift+B"),
                ..Preferences::default()
            }
        );
    }
    #[test]
    fn the_capture_shortcut_must_differ_from_the_toggle_shortcut() {
        let parse = |fields: Value| Preferences::parse(fields).map_err(|e| e.to_string());
        assert_eq!(
            parse(json!({"shortcut": "Alt+KeyG", "captureShortcut": "Alt+G"})).unwrap_err(),
            "Toggling Glassboard and taking a screenshot need different shortcuts."
        );
        assert_eq!(
            parse(json!({"shortcut": "Alt+KeyG", "captureShortcut": ""}))
                .unwrap()
                .capture_shortcut
                .get(),
            None
        );
        assert_eq!(
            parse(json!({"captureShortcut": "Shift+KeyS"})).unwrap_err(),
            "Include CommandOrControl, Control, Super, or Alt in the shortcut."
        );
    }
    #[test]
    fn files_from_before_the_capture_shortcut_take_its_default_unless_toggle_holds_it() {
        let dir = scratch_dir("capture-default");
        std::fs::create_dir_all(&dir).unwrap();
        for (toggle, capture) in [
            ("Alt+KeyG", "Super+Control+Shift+KeyS"),
            ("Control+Super+Shift+KeyS", ""),
        ] {
            std::fs::write(
                dir.join(FILE_NAME),
                json!({"version": 1, "tool": "pen", "shortcut": toggle}).to_string(),
            )
            .unwrap();
            let (preferences, warning) = load_from(&dir);
            assert!(warning.is_none(), "{toggle}");
            assert_eq!(preferences.shortcut.as_str(), toggle);
            assert_eq!(String::from(preferences.capture_shortcut), capture);
        }
        std::fs::remove_dir_all(dir).unwrap();
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
            tool: Tool::Pen,
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
        assert_eq!(
            preferences,
            Preferences {
                color: hex("#123abc"),
                shortcut: shortcut("Alt+KeyG"),
                keybindings: Keybindings([("undo".into(), "CommandOrControl+KeyU".into())].into()),
                toolbar_position: ToolbarPosition::Left,
                tutorial_completed: true,
                ..Preferences::default()
            }
        );
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
            tool: Tool::Pen,
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
        assert_eq!(legacy.color, hex("#669df0"));
        assert!(warning.is_none());
        // A color the user picks now is kept, both from the webview and from disk.
        let picked = Preferences::parse(
            serde_json::from_str(
                r##"{"tool":"pen","color":"#3388ff","shortcut":"CommandOrControl+Shift+A"}"##,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(picked.color, hex("#3388ff"));
        picked.save_to(&dir).unwrap();
        assert_eq!(load_from(&dir).0.color, hex("#3388ff"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
