-- Login-time location verification: the country an IP-geolocation lookup
-- resolves at login time is compared against the country the account
-- registered with. A temporary override lets an admin let a traveling user
-- back in without re-approving the whole account.
ALTER TABLE web_users ADD COLUMN location_override_until TIMESTAMPTZ NULL;

CREATE TABLE login_location_alerts (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES web_users (id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    expected_country TEXT NOT NULL,
    detected_country TEXT NOT NULL,
    ip TEXT NOT NULL,
    browser_lat DOUBLE PRECISION NULL,
    browser_lon DOUBLE PRECISION NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX login_location_alerts_unresolved_idx ON login_location_alerts (created_at) WHERE resolved = false;
