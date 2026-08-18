-- Web product accounts: separate from the Telegram-keyed `users` table,
-- since web sign-in is by email/password rather than a Telegram identity.
CREATE TABLE web_users (
    id TEXT PRIMARY KEY,
    user_code TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    language TEXT NOT NULL DEFAULT 'en',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- One row per consent event, mirroring the Telegram `consents` table, so
-- re-consenting after a terms change keeps the prior acceptance on record.
CREATE TABLE web_consents (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES web_users (id) ON DELETE CASCADE,
    terms_version TEXT NOT NULL,
    consented_at_millis BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX web_consents_user_id_idx ON web_consents (user_id);

-- Server-side sessions backing the HttpOnly session cookie. The token
-- column stores a hash of the bearer value, never the raw cookie, so a
-- database leak alone cannot be used to impersonate a live session.
CREATE TABLE web_sessions (
    token_hash TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES web_users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX web_sessions_user_id_idx ON web_sessions (user_id);
CREATE INDEX web_sessions_expires_at_idx ON web_sessions (expires_at);
