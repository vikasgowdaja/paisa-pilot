ALTER TABLE gmail_settings ADD COLUMN account_email TEXT NOT NULL DEFAULT '';
PRAGMA user_version = 3;
