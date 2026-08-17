//! Fluent-backed i18n lookups. No user-facing string is ever hardcoded at
//! the call site; every message goes through [`t`] or [`t_args`], reading
//! from `/locales/<lang>.ftl`, embedded at compile time.

use std::collections::HashMap;
use std::sync::OnceLock;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// Locale codes Boldtrace supports, matching `/locales/<code>.ftl`.
pub const SUPPORTED_LANGUAGES: [&str; 6] = ["en", "tr", "fr", "de", "ar", "ru"];

const LOCALE_SOURCES: [(&str, &str); 6] = [
    ("en", include_str!("../../locales/en.ftl")),
    ("tr", include_str!("../../locales/tr.ftl")),
    ("fr", include_str!("../../locales/fr.ftl")),
    ("de", include_str!("../../locales/de.ftl")),
    ("ar", include_str!("../../locales/ar.ftl")),
    ("ru", include_str!("../../locales/ru.ftl")),
];

fn bundles() -> &'static HashMap<&'static str, FluentBundle<FluentResource>> {
    static BUNDLES: OnceLock<HashMap<&'static str, FluentBundle<FluentResource>>> = OnceLock::new();
    BUNDLES.get_or_init(|| {
        LOCALE_SOURCES
            .iter()
            .map(|(code, source)| {
                let lang_id: LanguageIdentifier = code.parse().expect("supported locale codes are valid language ids");
                let resource = FluentResource::try_new(source.to_string())
                    .unwrap_or_else(|(_, errors)| panic!("invalid Fluent syntax in {code}.ftl: {errors:?}"));
                let mut bundle = FluentBundle::new_concurrent(vec![lang_id]);
                bundle
                    .add_resource(resource)
                    .unwrap_or_else(|errors| panic!("duplicate Fluent entries in {code}.ftl: {errors:?}"));
                (*code, bundle)
            })
            .collect()
    })
}

pub fn is_supported(code: &str) -> bool {
    SUPPORTED_LANGUAGES.contains(&code)
}

/// Maps a Telegram client language code (e.g. `en-US`) to a supported
/// Boldtrace locale, falling back to `en` when unset or unsupported.
pub fn normalize_language(code: Option<&str>) -> String {
    let primary = code
        .and_then(|c| c.split(['-', '_']).next())
        .unwrap_or("en")
        .to_lowercase();
    if is_supported(&primary) {
        primary
    } else {
        "en".to_string()
    }
}

/// Looks up `key` in `lang`'s locale file.
pub fn t(lang: &str, key: &str) -> String {
    t_args(lang, key, &[])
}

/// Looks up `key` in `lang`'s locale file, substituting `args`.
pub fn t_args(lang: &str, key: &str, args: &[(&str, String)]) -> String {
    let bundles = bundles();
    let bundle = bundles.get(lang).or_else(|| bundles.get("en")).expect("en locale is always loaded");

    let Some(message) = bundle.get_message(key) else {
        tracing::warn!(lang, key, "missing i18n key");
        return format!("[[{key}]]");
    };
    let Some(pattern) = message.value() else {
        tracing::warn!(lang, key, "i18n key has no value");
        return format!("[[{key}]]");
    };

    let mut fluent_args = FluentArgs::new();
    for (name, value) in args {
        fluent_args.set(*name, FluentValue::from(value.clone()));
    }

    let mut errors = Vec::new();
    let value = bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
    if !errors.is_empty() {
        tracing::warn!(lang, key, ?errors, "fluent formatting errors");
    }
    value.into_owned()
}

/// Appends the mandatory footer (company line + investment-advice
/// disclaimer) to `body`, resolved in `lang`. Every message the bot sends
/// must go through this.
pub fn with_footer(lang: &str, body: &str) -> String {
    format!(
        "{body}\n\n---\n{} {}\n{}",
        t(lang, "footer-company"),
        t(lang, "footer-rights"),
        t(lang, "footer-disclaimer"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_unsupported_and_regional_codes() {
        assert_eq!(normalize_language(Some("en-US")), "en");
        assert_eq!(normalize_language(Some("tr")), "tr");
        assert_eq!(normalize_language(Some("xx")), "en");
        assert_eq!(normalize_language(None), "en");
    }

    #[test]
    fn resolves_a_plain_message_in_every_supported_language() {
        for lang in SUPPORTED_LANGUAGES {
            let message = t(lang, "welcome-greeting");
            assert!(!message.is_empty(), "missing welcome-greeting for {lang}");
            assert!(!message.starts_with("[["), "unresolved key for {lang}: {message}");
        }
    }

    #[test]
    fn resolves_a_message_with_arguments() {
        let message = t_args("en", "language-changed", &[("language", "English".to_string())]);
        assert!(message.contains("English"));
    }

    #[test]
    fn footer_always_carries_the_disclaimer() {
        for lang in SUPPORTED_LANGUAGES {
            let footer = with_footer(lang, "body");
            assert!(!t(lang, "footer-disclaimer").is_empty());
            assert!(footer.contains(&t(lang, "footer-disclaimer")));
        }
    }

    #[test]
    fn missing_key_falls_back_to_a_visible_marker_instead_of_panicking() {
        let message = t("en", "does-not-exist");
        assert_eq!(message, "[[does-not-exist]]");
    }
}
