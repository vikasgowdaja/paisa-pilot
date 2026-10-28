use chrono::NaiveDate;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

type Result<T> = std::result::Result<T, String>;
const LIMIT: i64 = 100_000_000_000;

fn db_error(e: impl std::fmt::Display) -> String {
    format!("Database operation failed: {e}")
}

#[derive(Debug, Deserialize)]
pub struct TransactionInput {
    pub id: Option<i64>,
    pub date: String,
    pub amount: String,
    pub kind: String,
    pub description: String,
    pub category_id: i64,
    pub account_id: i64,
}

#[derive(Debug, Serialize)]
pub struct Transaction {
    pub id: i64,
    pub date: String,
    pub amount: i64,
    pub amount_input: String,
    pub amount_display: String,
    pub kind: String,
    pub description: String,
    pub category_id: i64,
    pub account_id: i64,
    pub source_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub balance: String,
}

#[derive(Debug, Serialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub transactions: Vec<Transaction>,
    pub accounts: Vec<Account>,
    pub categories: Vec<Category>,
    pub income: String,
    pub expenses: String,
    pub balance: String,
    pub total_balance: String,
    pub database_path: String,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
}

#[derive(Debug, Deserialize)]
pub struct GmailMessageInput {
    pub message_id: String,
    pub subject: String,
    pub sender: String,
    pub received_at_ms: i64,
    pub transaction_date: String,
    pub amount: Option<String>,
    pub currency: String,
    pub event_type: String,
    pub kind: String,
    pub merchant: String,
    pub account_hint: String,
    pub reference: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct GmailApprovalInput {
    pub candidate_id: i64,
    pub date: String,
    pub amount: String,
    pub kind: String,
    pub description: String,
    pub category_id: i64,
    pub account_id: i64,
}

#[derive(Debug, Serialize)]
pub struct GmailCandidate {
    pub id: i64,
    pub message_id: String,
    pub subject: String,
    pub sender: String,
    pub received_at_ms: i64,
    pub transaction_date: String,
    pub amount: Option<String>,
    pub currency: String,
    pub event_type: String,
    pub kind: String,
    pub merchant: String,
    pub account_hint: String,
    pub reference: String,
    pub description: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct GmailSyncResult {
    pub added: usize,
    pub skipped: usize,
}

pub struct Store {
    conn: Connection,
    path: String,
}

pub fn parse_amount(value: &str) -> Result<i64> {
    let value = value.trim();
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() > 2
        || parts[0].is_empty()
        || !parts[0].bytes().all(|c| c.is_ascii_digit())
        || parts
            .get(1)
            .is_some_and(|p| p.is_empty() || p.len() > 2 || !p.bytes().all(|c| c.is_ascii_digit()))
    {
        return Err("Enter a positive rupee amount with at most two decimal places.".into());
    }
    let rupees = parts[0]
        .parse::<i64>()
        .map_err(|_| "Amount is too large.")?;
    let paise = match parts.get(1) {
        None => 0,
        Some(p) => {
            p.parse::<i64>().map_err(|_| "Invalid paise.")? * if p.len() == 1 { 10 } else { 1 }
        }
    };
    let amount = rupees
        .checked_mul(100)
        .and_then(|v| v.checked_add(paise))
        .ok_or("Amount is too large.")?;
    if !(1..=LIMIT).contains(&amount) {
        return Err("Amount must be between INR 0.01 and INR 1,00,00,00,000.00.".into());
    }
    Ok(amount)
}

fn decimal(amount: i64) -> String {
    format!("{}.{:02}", amount / 100, amount % 100)
}

fn money(amount: i64) -> String {
    let digits = (amount.unsigned_abs() / 100).to_string();
    let n = digits.len();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (n - i == 3 || (n - i > 3 && (n - i - 3) % 2 == 0)) {
            out.push(',');
        }
        out.push(c);
    }
    format!(
        "{}₹{}.{:02}",
        if amount < 0 { "-" } else { "" },
        out,
        amount.unsigned_abs() % 100
    )
}

fn normalize_reference(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path).map_err(db_error)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(db_error)?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")
            .map_err(db_error)?;
        let mut version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(db_error)?;
        if version > 3 {
            return Err("Database was created by a newer PaisaPilot version.".into());
        }
        if version == 0 {
            let tx = conn.transaction().map_err(db_error)?;
            tx.execute_batch(include_str!("../migrations/001_initial.sql"))
                .map_err(db_error)?;
            tx.commit().map_err(db_error)?;
            version = 1;
        }
        if version < 2 {
            let tx = conn.transaction().map_err(db_error)?;
            tx.execute_batch(include_str!("../migrations/002_gmail_import.sql"))
                .map_err(db_error)?;
            tx.commit().map_err(db_error)?;
        }
        if version < 3 {
            let tx = conn.transaction().map_err(db_error)?;
            tx.execute_batch(include_str!("../migrations/003_gmail_account.sql"))
                .map_err(db_error)?;
            tx.commit().map_err(db_error)?;
        }
        Ok(Self {
            conn,
            path: path.display().to_string(),
        })
    }

    pub fn gmail_last_sync_ms(&self) -> Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT last_sync_ms FROM gmail_settings WHERE id=1",
                [],
                |r| r.get(0),
            )
            .map_err(db_error)
    }

    pub fn set_gmail_last_sync_ms(&mut self, synced_at_ms: i64) -> Result<()> {
        self.conn
            .execute(
                "UPDATE gmail_settings SET last_sync_ms=?1 WHERE id=1",
                [synced_at_ms],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn gmail_account_email(&self) -> Result<String> {
        self.conn
            .query_row(
                "SELECT account_email FROM gmail_settings WHERE id=1",
                [],
                |row| row.get(0),
            )
            .map_err(db_error)
    }

    pub fn set_gmail_account_email(&mut self, email: &str) -> Result<()> {
        if email.len() > 254 || (!email.is_empty() && !email.contains('@')) {
            return Err("Google returned an invalid Gmail account address.".into());
        }
        self.conn
            .execute(
                "UPDATE gmail_settings SET account_email=?1 WHERE id=1",
                [email],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn reset_gmail_sync(&mut self) -> Result<()> {
        self.conn
            .execute("UPDATE gmail_settings SET last_sync_ms=NULL WHERE id=1", [])
            .map_err(db_error)?;
        Ok(())
    }

    pub fn record_gmail_message(&mut self, message: &GmailMessageInput) -> Result<bool> {
        if message.message_id.is_empty() || message.message_id.len() > 200 {
            return Err("Gmail returned an invalid message ID.".into());
        }
        let tx = self.conn.transaction().map_err(db_error)?;
        let inserted = tx.execute(
            "INSERT INTO gmail_messages(message_id,subject,sender,received_at_ms) VALUES(?1,?2,?3,?4) ON CONFLICT(message_id) DO NOTHING",
            params![message.message_id, message.subject, message.sender, message.received_at_ms],
        ).map_err(db_error)?;
        if inserted == 0 {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO gmail_candidates(message_id,transaction_date,amount,currency,event_type,kind,merchant,account_hint,reference,description,status)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'pending')",
            params![message.message_id, message.transaction_date, message.amount, message.currency, message.event_type, message.kind,
                message.merchant, message.account_hint, message.reference, message.description],
        ).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(true)
    }

    pub fn gmail_candidates(&self) -> Result<Vec<GmailCandidate>> {
        let mut statement = self.conn.prepare(
            "SELECT c.id,c.message_id,m.subject,m.sender,m.received_at_ms,c.transaction_date,c.amount,c.currency,c.event_type,c.kind,
             c.merchant,c.account_hint,c.reference,c.description,c.status
             FROM gmail_candidates c JOIN gmail_messages m ON m.message_id=c.message_id
             WHERE c.status IN ('pending','duplicate') ORDER BY c.id DESC",
        ).map_err(db_error)?;
        let rows = statement
            .query_map([], |r| {
                Ok(GmailCandidate {
                    id: r.get(0)?,
                    message_id: r.get(1)?,
                    subject: r.get(2)?,
                    sender: r.get(3)?,
                    received_at_ms: r.get(4)?,
                    transaction_date: r.get(5)?,
                    amount: r.get(6)?,
                    currency: r.get(7)?,
                    event_type: r.get(8)?,
                    kind: r.get(9)?,
                    merchant: r.get(10)?,
                    account_hint: r.get(11)?,
                    reference: r.get(12)?,
                    description: r.get(13)?,
                    status: r.get(14)?,
                })
            })
            .map_err(db_error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error)?;
        Ok(rows)
    }

    pub fn approve_gmail_candidate(&mut self, input: &GmailApprovalInput) -> Result<bool> {
        let candidate_result = self.conn.query_row(
            "SELECT message_id,transaction_date,amount,currency,event_type,reference,description
             FROM gmail_candidates WHERE id=?1 AND status='pending'",
            [input.candidate_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        );
        let candidate: (String, String, String, String, String, String, String) =
            match candidate_result {
                Ok(candidate) => candidate,
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    return Err(
                        "This email candidate is no longer available. Refresh and retry.".into(),
                    )
                }
                Err(error) => return Err(db_error(error)),
            };
        let (message_id, _date, _amount, currency, event_type, reference, _description) = candidate;
        if currency != "INR" {
            return Err("Only INR transactions can be added to the current ledger.".into());
        }
        if ["transfer", "card_payment"].contains(&event_type.as_str()) {
            return Err("Transfers and credit-card payments stay out of the income/expense ledger to prevent double-counting.".into());
        }
        let transaction = TransactionInput {
            id: None,
            date: input.date.clone(),
            amount: input.amount.clone(),
            kind: input.kind.clone(),
            description: input.description.clone(),
            category_id: input.category_id,
            account_id: input.account_id,
        };
        let amount = self.validate(&transaction)?;
        let source_id = if reference.trim().is_empty() {
            format!("gmail:{message_id}")
        } else {
            let normalized = normalize_reference(&reference);
            if normalized.is_empty() {
                format!("gmail:{message_id}")
            } else {
                format!("gmail-ref:{normalized}")
            }
        };
        let tx = self.conn.transaction().map_err(db_error)?;
        let existing_source: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM transactions WHERE source_id=?1)",
                [&source_id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        let normalized_reference = normalize_reference(&reference);
        let existing_reference = if normalized_reference.is_empty() {
            false
        } else {
            let mut statement = tx
                .prepare("SELECT source_id,description FROM transactions")
                .map_err(db_error)?;
            let rows = statement
                .query_map([], |r| {
                    Ok((r.get::<_, Option<String>>(0)?, r.get::<_, String>(1)?))
                })
                .map_err(db_error)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(db_error)?
                .iter()
                .any(|(source, description)| {
                    source
                        .as_deref()
                        .is_some_and(|s| normalize_reference(s).contains(&normalized_reference))
                        || normalize_reference(description).contains(&normalized_reference)
                });
            rows
        };
        let conservative_match: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM transactions WHERE date=?1 AND amount=?2 AND kind=?3 AND lower(trim(description))=lower(trim(?4)))",
            params![transaction.date,amount,transaction.kind,transaction.description.trim()], |r| r.get(0),
        ).map_err(db_error)?;
        if existing_source || existing_reference || conservative_match {
            tx.execute(
                "UPDATE gmail_candidates SET status='duplicate' WHERE id=?1",
                [input.candidate_id],
            )
            .map_err(db_error)?;
            tx.commit().map_err(db_error)?;
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO transactions(date,amount,kind,description,category_id,account_id,source_id) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![transaction.date,amount,transaction.kind,transaction.description.trim(),transaction.category_id,transaction.account_id,source_id],
        ).map_err(db_error)?;
        let transaction_id = tx.last_insert_rowid();
        if tx.execute(
            "UPDATE gmail_candidates SET status='imported',account_id=?1,category_id=?2,transaction_id=?3 WHERE id=?4 AND status='pending'",
            params![transaction.account_id,transaction.category_id,transaction_id,input.candidate_id],
        ).map_err(db_error)? != 1 {
            return Err("Email candidate changed during import. Refresh and retry.".into());
        }
        tx.commit().map_err(db_error)?;
        Ok(true)
    }

    pub fn ignore_gmail_candidate(&mut self, candidate_id: i64) -> Result<()> {
        if self.conn.execute(
            "UPDATE gmail_candidates SET status='ignored' WHERE id=?1 AND status IN ('pending','duplicate')",
            [candidate_id],
        ).map_err(db_error)? != 1 {
            return Err("This email candidate is no longer available. Refresh and retry.".into());
        }
        Ok(())
    }

    fn validate(&self, input: &TransactionInput) -> Result<i64> {
        let date = NaiveDate::parse_from_str(&input.date, "%Y-%m-%d")
            .map_err(|_| "Use a valid YYYY-MM-DD date.")?;
        if date.format("%Y-%m-%d").to_string() != input.date
            || !input.date.starts_with(|c: char| c.is_ascii_digit())
            || input.date.len() != 10
        {
            return Err("Use a valid YYYY-MM-DD date.".into());
        }
        if !["income", "expense"].contains(&input.kind.as_str()) {
            return Err("Choose income or expense.".into());
        }
        if !(1..=240).contains(&input.description.trim().chars().count()) {
            return Err("Description must contain 1-240 characters.".into());
        }
        let valid: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM categories WHERE id=?1 AND kind=?2) AND EXISTS(SELECT 1 FROM accounts WHERE id=?3)",
            params![input.category_id, input.kind, input.account_id], |r| r.get(0)).map_err(db_error)?;
        if !valid {
            return Err(
                "Choose an existing account and a category matching the transaction type.".into(),
            );
        }
        parse_amount(&input.amount)
    }

    pub fn save(&mut self, input: &TransactionInput) -> Result<()> {
        let amount = self.validate(input)?;
        let changed = if let Some(id) = input.id {
            self.conn.execute("UPDATE transactions SET date=?1,amount=?2,kind=?3,description=?4,category_id=?5,account_id=?6 WHERE id=?7",
                params![input.date, amount, input.kind, input.description.trim(), input.category_id, input.account_id, id])
        } else {
            self.conn.execute("INSERT INTO transactions(date,amount,kind,description,category_id,account_id) VALUES(?1,?2,?3,?4,?5,?6)",
                params![input.date, amount, input.kind, input.description.trim(), input.category_id, input.account_id])
        }.map_err(db_error)?;
        if changed != 1 {
            return Err("Transaction no longer exists. Refresh and retry.".into());
        }
        Ok(())
    }

    pub fn delete(&mut self, id: i64) -> Result<()> {
        if self
            .conn
            .execute("DELETE FROM transactions WHERE id=?1", [id])
            .map_err(db_error)?
            != 1
        {
            return Err("Transaction no longer exists.".into());
        }
        Ok(())
    }

    pub fn add_account(&mut self, name: &str) -> Result<()> {
        let name = name.trim();
        if !(1..=60).contains(&name.chars().count()) {
            return Err("Account name must contain 1-60 characters.".into());
        }
        let exists: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE name=?1)",
                [name],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if exists {
            return Err("An account with that name already exists.".into());
        }
        self.conn
            .execute("INSERT INTO accounts(name) VALUES(?1)", [name])
            .map_err(db_error)?;
        Ok(())
    }

    pub fn snapshot(&self, month: &str) -> Result<Snapshot> {
        let date = NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d")
            .map_err(|_| "Use a valid YYYY-MM month.")?;
        if date.format("%Y-%m").to_string() != month {
            return Err("Use a valid YYYY-MM month.".into());
        }
        let mut statement = self.conn.prepare("SELECT id,date,amount,kind,description,category_id,account_id,source_id FROM transactions WHERE substr(date,1,7)=?1 ORDER BY date DESC,id DESC").map_err(db_error)?;
        let transactions = statement
            .query_map([month], |r| {
                let amount: i64 = r.get(2)?;
                Ok(Transaction {
                    id: r.get(0)?,
                    date: r.get(1)?,
                    amount,
                    amount_input: decimal(amount),
                    amount_display: money(amount),
                    kind: r.get(3)?,
                    description: r.get(4)?,
                    category_id: r.get(5)?,
                    account_id: r.get(6)?,
                    source_id: r.get(7)?,
                })
            })
            .map_err(db_error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error)?;
        let mut income = 0i64;
        let mut expenses = 0i64;
        for t in &transactions {
            let total = if t.kind == "income" {
                &mut income
            } else {
                &mut expenses
            };
            *total = total
                .checked_add(t.amount)
                .ok_or("Monthly total exceeds supported range.")?;
        }
        let accounts = self.conn.prepare("SELECT a.id,a.name,COALESCE(SUM(CASE WHEN t.kind='income' THEN t.amount ELSE -t.amount END),0) FROM accounts a LEFT JOIN transactions t ON t.account_id=a.id GROUP BY a.id ORDER BY a.id").map_err(db_error)?
            .query_map([], |r| Ok(Account { id:r.get(0)?, name:r.get(1)?, balance:money(r.get(2)?) })).map_err(db_error)?
            .collect::<std::result::Result<Vec<_>,_>>().map_err(db_error)?;
        let total: i64 = self.conn.query_row("SELECT COALESCE(SUM(CASE WHEN kind='income' THEN amount ELSE -amount END),0) FROM transactions", [], |r| r.get(0)).map_err(db_error)?;
        let categories = self
            .conn
            .prepare("SELECT id,name,kind FROM categories ORDER BY id")
            .map_err(db_error)?
            .query_map([], |r| {
                Ok(Category {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    kind: r.get(2)?,
                })
            })
            .map_err(db_error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error)?;
        Ok(Snapshot {
            transactions,
            accounts,
            categories,
            income: money(income),
            expenses: money(expenses),
            balance: money(
                income
                    .checked_sub(expenses)
                    .ok_or("Balance exceeds supported range.")?,
            ),
            total_balance: money(total),
            database_path: self.path.clone(),
        })
    }

    pub fn export_csv(&self) -> Result<String> {
        let mut writer = csv::Writer::from_writer(Vec::new());
        writer
            .write_record([
                "source_id",
                "date",
                "amount",
                "kind",
                "description",
                "category",
                "account",
            ])
            .map_err(db_error)?;
        let mut statement = self.conn.prepare("SELECT COALESCE(t.source_id,'paisapilot:' || lower(hex(randomblob(16)))) ,t.date,t.amount,t.kind,t.description,c.name,a.name,t.id FROM transactions t JOIN categories c ON c.id=t.category_id JOIN accounts a ON a.id=t.account_id ORDER BY t.id").map_err(db_error)?;
        let rows = statement
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, i64>(7)?,
                ))
            })
            .map_err(db_error)?;
        for row in rows {
            let (source, date, amount, kind, description, category, account, id) =
                row.map_err(db_error)?;
            self.conn
                .execute(
                    "UPDATE transactions SET source_id=?1 WHERE id=?2 AND source_id IS NULL",
                    params![source, id],
                )
                .map_err(db_error)?;
            writer
                .write_record([
                    source,
                    date,
                    decimal(amount),
                    kind,
                    description,
                    category,
                    account,
                ])
                .map_err(db_error)?;
        }
        String::from_utf8(writer.into_inner().map_err(db_error)?).map_err(db_error)
    }

    pub fn import_csv(&mut self, content: &str) -> Result<ImportResult> {
        if content.len() > 5_000_000 {
            return Err("CSV must be smaller than 5 MB.".into());
        }
        let mut reader = csv::Reader::from_reader(content.as_bytes());
        let expected = [
            "source_id",
            "date",
            "amount",
            "kind",
            "description",
            "category",
            "account",
        ];
        if reader
            .headers()
            .map_err(|e| format!("Invalid CSV: {e}"))?
            .iter()
            .ne(expected)
        {
            return Err(
                "CSV headers must be: source_id,date,amount,kind,description,category,account"
                    .into(),
            );
        }
        let mut pending = Vec::new();
        for (i, record) in reader.records().enumerate() {
            let record = record.map_err(|e| format!("CSV row {}: {e}", i + 2))?;
            let source = record[0].trim();
            if source.is_empty() || source.len() > 200 {
                return Err(format!(
                    "CSV row {} requires a stable source_id (1-200 bytes).",
                    i + 2
                ));
            }
            let category_id = self
                .conn
                .query_row(
                    "SELECT id FROM categories WHERE name=?1 AND kind=?2",
                    params![&record[5], &record[3]],
                    |r| r.get(0),
                )
                .map_err(|_| format!("CSV row {}: unknown category or mismatched type.", i + 2))?;
            let account_id = self
                .conn
                .query_row("SELECT id FROM accounts WHERE name=?1", [&record[6]], |r| {
                    r.get(0)
                })
                .map_err(|_| format!("CSV row {}: unknown account. Create it first.", i + 2))?;
            let input = TransactionInput {
                id: None,
                date: record[1].into(),
                amount: record[2].into(),
                kind: record[3].into(),
                description: record[4].into(),
                category_id,
                account_id,
            };
            let amount = self
                .validate(&input)
                .map_err(|e| format!("CSV row {}: {e}", i + 2))?;
            pending.push((source.to_string(), input, amount));
        }
        let tx = self.conn.transaction().map_err(db_error)?;
        let mut result = ImportResult {
            imported: 0,
            skipped: 0,
        };
        for (source, input, amount) in pending {
            let changed = tx.execute("INSERT INTO transactions(date,amount,kind,description,category_id,account_id,source_id) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(source_id) DO NOTHING",
                params![input.date,amount,input.kind,input.description.trim(),input.category_id,input.account_id,source]).map_err(db_error)?;
            if changed == 1 {
                result.imported += 1;
            } else {
                result.skipped += 1;
            }
        }
        tx.commit().map_err(db_error)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(amount: &str, kind: &str) -> TransactionInput {
        TransactionInput {
            id: None,
            date: "2026-10-28".into(),
            amount: amount.into(),
            kind: kind.into(),
            description: "Test transaction".into(),
            category_id: if kind == "income" { 1 } else { 3 },
            account_id: 1,
        }
    }
    fn gmail_message(message_id: &str, reference: &str) -> GmailMessageInput {
        GmailMessageInput {
            message_id: message_id.into(),
            subject: "UPI payment".into(),
            sender: "Bank".into(),
            received_at_ms: 0,
            transaction_date: "2026-10-28".into(),
            amount: Some("12.34".into()),
            currency: "INR".into(),
            event_type: "expense".into(),
            kind: "expense".into(),
            merchant: "Local Market".into(),
            account_hint: "1234".into(),
            reference: reference.into(),
            description: "Local Market".into(),
        }
    }

    fn gmail_approval(candidate_id: i64) -> GmailApprovalInput {
        GmailApprovalInput {
            candidate_id,
            date: "2026-10-28".into(),
            amount: "12.34".into(),
            kind: "expense".into(),
            description: "Local Market".into(),
            category_id: 3,
            account_id: 1,
        }
    }
    #[test]
    fn exact_money_and_validation() {
        assert_eq!(parse_amount("0.29").unwrap(), 29);
        assert_eq!(parse_amount("123.4").unwrap(), 12340);
        for invalid in [
            "0",
            "-1",
            "1.001",
            "NaN",
            "1e3",
            "1,000",
            "999999999999999999",
            "1.",
        ] {
            assert!(parse_amount(invalid).is_err(), "{invalid}");
        }
        assert_eq!(money(123456789), "₹12,34,567.89");
        assert_eq!(money(-29), "-₹0.29");
    }
    #[test]
    fn crud_totals_migrations_and_restart_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sqlite3");
        let mut store = Store::open(&path).unwrap();
        store.save(&input("1000.10", "income")).unwrap();
        store.save(&input("0.29", "expense")).unwrap();
        let snapshot = store.snapshot("2026-10").unwrap();
        assert_eq!(snapshot.income, "₹1,000.10");
        assert_eq!(snapshot.expenses, "₹0.29");
        assert_eq!(snapshot.balance, "₹999.81");
        let id = snapshot
            .transactions
            .iter()
            .find(|t| t.kind == "expense")
            .unwrap()
            .id;
        let mut edit = input("10.11", "expense");
        edit.id = Some(id);
        store.save(&edit).unwrap();
        drop(store);
        let mut reopened = Store::open(&path).unwrap();
        assert_eq!(reopened.snapshot("2026-10").unwrap().balance, "₹989.99");
        assert_eq!(reopened.snapshot("2026-09").unwrap().transactions.len(), 0);
        reopened.delete(id).unwrap();
        drop(reopened);
        let reopened = Store::open(&path).unwrap();
        assert_eq!(reopened.snapshot("2026-10").unwrap().transactions.len(), 1);
        assert_eq!(
            reopened.snapshot("2026-10").unwrap().total_balance,
            "₹1,000.10"
        );
    }
    #[test]
    fn import_is_atomic_and_deduplicated() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        let csv = "source_id,date,amount,kind,description,category,account\nbank:1,2026-10-28,12.34,expense,Lunch,Food & dining,Bank account\n";
        assert_eq!(store.import_csv(csv).unwrap().imported, 1);
        assert_eq!(store.import_csv(csv).unwrap().skipped, 1);
        let exported = store.export_csv().unwrap();
        assert_eq!(store.import_csv(&exported).unwrap().skipped, 1);
        let bad = format!("{csv}bank:2,2026-10-28,5,expense,Tea,Food & dining,Bank account\nbank:3,invalid,5,expense,Tea,Food & dining,Bank account\n");
        assert!(store.import_csv(&bad).is_err());
        assert_eq!(store.snapshot("2026-10").unwrap().transactions.len(), 1);
        let mut invalid = input("1", "expense");
        invalid.category_id = 1;
        assert!(store.save(&invalid).is_err());
        invalid.category_id = 3;
        invalid.account_id = 999;
        assert!(store.save(&invalid).is_err());
    }

    #[test]
    fn gmail_message_retries_and_transaction_references_are_idempotent() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        store.set_gmail_last_sync_ms(42).unwrap();
        store.reset_gmail_sync().unwrap();
        assert_eq!(store.gmail_last_sync_ms().unwrap(), None);
        assert!(store
            .record_gmail_message(&gmail_message("message-1", "UPI123456"))
            .unwrap());
        assert!(!store
            .record_gmail_message(&gmail_message("message-1", "UPI123456"))
            .unwrap());
        let candidate = store.gmail_candidates().unwrap().remove(0);
        assert!(store
            .approve_gmail_candidate(&gmail_approval(candidate.id))
            .unwrap());
        assert!(store
            .record_gmail_message(&gmail_message("message-2", "UPI123456"))
            .unwrap());
        let second = store.gmail_candidates().unwrap().remove(0);
        assert!(!store
            .approve_gmail_candidate(&gmail_approval(second.id))
            .unwrap());
        assert_eq!(store.snapshot("2026-10").unwrap().transactions.len(), 1);
        assert_eq!(store.gmail_candidates().unwrap()[0].status, "duplicate");
    }

    #[test]
    fn gmail_import_blocks_conservative_existing_ledger_matches() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        let mut manual = input("12.34", "expense");
        manual.description = "Local Market".into();
        store.save(&manual).unwrap();
        store
            .record_gmail_message(&gmail_message("message-3", ""))
            .unwrap();
        let candidate = store.gmail_candidates().unwrap().remove(0);
        assert!(!store
            .approve_gmail_candidate(&gmail_approval(candidate.id))
            .unwrap());
        assert_eq!(store.snapshot("2026-10").unwrap().transactions.len(), 1);
        assert_eq!(store.gmail_candidates().unwrap()[0].status, "duplicate");
    }

    #[test]
    fn gmail_non_ledger_payment_types_are_not_imported() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        let mut transfer = gmail_message("message-4", "");
        transfer.event_type = "transfer".into();
        store.record_gmail_message(&transfer).unwrap();
        let candidate = store.gmail_candidates().unwrap().remove(0);
        assert!(store
            .approve_gmail_candidate(&gmail_approval(candidate.id))
            .is_err());
        assert_eq!(store.snapshot("2026-10").unwrap().transactions.len(), 0);
    }

    #[test]
    fn gmail_migration_preserves_existing_v1_transactions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.sqlite3");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(include_str!("../migrations/001_initial.sql"))
            .unwrap();
        conn.execute(
            "INSERT INTO transactions(date,amount,kind,description,category_id,account_id) VALUES('2026-10-28',1234,'expense','Existing coffee',3,1)",
            [],
        ).unwrap();
        drop(conn);

        let store = Store::open(&path).unwrap();
        assert_eq!(store.snapshot("2026-10").unwrap().transactions.len(), 1);
        assert_eq!(store.gmail_candidates().unwrap().len(), 0);
        assert_eq!(store.gmail_account_email().unwrap(), "");
    }
}
