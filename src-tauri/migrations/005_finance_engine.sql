ALTER TABLE categories ADD COLUMN flow TEXT NOT NULL DEFAULT 'variable'
    CHECK(flow IN ('salary','income','refund','transfer','deduction','fixed','variable','investment','saving'));
UPDATE categories SET flow = CASE
    WHEN kind = 'income' AND name = 'Salary' THEN 'salary'
    WHEN kind = 'income' THEN 'income'
    WHEN bucket = 'deduction' THEN 'deduction'
    WHEN bucket = 'investment' THEN 'investment'
    WHEN name = 'Bills & utilities' THEN 'fixed'
    ELSE 'variable' END;
UPDATE categories SET flow = 'fixed' WHERE name IN ('Loan EMI', 'Insurance premium');
INSERT INTO categories(name,kind,bucket,flow) VALUES
('Rent / Housing','expense','expense','fixed'),
('Subscriptions','expense','expense','fixed'),
('Professional tax','expense','deduction','deduction'),
('Insurance (payroll)','expense','deduction','deduction'),
('Other investments','expense','investment','investment'),
('Emergency fund','expense','expense','saving'),
('General savings','expense','expense','saving'),
('Refund / reversal','income','income','refund'),
('Transfer in','income','income','transfer'),
('Transfer out','expense','expense','transfer');

CREATE TABLE salary_plans (
    month TEXT PRIMARY KEY CHECK(length(month) = 7),
    gross INTEGER NOT NULL CHECK(gross > 0 AND gross <= 100000000000),
    deductions INTEGER NOT NULL DEFAULT 0 CHECK(deductions >= 0),
    CHECK(deductions <= gross)
);
CREATE TABLE budgets (
    category_id INTEGER PRIMARY KEY REFERENCES categories(id),
    amount INTEGER NOT NULL CHECK(amount > 0 AND amount <= 100000000000)
);
CREATE TABLE app_settings (
    key TEXT PRIMARY KEY,
    value INTEGER NOT NULL
);
INSERT INTO app_settings(key,value) VALUES
('budget_warn_pct', 80),
('savings_goal_pct', 10),
('invest_goal_pct', 10);
PRAGMA user_version = 5;
