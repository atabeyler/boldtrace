-- Registration now requires administrator approval before an account can
-- sign in (BOLDTRACE's "pending/approval UI" option from the auth
-- contract). Existing accounts predate this gate and are grandfathered in
-- as approved so they keep working.
ALTER TABLE web_users ADD COLUMN status TEXT NOT NULL DEFAULT 'pending';
UPDATE web_users SET status = 'approved';
ALTER TABLE web_users ADD CONSTRAINT web_users_status_check CHECK (status IN ('pending', 'approved', 'rejected'));

ALTER TABLE web_users ADD COLUMN is_admin BOOLEAN NOT NULL DEFAULT false;

CREATE INDEX web_users_status_idx ON web_users (status) WHERE status = 'pending';
