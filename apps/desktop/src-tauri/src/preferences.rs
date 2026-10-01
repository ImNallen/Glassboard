use crate::{toolbar_position::ToolbarPosition, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Modifiers, Shortcut};

pub(crate) const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+A";

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
    #[serde(deserialize_with = "deserialize_color")]
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
// Upgrade saved palette colors while retaining all other preferences.
fn deserialize_color<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<String, D::Error> {
    let color = String::deserialize(deserializer)?;
    Ok(match color.as_str() {
        "#efa5a5" | "#ff3355" => "#f46b78".into(),
        "#efb895" | "#ff8a1f" | "#e8cf91" | "#f5ff00" => "#f2c85b".into(),
        "#c6d99c" | "#a3ff12" | "#9fd5b5" | "#00f58a" => "#4dcaa0".into(),
        "#98d3cf" | "#00f0ff" => "#4fc5d5".into(),
        "#9fc5e8" | "#3388ff" => "#669df0".into(),
        "#afb5e5" | "#7855ff" | "#c9ace0" | "#c43cff" => "#a184e8".into(),
        "#e1a9cf" | "#ff33cc" => "#e580b5".into(),
        _ => color,
    })
}
impl Preferences {
    pub(crate) fn load(app: &tauri::AppHandle) -> Self {
        app.path()
            .app_config_dir()
            .ok()
            .and_then(|dir| std::fs::read(dir.join("preferences.json")).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(|preferences| preferences.validate().is_ok())
            .unwrap_or_default()
    }

    pub(crate) fn save(&self, app: &tauri::AppHandle) -> Result<()> {
        let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("preferences.json"), bytes).map_err(|e| e.to_string())
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
