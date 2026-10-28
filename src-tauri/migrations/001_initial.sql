CREATE TABLE accounts (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE CHECK(length(trim(name)) BETWEEN 1 AND 60)
);
CREATE TABLE categories (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    kind TEXT NOT NULL CHECK(kind IN ('income','expense'))
);
CREATE TABLE transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    amount INTEGER NOT NULL CHECK(amount > 0 AND amount <= 100000000000),
    kind TEXT NOT NULL CHECK(kind IN ('income','expense')),
    description TEXT NOT NULL CHECK(length(trim(description)) BETWEEN 1 AND 240),
    category_id INTEGER NOT NULL REFERENCES categories(id),
    account_id INTEGER NOT NULL REFERENCES accounts(id),
    source_id TEXT UNIQUE
);
CREATE INDEX transactions_date ON transactions(date);
INSERT INTO accounts(name) VALUES ('Bank account'), ('Cash');
INSERT INTO categories(name,kind) VALUES
('Salary','income'),('Other income','income'),
('Food & dining','expense'),('Groceries','expense'),('Transport','expense'),
('Shopping','expense'),('Bills & utilities','expense'),('Health','expense'),
('Entertainment','expense'),('Other expense','expense');
PRAGMA user_version = 1;
