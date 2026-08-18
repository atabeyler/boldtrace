-- Registration now collects a country and a national identity/citizenship
-- number (not Turkey-specific — any country's ID number), and the user
-- chooses their own user_code instead of it being server-generated.
ALTER TABLE web_users ADD COLUMN country TEXT NOT NULL DEFAULT '';
ALTER TABLE web_users ADD COLUMN national_id TEXT NOT NULL DEFAULT '';
