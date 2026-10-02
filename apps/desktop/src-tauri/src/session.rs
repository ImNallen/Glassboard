use crate::{
    preferences::Preferences,
    windows::{geometry::Rect, Surface},
};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Mode {
    Hidden,
    Draw,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TutorialStep {
    Welcome,
    Draw,
    Hide,
    Done,
}

/// Actions the session state machine owns. The rest of the IPC vocabulary is in `commands::Action`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Transition {
    Toggle,
    Show,
    Hide,
    CancelCapture,
    TutorialStart,
    ReplayTutorial,
    DismissTutorial,
    Settings,
    CloseSettings,
    DismissError,
}
impl Transition {
    fn leaves_capture(self) -> bool {
        use Transition::*;
        match self {
            Toggle | Show | Hide | CancelCapture | TutorialStart | ReplayTutorial | Settings => {
                true
            }
            DismissTutorial | CloseSettings | DismissError => false,
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Session {
    pub(crate) mode: Mode,
    pub(crate) annotation_session: u32,
    pub(crate) tutorial: Option<TutorialStep>,
    pub(crate) settings_open: bool,
    pub(crate) active_overlay: Surface,
    pub(crate) cycle_index: u32,
    pub(crate) history_by_overlay: HashMap<Surface, HistoryAvailability>,
    pub(crate) preferences: Preferences,
    pub(crate) error: Option<String>,
    /// The toggle shortcut could not be registered, usually because another app holds it.
    pub(crate) shortcut_unavailable: bool,
    pub(crate) capture: Option<CaptureSession>,
}
#[derive(Clone)]
pub(crate) struct CaptureSession {
    pub(crate) id: u32,
    pub(crate) ready: Option<ReadyCapture>,
}
/// The captured frame lives only here, in memory, and goes away with the
/// capture on copy, cancellation, or a mode change.
#[derive(Clone)]
pub(crate) struct ReadyCapture {
    /// The native toolbar's bounds in the capture webview's CSS pixels.
    pub(crate) toolbar_dock: Rect,
    pub(crate) image: Arc<image::RgbaImage>,
}
/// The webviews see `{ id, ready, toolbarDock? }` and fetch the frame as binary on request.
impl Serialize for CaptureSession {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut capture = serializer.serialize_struct("CaptureSession", 3)?;
        capture.serialize_field("id", &self.id)?;
        capture.serialize_field("ready", &self.ready.is_some())?;
        if let Some(ready) = &self.ready {
            capture.serialize_field("toolbarDock", &ready.toolbar_dock)?;
        }
        capture.end()
    }
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
            active_overlay: Surface::Overlay(0),
            cycle_index: 0,
            history_by_overlay: HashMap::new(),
            preferences,
            error: None,
            shortcut_unavailable: false,
            capture: None,
        }
    }
    /// Opens a capture and returns its ID, which the frame must carry back to `finish_capture`.
    pub(crate) fn begin_capture(&mut self) -> u32 {
        // Captures own a fresh history, independent of live screen annotations.
        self.annotation_session = self.annotation_session.wrapping_add(1);
        self.history_by_overlay.clear();
        self.mode = Mode::Hidden;
        self.settings_open = false;
        if matches!(self.tutorial, Some(TutorialStep::Draw | TutorialStep::Hide)) {
            self.tutorial = Some(TutorialStep::Welcome);
        }
        self.active_overlay = Surface::Capture;
        self.error = None;
        let id = self.annotation_session;
        self.capture = Some(CaptureSession { id, ready: None });
        id
    }
    /// Attaches a captured frame to capture `id`. A frame for a capture that was
    /// cancelled or replaced while the screen was being read is dropped instead.
    pub(crate) fn finish_capture(&mut self, id: u32, ready: ReadyCapture) -> bool {
        match self.capture.as_mut() {
            Some(capture) if capture.id == id => {
                capture.ready = Some(ready);
                true
            }
            _ => false,
        }
    }
    pub(crate) fn record_history(
        &mut self,
        overlay: Surface,
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
        self.history_by_overlay.insert(overlay, availability);
        if advance_cycle {
            self.cycle_index = self.cycle_index.wrapping_add(1);
        }
    }
    pub(crate) fn transition(&mut self, transition: Transition) {
        use Transition::*;
        let previous_mode = self.mode;
        let leaving_capture = self.capture.is_some() && transition.leaves_capture();
        match transition {
            Toggle => {
                self.mode = if self.mode == Mode::Hidden && self.capture.is_none() {
                    Mode::Draw
                } else {
                    Mode::Hidden
                };
            }
            Show => self.mode = Mode::Draw,
            Hide | CancelCapture => self.mode = Mode::Hidden,
            TutorialStart => {
                self.mode = Mode::Draw;
                self.tutorial = Some(TutorialStep::Draw);
            }
            ReplayTutorial => {
                self.mode = Mode::Hidden;
                self.tutorial = Some(TutorialStep::Welcome);
            }
            DismissTutorial => self.tutorial = None,
            Settings => self.settings_open = true,
            CloseSettings => self.settings_open = false,
            DismissError => self.error = None,
        }
        if leaving_capture {
            self.capture = None;
        }
        if leaving_capture || (previous_mode == Mode::Draw && self.mode == Mode::Hidden) {
            self.annotation_session = self.annotation_session.wrapping_add(1);
            self.history_by_overlay.clear();
        }
        if previous_mode == Mode::Hidden
            && self.mode == Mode::Draw
            && self.tutorial == Some(TutorialStep::Welcome)
        {
            self.tutorial = Some(TutorialStep::Draw);
        }
        if previous_mode == Mode::Draw && self.mode == Mode::Hidden && transition != ReplayTutorial
        {
            self.tutorial = match self.tutorial {
                Some(TutorialStep::Hide) => Some(TutorialStep::Done),
                Some(TutorialStep::Draw) => Some(TutorialStep::Welcome),
                other => other,
            };
        }
        if transition.leaves_capture() && transition != Settings {
            self.settings_open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Transition::*, *};
    use crate::commands::Action;
    use serde_json::{Map, Value};

    /// Replayed by vitest against the browser preview as well.
    const CASES: &str = include_str!("../../../../packages/ui/src/contract/transitions.json");
    #[derive(Deserialize)]
    struct Case {
        name: String,
        #[serde(default)]
        from: Map<String, Value>,
        steps: Vec<Step>,
        expect: Value,
    }
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Step {
        Action(Action),
        History(HistoryStep),
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct HistoryStep {
        history: HistoryAvailability,
        annotation_session: u32,
        #[serde(default)]
        advance_cycle: bool,
    }
    fn start(from: Map<String, Value>) -> Session {
        let mut preferences = serde_json::to_value(Preferences::default()).unwrap();
        if let Some(Value::Object(fields)) = from.get("preferences") {
            for (field, value) in fields {
                preferences[field] = value.clone();
            }
        }
        let mut s = Session::new(serde_json::from_value(preferences).unwrap());
        for (field, value) in from {
            match field.as_str() {
                "preferences" => {}
                "mode" => s.mode = serde_json::from_value(value).unwrap(),
                "tutorial" => s.tutorial = serde_json::from_value(value).unwrap(),
                "settingsOpen" => s.settings_open = serde_json::from_value(value).unwrap(),
                "error" => s.error = serde_json::from_value(value).unwrap(),
                other => panic!("transitions.json: unsupported `from` field {other}"),
            }
        }
        s
    }
    /// `expect` names only the fields it checks. An empty object requires an empty one,
    /// and `{"$rust": …, "$preview": …}` picks this side's value.
    fn mismatch(actual: &Value, expected: &Value, path: &str) -> Option<String> {
        let Value::Object(fields) = expected else {
            return (actual != expected)
                .then(|| format!("{path}: expected {expected}, got {actual}"));
        };
        if fields.keys().any(|key| key.starts_with('$')) {
            return match fields.get("$rust") {
                Some(sided) => mismatch(actual, sided, path),
                None => Some(format!("{path}: no $rust value")),
            };
        }
        let Value::Object(actual) = actual else {
            return Some(format!("{path}: expected an object, got {actual}"));
        };
        if fields.is_empty() && !actual.is_empty() {
            return Some(format!(
                "{path}: expected {{}}, got {}",
                Value::Object(actual.clone())
            ));
        }
        fields.iter().find_map(|(key, value)| {
            let actual = actual.get(key).unwrap_or(&Value::Null);
            mismatch(actual, value, &format!("{path}.{key}"))
        })
    }
    #[test]
    fn transitions_match_the_shared_fixture() {
        let cases: Vec<Case> = serde_json::from_str(CASES).unwrap();
        assert!(cases.len() > 1);
        for case in cases {
            let mut s = start(case.from);
            for step in case.steps {
                match step {
                    Step::Action(Action::Capture) => {
                        s.begin_capture();
                    }
                    Step::Action(Action::Session(transition)) => s.transition(transition),
                    Step::Action(other) => {
                        panic!("{}: {other:?} is not a session transition", case.name)
                    }
                    Step::History(step) => s.record_history(
                        s.active_overlay,
                        step.history,
                        step.annotation_session,
                        step.advance_cycle,
                    ),
                }
            }
            let published = serde_json::to_value(&s).unwrap();
            if let Some(problem) = mismatch(&published, &case.expect, "session") {
                panic!("{}: {problem}", case.name);
            }
        }
    }
    #[test]
    fn a_frame_only_attaches_to_the_capture_it_was_taken_for() {
        let ready = || ReadyCapture {
            toolbar_dock: Rect {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0,
            },
            image: Arc::new(image::RgbaImage::new(1, 1)),
        };
        let published = |s: &Session| serde_json::to_value(s).unwrap()["capture"].clone();
        let mut s = Session::new(Preferences::default());
        let stale = s.begin_capture();
        s.transition(CancelCapture);
        assert!(!s.finish_capture(stale, ready()));
        assert!(s.capture.is_none());
        let id = s.begin_capture();
        assert!(!s.finish_capture(stale, ready()));
        assert_eq!(
            published(&s),
            serde_json::json!({ "id": id, "ready": false })
        );
        assert!(s.finish_capture(id, ready()));
        // The frame never crosses the session event; the webview requests it by ID.
        assert_eq!(
            published(&s),
            serde_json::json!({
                "id": id,
                "ready": true,
                "toolbarDock": { "x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0 }
            })
        );
    }
}
