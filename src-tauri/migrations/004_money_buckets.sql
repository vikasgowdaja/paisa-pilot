ALTER TABLE categories ADD COLUMN bucket TEXT NOT NULL DEFAULT 'expense'
    CHECK(bucket IN ('income','deduction','expense','investment'));
UPDATE categories SET bucket = 'income' WHERE kind = 'income';
INSERT INTO categories(name,kind,bucket) VALUES
('Income tax / TDS','expense','deduction'),
('PF / EPF','expense','deduction'),
('Insurance premium','expense','deduction'),
('Loan EMI','expense','deduction'),
('SIP / Mutual funds','expense','investment'),
('Stocks','expense','investment'),
('FD / RD','expense','investment'),
('Gold','expense','investment'),
('PPF / NPS','expense','investment');
PRAGMA user_version = 4;
