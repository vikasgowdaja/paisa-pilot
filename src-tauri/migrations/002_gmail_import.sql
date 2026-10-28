CREATE TABLE gmail_settings (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    last_sync_ms INTEGER
);
INSERT INTO gmail_settings(id) VALUES (1);

CREATE TABLE gmail_messages (
    message_id TEXT PRIMARY KEY,
    subject TEXT NOT NULL,
    sender TEXT NOT NULL,
    received_at_ms INTEGER NOT NULL,
    processed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE gmail_candidates (
    id INTEGER PRIMARY KEY,
    message_id TEXT NOT NULL UNIQUE REFERENCES gmail_messages(message_id),
    transaction_date TEXT NOT NULL,
    amount TEXT,
    currency TEXT NOT NULL,
    event_type TEXT NOT NULL,
    kind TEXT NOT NULL,
    merchant TEXT NOT NULL,
    account_hint TEXT NOT NULL,
    reference TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending','imported','ignored','duplicate')),
    account_id INTEGER REFERENCES accounts(id),
    category_id INTEGER REFERENCES categories(id),
    transaction_id INTEGER REFERENCES transactions(id)
);
CREATE INDEX gmail_candidates_status ON gmail_candidates(status, id);
PRAGMA user_version = 2;
