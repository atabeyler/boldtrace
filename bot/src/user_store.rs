//! User persistence. `InMemoryUserStore` is a placeholder for local
//! development and tests; production wires a Supabase-Postgres-backed
//! implementation of the same trait (see phase 6).

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use shared::User;

#[async_trait]
pub trait UserStore: Send + Sync {
    async fn get(&self, telegram_id: i64) -> Option<User>;
    async fn set_language(&self, telegram_id: i64, language: &str);
    async fn record_consent(&self, telegram_id: i64, terms_version: &str, consented_at: i64);
}

#[derive(Default)]
pub struct InMemoryUserStore {
    users: Mutex<HashMap<i64, User>>,
}

#[async_trait]
impl UserStore for InMemoryUserStore {
    async fn get(&self, telegram_id: i64) -> Option<User> {
        self.users.lock().expect("user store lock poisoned").get(&telegram_id).cloned()
    }

    async fn set_language(&self, telegram_id: i64, language: &str) {
        let mut users = self.users.lock().expect("user store lock poisoned");
        users
            .entry(telegram_id)
            .and_modify(|user| user.language = language.to_string())
            .or_insert_with(|| User {
                telegram_id,
                language: language.to_string(),
                consent_given_at: None,
                consent_terms_version: None,
            });
    }

    async fn record_consent(&self, telegram_id: i64, terms_version: &str, consented_at: i64) {
        let mut users = self.users.lock().expect("user store lock poisoned");
        users
            .entry(telegram_id)
            .and_modify(|user| {
                user.consent_given_at = Some(consented_at);
                user.consent_terms_version = Some(terms_version.to_string());
            })
            .or_insert_with(|| User {
                telegram_id,
                language: "en".to_string(),
                consent_given_at: Some(consented_at),
                consent_terms_version: Some(terms_version.to_string()),
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unknown_user_has_no_consent() {
        let store = InMemoryUserStore::default();
        assert!(store.get(1).await.is_none());
    }

    #[tokio::test]
    async fn records_language_and_consent_independently() {
        let store = InMemoryUserStore::default();
        store.set_language(1, "tr").await;
        store.record_consent(1, "v1", 1_000).await;

        let user = store.get(1).await.unwrap();
        assert_eq!(user.language, "tr");
        assert_eq!(user.consent_terms_version.as_deref(), Some("v1"));
        assert_eq!(user.consent_given_at, Some(1_000));
    }

    #[tokio::test]
    async fn re_consenting_updates_version_and_timestamp() {
        let store = InMemoryUserStore::default();
        store.record_consent(1, "v1", 1_000).await;
        store.record_consent(1, "v2", 2_000).await;

        let user = store.get(1).await.unwrap();
        assert_eq!(user.consent_terms_version.as_deref(), Some("v2"));
        assert_eq!(user.consent_given_at, Some(2_000));
    }
}
