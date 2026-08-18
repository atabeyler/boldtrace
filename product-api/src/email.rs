//! Transactional email via the Resend HTTP API: notifies the admin of new
//! registrations awaiting approval, and notifies applicants once an admin
//! approves or rejects them. Sending is best-effort — a missing API key or
//! a delivery failure is logged and swallowed rather than failing the
//! request that triggered it, since a registration or an approval decision
//! must not be lost just because a notification email didn't go out.

use serde_json::json;

#[derive(Clone)]
pub struct EmailConfig {
    pub api_key: Option<String>,
    pub admin_email: Option<String>,
    pub from: String,
}

impl EmailConfig {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("RESEND_API_KEY").ok(),
            admin_email: std::env::var("ADMIN_NOTIFICATION_EMAIL").ok(),
            from: std::env::var("EMAIL_FROM").unwrap_or_else(|_| "BOLDTRACE <noreply@boldkimya.com.tr>".into()),
        }
    }
}

async fn send(config: &EmailConfig, to: &str, subject: &str, html: String) {
    let Some(api_key) = config.api_key.as_ref() else {
        tracing::info!(%to, %subject, "RESEND_API_KEY not set, skipping email");
        return;
    };
    let client = reqwest::Client::new();
    let result = client
        .post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        .json(&json!({
            "from": config.from,
            "to": [to],
            "subject": subject,
            "html": html,
        }))
        .send()
        .await;
    match result {
        Ok(res) if res.status().is_success() => {
            tracing::info!(%to, %subject, "notification email sent");
        }
        Ok(res) => {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            tracing::warn!(%to, %subject, %status, %body, "notification email rejected by Resend");
        }
        Err(err) => {
            tracing::warn!(%to, %subject, error=%err, "failed to reach Resend");
        }
    }
}

pub async fn notify_admin_new_registration(
    config: &EmailConfig,
    first_name: &str,
    last_name: &str,
    email: &str,
    user_code: &str,
) {
    let Some(admin_email) = config.admin_email.clone() else {
        tracing::info!("ADMIN_NOTIFICATION_EMAIL not set, skipping admin notification");
        return;
    };
    let html = format!(
        "<p>A new BOLDTRACE account is awaiting approval.</p>\
         <ul>\
         <li><b>Name:</b> {first_name} {last_name}</li>\
         <li><b>Email:</b> {email}</li>\
         <li><b>User code:</b> {user_code}</li>\
         </ul>\
         <p>Review it in the admin panel.</p>"
    );
    send(config, &admin_email, "BOLDTRACE: new account pending approval", html).await;
}

pub async fn notify_applicant_approved(config: &EmailConfig, to: &str, first_name: &str) {
    let html = format!(
        "<p>Hi {first_name},</p>\
         <p>Your BOLDTRACE account has been approved. You can now sign in.</p>"
    );
    send(config, to, "Your BOLDTRACE account has been approved", html).await;
}

pub async fn notify_applicant_rejected(config: &EmailConfig, to: &str, first_name: &str) {
    let html = format!(
        "<p>Hi {first_name},</p>\
         <p>Your BOLDTRACE account request was not approved.</p>"
    );
    send(config, to, "Your BOLDTRACE account request", html).await;
}

#[allow(clippy::too_many_arguments)]
pub async fn notify_admin_location_mismatch(
    config: &EmailConfig,
    first_name: &str,
    last_name: &str,
    email: &str,
    expected_country: &str,
    detected_country: &str,
    ip: &str,
) {
    let Some(admin_email) = config.admin_email.clone() else {
        tracing::info!("ADMIN_NOTIFICATION_EMAIL not set, skipping location alert");
        return;
    };
    let html = format!(
        "<p>A login attempt was blocked because the request's country didn't match \
         the account's registered country.</p>\
         <ul>\
         <li><b>Name:</b> {first_name} {last_name}</li>\
         <li><b>Email:</b> {email}</li>\
         <li><b>Registered country:</b> {expected_country}</li>\
         <li><b>Detected country:</b> {detected_country}</li>\
         <li><b>IP:</b> {ip}</li>\
         </ul>\
         <p>If this is the account holder traveling, allow the login from the admin panel.</p>"
    );
    send(config, &admin_email, "BOLDTRACE: login blocked, country mismatch", html).await;
}
