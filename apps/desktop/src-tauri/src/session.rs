use crate::{preferences::Preferences, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Mode {
    Hidden,
    Draw,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TutorialStep {
    Welcome,
    Draw,
    Hide,
    Done,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Session {
    pub(crate) mode: Mode,
    pub(crate) annotation_session: u32,
    pub(crate) tutorial: Option<TutorialStep>,
    pub(crate) settings_open: bool,
    pub(crate) active_overlay: String,
    pub(crate) cycle_index: u32,
    pub(crate) history_by_overlay: HashMap<String, HistoryAvailability>,
    pub(crate) preferences: Preferences,
    pub(crate) error: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HistoryAvailability {
    pub(crate) can_undo: bool,
    pub(crate) can_redo: bool,
}
impl Session {
    pub(crate) fn new(preferences: Preferences) -> Self {
        Self {
            mode: Mode::Hidden,
            annotation_session: 0,
            tutorial: if preferences.tutorial_completed {
                None
            } else {
                Some(TutorialStep::Welcome)
            },
            settings_open: false,
            active_overlay: "overlay-0".into(),
            cycle_index: 0,
            history_by_overlay: HashMap::new(),
            preferences,
            error: None,
        }
    }
    pub(crate) fn record_history(
        &mut self,
        overlay: &str,
        availability: HistoryAvailability,
        annotation_session: u32,
        advance_cycle: bool,
    ) {
        if annotation_session != self.annotation_session {
            return;
        }
        if self.tutorial == Some(TutorialStep::Draw) && availability.can_undo {
            self.tutorial = Some(TutorialStep::Hide);
        }
        self.history_by_overlay.insert(overlay.into(), availability);
        if advance_cycle {
            self.cycle_index = self.cycle_index.wrapping_add(1);
        }
    }
    pub(crate) fn transition(&mut self, action: &str) -> Result<()> {
        let previous_mode = self.mode;
        match action {
            "toggle" => {
                self.mode = if self.mode == Mode::Hidden {
                    Mode::Draw
                } else {
                    Mode::Hidden
                };
            }
            "show" => self.mode = Mode::Draw,
            "hide" => self.mode = Mode::Hidden,
            "tutorial-start" => {
                self.mode = Mode::Draw;
                self.tutorial = Some(TutorialStep::Draw);
            }
            "replay-tutorial" => {
                self.mode = Mode::Hidden;
                self.tutorial = Some(TutorialStep::Welcome);
            }
            "dismiss-tutorial" => self.tutorial = None,
            "settings" => self.settings_open = true,
            "close-settings" => self.settings_open = false,
            "dismiss-error" => self.error = None,
            _ => return Err("Unknown action".into()),
        }
        if previous_mode == Mode::Draw && self.mode == Mode::Hidden {
            self.annotation_session += 1;
            self.history_by_overlay.clear();
        }
        if previous_mode == Mode::Hidden
            && self.mode == Mode::Draw
            && self.tutorial == Some(TutorialStep::Welcome)
        {
            self.tutorial = Some(TutorialStep::Draw);
        }
        if previous_mode == Mode::Draw && self.mode == Mode::Hidden && action != "replay-tutorial" {
            self.tutorial = match self.tutorial {
                Some(TutorialStep::Hide) => Some(TutorialStep::Done),
                Some(TutorialStep::Draw) => Some(TutorialStep::Welcome),
                other => other,
            };
        }
        if matches!(
            action,
            "toggle" | "show" | "hide" | "tutorial-start" | "replay-tutorial"
        ) {
            self.settings_open = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        Session::new(Preferences::default())
    }
    #[test]
    fn hide_and_restore_resets_input_mode() {
        let mut s = session();
        s.transition("show").unwrap();
        s.transition("hide").unwrap();
        s.transition("toggle").unwrap();
        assert_eq!(s.mode, Mode::Draw);
        // The native enum must retain the frontend's string-based wire format.
        assert_eq!(serde_json::to_value(&s).unwrap()["mode"], "draw");
        s.transition("hide").unwrap();
        assert_eq!(serde_json::to_value(&s).unwrap()["mode"], "hidden");
    }
    #[test]
    fn removed_toolbar_action_is_rejected() {
        assert!(session().transition("toolbar").is_err());
    }
    #[test]
    fn settings_can_open_while_annotations_stay_hidden() {
        let mut s = session();
        s.transition("hide").unwrap();
        s.transition("settings").unwrap();
        assert!(s.settings_open);
        assert_eq!(s.mode, Mode::Hidden);
        s.transition("close-settings").unwrap();
        assert!(!s.settings_open);
        assert_eq!(s.mode, Mode::Hidden);
        s.transition("settings").unwrap();
        s.transition("toggle").unwrap();
        assert!(!s.settings_open);
        assert_eq!(s.mode, Mode::Draw);
    }
    #[test]
    fn removed_interact_action_is_rejected() {
        assert!(session().transition("interact").is_err());
    }
    #[test]
    fn leaving_annotations_resets_every_display_before_reopening() {
        for exit in ["hide", "toggle", "replay-tutorial"] {
            let mut s = session();
            s.transition("show").unwrap();
            let generation = s.annotation_session;
            for overlay in ["overlay-0", "overlay-1"] {
                s.record_history(
                    overlay,
                    HistoryAvailability {
                        can_undo: true,
                        can_redo: true,
                    },
                    generation,
                    false,
                );
            }
            s.transition("show").unwrap();
            assert_eq!(s.annotation_session, generation);
            assert_eq!(s.history_by_overlay.len(), 2);
            s.transition(exit).unwrap();
            assert_eq!(s.mode, Mode::Hidden);
            assert_eq!(s.annotation_session, generation + 1);
            assert!(s.history_by_overlay.is_empty());
            s.record_history(
                "overlay-1",
                HistoryAvailability {
                    can_undo: true,
                    can_redo: true,
                },
                generation,
                true,
            );
            assert!(s.history_by_overlay.is_empty());
            assert_eq!(s.cycle_index, 0);
            s.transition("hide").unwrap();
            s.transition("show").unwrap();
            assert_eq!(s.annotation_session, generation + 1);
            assert!(s.history_by_overlay.is_empty());
        }
    }
    #[test]
    fn tutorial_follows_a_completed_mark_and_return_to_work() {
        let mut s = session();
        assert_eq!(s.mode, Mode::Hidden);
        assert_eq!(s.tutorial, Some(TutorialStep::Welcome));
        s.transition("tutorial-start").unwrap();
        assert_eq!(s.tutorial, Some(TutorialStep::Draw));
        s.record_history(
            "overlay-0",
            HistoryAvailability {
                can_undo: false,
                can_redo: false,
            },
            s.annotation_session,
            false,
        );
        assert_eq!(s.tutorial, Some(TutorialStep::Draw));
        s.record_history(
            "overlay-0",
            HistoryAvailability {
                can_undo: true,
                can_redo: false,
            },
            s.annotation_session,
            false,
        );
        assert_eq!(s.tutorial, Some(TutorialStep::Hide));
        s.transition("hide").unwrap();
        assert_eq!(s.tutorial, Some(TutorialStep::Done));
        assert_eq!(s.mode, Mode::Hidden);
        s.transition("dismiss-tutorial").unwrap();
        assert_eq!(s.tutorial, None);
        s.transition("replay-tutorial").unwrap();
        assert_eq!(s.tutorial, Some(TutorialStep::Welcome));
    }
    #[test]
    fn completed_tutorial_does_not_reappear_on_launch() {
        let preferences = Preferences {
            tutorial_completed: true,
            ..Preferences::default()
        };
        let saved = serde_json::to_vec(&preferences).unwrap();
        let s = Session::new(serde_json::from_slice(&saved).unwrap());
        assert_eq!(s.mode, Mode::Hidden);
        assert_eq!(s.tutorial, None);
        let mut old = serde_json::to_value(Preferences::default()).unwrap();
        old.as_object_mut().unwrap().remove("tutorialCompleted");
        assert_eq!(
            Session::new(serde_json::from_value(old).unwrap()).tutorial,
            Some(TutorialStep::Welcome)
        );
    }
    #[test]
    fn leaving_before_drawing_allows_retrying_the_tutorial() {
        let mut s = session();
        s.transition("toggle").unwrap();
        s.transition("hide").unwrap();
        assert_eq!(s.tutorial, Some(TutorialStep::Welcome));
        s.transition("toggle").unwrap();
        assert_eq!(s.tutorial, Some(TutorialStep::Draw));
    }
}
