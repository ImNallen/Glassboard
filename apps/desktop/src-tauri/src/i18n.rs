use serde::{Deserialize, Serialize, Serializer};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering::Relaxed},
        LazyLock,
    },
};

include!(concat!(env!("OUT_DIR"), "/catalogs.rs"));

type Catalog = HashMap<String, String>;
const SYSTEM: &str = "system";

#[cfg(not(test))]
const TEST_CATALOGS: &[(&str, &str)] = &[];
#[cfg(test)]
const TEST_CATALOGS: &[(&str, &str)] = &[(
    "x-pseudo",
    r#"{"Capture window is unavailable":"⟦Capture window⟧","Could not open the clipboard: {detail}":"{detail} ⟦clipboard⟧"}"#,
)];

static LANGUAGES: LazyLock<Vec<(&'static str, Catalog)>> = LazyLock::new(|| {
    CATALOGS
        .iter()
        .chain(TEST_CATALOGS)
        .map(|&(tag, json)| {
            (
                tag,
                serde_json::from_str(json).expect("catalogs are flat JSON"),
            )
        })
        .collect()
});

fn tags() -> Vec<&'static str> {
    LANGUAGES.iter().map(|&(tag, _)| tag).collect()
}

/// A sentence waiting for a language, and the crate's error type. Stored errors and notices
/// stay `Text` until shown, so they follow a language change without a new event.
/// `Serialize` renders the active language. `Display` renders English, for logs.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Text(Repr);
#[derive(Clone, Debug, PartialEq)]
enum Repr {
    Message {
        key: &'static str,
        args: Vec<(&'static str, String)>,
    },
    /// A library's text, or an invariant no user reaches.
    Untranslated(String),
    /// Sentences shown together, such as the startup problems.
    Joined(Vec<Text>),
}

pub(crate) type Result<T> = std::result::Result<T, Text>;

impl Text {
    /// Only `t!` calls this, after its `check` passes. The audit enforces that.
    pub(crate) fn message(key: &'static str, args: Vec<(&'static str, String)>) -> Self {
        Self(Repr::Message { key, args })
    }
    pub(crate) fn untranslated(text: impl ToString) -> Self {
        Self(Repr::Untranslated(text.to_string()))
    }
    pub(crate) fn joined(parts: Vec<Text>) -> Self {
        Self(Repr::Joined(parts))
    }
    /// For tray labels, titles, native dialogs, and serde errors, which need a string now.
    pub(crate) fn render(&self) -> String {
        self.render_in(&LANGUAGES[active().load(Relaxed)].1)
    }
    fn render_in(&self, catalog: &Catalog) -> String {
        match &self.0 {
            Repr::Message { key, args } => {
                fill(catalog.get(*key).map_or(key, String::as_str), args)
            }
            Repr::Untranslated(text) => text.clone(),
            Repr::Joined(parts) => parts
                .iter()
                .map(|part| part.render_in(catalog))
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}
impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.render_in(&Catalog::new()))
    }
}
impl Serialize for Text {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.render())
    }
}

fn fill(template: &str, args: &[(&'static str, String)]) -> String {
    let mut filled = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        filled.push_str(&rest[..open]);
        rest = &rest[open..];
        let value = rest.find('}').and_then(|close| {
            let (_, value) = args.iter().find(|(name, _)| *name == &rest[1..close])?;
            Some((value, close))
        });
        match value {
            Some((value, close)) => {
                filled.push_str(value);
                rest = &rest[close + 1..];
            }
            None => {
                filled.push('{');
                rest = &rest[1..];
            }
        }
    }
    filled.push_str(rest);
    filled
}

// A blanket `impl<E: std::error::Error>` would also cover `Text`'s own conversions.
macro_rules! from_library {
    ($($source:ty),* $(,)?) => {$(
        impl From<$source> for Text {
            fn from(error: $source) -> Self {
                t!("Something went wrong: {detail}", detail = error)
            }
        }
    )*};
}

/// A sentence from `messages.json`, with its placeholders as named arguments. A typo in
/// either fails the build through `check`.
macro_rules! t {
    ($key:literal $(, $name:ident = $value:expr)* $(,)?) => {{
        const _: () = $crate::i18n::check($key, &[$(stringify!($name)),*]);
        $crate::i18n::Text::message($key, vec![$((stringify!($name), $value.to_string())),*])
    }};
}
pub(crate) use t;

from_library!(
    tauri::Error,
    std::io::Error,
    serde_json::Error,
    xcap::XCapError,
    std::num::TryFromIntError,
    tauri_plugin_opener::Error,
    tauri_plugin_global_shortcut::Error,
    std::sync::mpsc::RecvError,
);

const fn same(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut at = 0;
    while at < a.len() {
        if a[at] != b[at] {
            return false;
        }
        at += 1;
    }
    true
}

/// Evaluated at every `t!`. Panics, which fails the build, on a sentence missing from
/// `messages.json` or arguments that differ from its placeholders.
pub(crate) const fn check(key: &str, names: &[&str]) {
    let mut known = false;
    let mut index = 0;
    while index < KEYS.len() {
        known = known || same(KEYS[index].as_bytes(), key.as_bytes());
        index += 1;
    }
    assert!(
        known,
        "this sentence is not in packages/ui/src/contract/messages.json"
    );
    let bytes = key.as_bytes();
    let mut used = [false; 8];
    assert!(names.len() <= used.len(), "too many arguments");
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'{' {
            let mut close = at + 1;
            while bytes[close] != b'}' {
                close += 1;
            }
            let (_, after) = bytes.split_at(at + 1);
            let (placeholder, _) = after.split_at(close - at - 1);
            let mut found = false;
            let mut name = 0;
            while name < names.len() {
                if same(names[name].as_bytes(), placeholder) {
                    found = true;
                    used[name] = true;
                }
                name += 1;
            }
            assert!(found, "this sentence has a placeholder with no argument");
            at = close;
        }
        at += 1;
    }
    let mut name = 0;
    while name < names.len() {
        assert!(
            used[name],
            "this argument is not a placeholder in the sentence"
        );
        name += 1;
    }
}

/// The saved request: "system" or a tag. Unvalidated, so a tag saved by a newer build
/// survives a downgrade and falls through to the OS list without a reset notice.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Language(String);

/// A `Session` field that serializes the active tag, so no stored copy can disagree with it.
#[derive(Clone, Copy)]
pub(crate) struct ActiveLocale;
impl Serialize for ActiveLocale {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(active_tag())
    }
}

pub(crate) fn active_tag() -> &'static str {
    LANGUAGES[active().load(Relaxed)].0
}

fn os_locales() -> Vec<String> {
    if cfg!(test) {
        Vec::new()
    } else {
        sys_locale::get_locales().collect()
    }
}

/// Index into `LANGUAGES`. The first read uses the OS list, so text rendered before preferences
/// load already follows the OS. Tests run in parallel, so each thread gets its own language.
fn active() -> &'static AtomicUsize {
    #[cfg(test)]
    {
        thread_local!(static ACTIVE: &'static AtomicUsize = Box::leak(Box::new(AtomicUsize::new(negotiate(os_locales(), &tags())))));
        ACTIVE.with(|active| *active)
    }
    #[cfg(not(test))]
    {
        static ACTIVE: std::sync::LazyLock<AtomicUsize> =
            std::sync::LazyLock::new(|| AtomicUsize::new(negotiate(os_locales(), &tags())));
        &ACTIVE
    }
}

pub(crate) fn activate(language: &Language) {
    let saved = (language.0 != SYSTEM).then(|| language.0.clone());
    active().store(
        negotiate(saved.into_iter().chain(os_locales()), &tags()),
        Relaxed,
    );
}

struct Subtags {
    language: String,
    script: Option<String>,
    region: Option<String>,
}
impl Subtags {
    fn parse(tag: &str) -> Self {
        let mut parts = tag.split(['-', '_']);
        let language = parts.next().unwrap_or_default().to_ascii_lowercase();
        let (mut script, mut region) = (None, None);
        for part in parts {
            let letters = part.bytes().all(|byte| byte.is_ascii_alphabetic());
            match part.len() {
                4 if letters => script = Some(part.to_ascii_lowercase()),
                2 if letters => region = Some(part.to_ascii_uppercase()),
                3 if part.bytes().all(|byte| byte.is_ascii_digit()) => region = Some(part.into()),
                _ => {}
            }
        }
        if language == "zh" && script.is_none() {
            let traditional = matches!(region.as_deref(), Some("TW" | "HK" | "MO"));
            script = Some(if traditional { "hant" } else { "hans" }.into());
        }
        Self {
            language,
            script,
            region,
        }
    }
    fn compatible(&self, other: &Self) -> bool {
        self.language == other.language
            && (self.script.is_none() || other.script.is_none() || self.script == other.script)
    }
}

/// The first requested tag with a compatible catalog wins. Among compatible catalogs the
/// exact region is best, then one with no region. English is the last resort.
fn negotiate(requested: impl IntoIterator<Item = String>, tags: &[&str]) -> usize {
    let catalogs: Vec<Subtags> = tags.iter().map(|tag| Subtags::parse(tag)).collect();
    requested
        .into_iter()
        .find_map(|requested| {
            let wanted = Subtags::parse(&requested);
            let family = || (0..tags.len()).filter(|&i| catalogs[i].compatible(&wanted));
            family()
                .find(|&i| catalogs[i].region == wanted.region)
                .or_else(|| family().find(|&i| catalogs[i].region.is_none()))
                .or_else(|| family().next())
        })
        .or_else(|| tags.iter().position(|&tag| tag == "en"))
        .unwrap_or(0)
}

/// Switches this thread to the fixture until dropped, since `--test-threads=1` shares one thread.
#[cfg(test)]
pub(crate) struct Pseudo;
#[cfg(test)]
impl Pseudo {
    pub(crate) fn activate() -> Self {
        activate(&Language("x-pseudo".into()));
        Self
    }
}
#[cfg(test)]
impl Drop for Pseudo {
    fn drop(&mut self) {
        activate(&Language(SYSTEM.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pseudo() -> &'static Catalog {
        &LANGUAGES
            .iter()
            .find(|(tag, _)| *tag == "x-pseudo")
            .expect("the fixture is registered")
            .1
    }

    #[test]
    fn every_catalog_parses_and_english_ships() {
        assert!(tags().contains(&"en"));
        assert!(!LANGUAGES.is_empty());
    }

    #[test]
    fn a_rust_test_starts_in_english_whatever_the_machine_is_set_to() {
        assert_eq!(active_tag(), "en");
        assert_eq!(
            t!("Could not open the clipboard: {detail}", detail = "busy").render(),
            "Could not open the clipboard: busy"
        );
    }

    #[test]
    fn a_sentence_renders_in_a_catalog_and_keeps_its_arguments() {
        let text = t!("Could not open the clipboard: {detail}", detail = "busy");
        assert_eq!(text.render_in(pseudo()), "busy ⟦clipboard⟧");
        assert_eq!(text.to_string(), "Could not open the clipboard: busy");
    }

    #[test]
    fn a_sentence_a_catalog_lacks_falls_back_to_english() {
        let text = t!("The capture is too large to copy");
        assert_eq!(text.render_in(pseudo()), "The capture is too large to copy");
    }

    #[test]
    fn an_argument_cannot_expand_another_placeholder() {
        let text = t!(
            "Could not register {shortcut}: {detail}",
            shortcut = "{detail}",
            detail = "held"
        );
        assert_eq!(text.to_string(), "Could not register {detail}: held");
    }

    #[test]
    fn joined_text_and_library_errors_render_as_one_line() {
        let joined = Text::joined(vec![
            t!("Capture window is unavailable"),
            Text::untranslated("kept as is"),
        ]);
        assert_eq!(joined.render_in(pseudo()), "⟦Capture window⟧ kept as is");
        let library = Text::from(std::io::Error::other("disk full"));
        assert_eq!(library.to_string(), "Something went wrong: disk full");
    }

    #[test]
    fn stored_text_follows_activate_when_serialized_and_display_stays_english() {
        let text = t!("Capture window is unavailable");
        assert_eq!(
            serde_json::to_value(&text).unwrap(),
            "Capture window is unavailable"
        );
        let language = Pseudo::activate();
        assert_eq!(active_tag(), "x-pseudo");
        assert_eq!(serde_json::to_value(&text).unwrap(), "⟦Capture window⟧");
        assert_eq!(serde_json::to_value(ActiveLocale).unwrap(), "x-pseudo");
        assert_eq!(text.to_string(), "Capture window is unavailable");
        drop(language);
        assert_eq!(active_tag(), "en");
    }

    #[test]
    fn a_saved_language_with_no_catalog_falls_through_to_the_os_list() {
        let _restore = Pseudo::activate();
        activate(&Language("ko".into()));
        assert_eq!(active_tag(), "en");
    }

    #[test]
    #[should_panic(expected = "not in packages/ui/src/contract/messages.json")]
    fn check_rejects_a_sentence_outside_messages_json() {
        check("Could not captur the screen: {detail}", &["detail"]);
    }

    #[test]
    #[should_panic(expected = "placeholder with no argument")]
    fn check_rejects_a_missing_argument() {
        check("Could not capture the screen: {detail}", &[]);
    }

    #[test]
    #[should_panic(expected = "placeholder with no argument")]
    fn check_rejects_a_misnamed_argument() {
        check("Could not capture the screen: {detail}", &["detial"]);
    }

    #[test]
    #[should_panic(expected = "not a placeholder")]
    fn check_rejects_an_extra_argument() {
        check("The capture is too large to copy", &["detail"]);
    }

    const SHIPPED: &[&str] = &["de", "en", "es", "fr", "ja", "pt-BR", "zh-Hans"];

    fn negotiated(requested: &[&str], tags: &'static [&str]) -> &'static str {
        tags[negotiate(requested.iter().map(|tag| tag.to_string()), tags)]
    }

    #[test]
    fn negotiation_picks_the_first_requested_language_with_a_compatible_catalog() {
        for (requested, expected) in [
            (&["de-DE"][..], "de"),
            (&["de_DE"], "de"),
            (&["es-419"], "es"),
            (&["fr-CA"], "fr"),
            (&["zh-Hant-TW"], "en"),
            (&["zh-HK"], "en"),
            (&["zh-TW", "ja-JP"], "ja"),
            (&["zh-Hans-CN"], "zh-Hans"),
            (&["zh-CN"], "zh-Hans"),
            (&["zh"], "zh-Hans"),
            (&["zh-Hans-SG"], "zh-Hans"),
            (&["pt-PT"], "pt-BR"),
            (&["pt"], "pt-BR"),
            (&["pt-BR"], "pt-BR"),
            (&["nl-NL", "de-DE"], "de"),
            (&["en-GB", "de-DE"], "en"),
            (&["C"], "en"),
            (&["POSIX"], "en"),
            (&[""], "en"),
            (&[], "en"),
        ] {
            assert_eq!(negotiated(requested, SHIPPED), expected, "{requested:?}");
        }
    }

    #[test]
    fn a_closer_catalog_wins_once_it_exists() {
        const LATER: &[&str] = &["en", "pt", "pt-BR", "pt-PT", "zh-Hans", "zh-Hant"];
        assert_eq!(negotiated(&["pt-PT"], LATER), "pt-PT");
        assert_eq!(negotiated(&["pt-AO"], LATER), "pt");
        assert_eq!(negotiated(&["zh-TW"], LATER), "zh-Hant");
        assert_eq!(negotiated(&["zh-Hant-HK"], LATER), "zh-Hant");
        assert_eq!(negotiated(&["zh-CN"], LATER), "zh-Hans");
    }
}
