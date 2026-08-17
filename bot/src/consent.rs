//! Consent gating. No score/data command runs before the user has
//! accepted the current terms version — checked once per update, not
//! repeated in every handler.

use shared::User;

/// Bump this whenever the terms/disclaimer text changes; existing users
/// with an older accepted version will be asked to re-consent.
pub const CURRENT_TERMS_VERSION: &str = "v1";

/// Whether `user` has consented to the currently active terms version.
pub fn has_current_consent(user: Option<&User>) -> bool {
    matches!(
        user.and_then(|u| u.consent_terms_version.as_deref()),
        Some(version) if version == CURRENT_TERMS_VERSION
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_with_consent(version: Option<&str>) -> User {
        User {
            telegram_id: 1,
            language: "en".to_string(),
            consent_given_at: Some(1),
            consent_terms_version: version.map(str::to_string),
        }
    }

    #[test]
    fn no_user_has_no_consent() {
        assert!(!has_current_consent(None));
    }

    #[test]
    fn stale_terms_version_is_not_current_consent() {
        let user = user_with_consent(Some("v0"));
        assert!(!has_current_consent(Some(&user)));
    }

    #[test]
    fn matching_terms_version_is_current_consent() {
        let user = user_with_consent(Some(CURRENT_TERMS_VERSION));
        assert!(has_current_consent(Some(&user)));
    }
}
