use serde::{Deserialize, Serialize, Serializer};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

const TAGS: &[&str] = &["en"];
const SYSTEM: &str = "system";

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
    TAGS[active().load(Relaxed)]
}

fn os_locales() -> Vec<String> {
    if cfg!(test) {
        Vec::new()
    } else {
        sys_locale::get_locales().collect()
    }
}

/// Index into `TAGS`. The first read uses the OS list, so text rendered before preferences
/// load already follows the OS. Tests run in parallel, so each thread gets its own language.
fn active() -> &'static AtomicUsize {
    #[cfg(test)]
    {
        thread_local!(static ACTIVE: &'static AtomicUsize = Box::leak(Box::new(AtomicUsize::new(negotiate(os_locales(), TAGS)))));
        ACTIVE.with(|active| *active)
    }
    #[cfg(not(test))]
    {
        static ACTIVE: std::sync::LazyLock<AtomicUsize> =
            std::sync::LazyLock::new(|| AtomicUsize::new(negotiate(os_locales(), TAGS)));
        &ACTIVE
    }
}

pub(crate) fn activate(language: &Language) {
    let saved = (language.0 != SYSTEM).then(|| language.0.clone());
    active().store(
        negotiate(saved.into_iter().chain(os_locales()), TAGS),
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

#[cfg(test)]
mod tests {
    use super::*;

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
