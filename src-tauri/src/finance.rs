//! Pure financial calculation layer. It never touches the database: `Store` loads
//! per-month aggregates into a `Context`, and everything below is deterministic.
//!
//! Rules enforced here:
//! * Net = gross - deductions; deductions are never counted as expenses.
//! * Investments and savings are allocations, not consumption.
//! * Transfers between own accounts are ignored; refunds reduce consumption.
//! * Unallocated money is reported as "remaining", never silently treated as spent.

use chrono::{Months, NaiveDate};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

pub const TRACKING_DAYS_GOAL: i64 = 20;
pub const XP_PER_LEVEL: i64 = 500;
const XP_DAY: i64 = 5;
const XP_TRACKING_MONTH: i64 = 30;
const XP_BUDGET: i64 = 20;
const XP_SAVINGS: i64 = 50;
const XP_INVEST: i64 = 50;
const XP_MISSION: i64 = 40;
const TITLES: [&str; 6] = [
    "Rookie Saver",
    "Budget Beginner",
    "Smart Spender",
    "Smart Money Manager",
    "Wealth Builder",
    "Money Pilot",
];

#[derive(Clone, Debug)]
pub struct CategoryInfo {
    pub id: i64,
    pub name: String,
    pub flow: String,
}

#[derive(Clone, Debug, Default)]
pub struct MonthInput {
    /// (gross, deductions) entered through salary setup.
    pub plan: Option<(i64, i64)>,
    /// category id -> total paise recorded that month.
    pub totals: HashMap<i64, i64>,
    /// Distinct days with at least one recorded expense/investment/saving.
    pub tracked_days: i64,
    pub tx_count: i64,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Settings {
    pub budget_warn_pct: i64,
    pub savings_goal_pct: i64,
    pub invest_goal_pct: i64,
}

pub struct Context {
    pub categories: Vec<CategoryInfo>,
    pub months: BTreeMap<String, MonthInput>,
    pub budgets: HashMap<i64, i64>,
    pub settings: Settings,
}

#[derive(Clone, Debug, Serialize)]
pub struct Summary {
    pub month: String,
    /// "setup" (salary setup), "transactions" (derived from entries) or "none".
    pub source: String,
    pub gross: i64,
    pub deductions: i64,
    pub net: i64,
    pub other_income: i64,
    pub refunds: i64,
    /// Money available to allocate: net salary plus other income.
    pub base: i64,
    pub fixed: i64,
    pub variable: i64,
    pub consumed: i64,
    pub investments: i64,
    pub savings: i64,
    pub allocated: i64,
    pub remaining: i64,
    pub utilization_pct: f64,
    pub consumption_pct: f64,
    pub investment_pct: f64,
    pub savings_pct: f64,
    /// healthy | moderate | high | almost_full | over | no_income | empty
    pub state: String,
    pub tx_count: i64,
    pub tracked_days: i64,
}

#[derive(Debug, Serialize)]
pub struct Slice {
    pub key: String,
    pub label: String,
    pub group: String,
    pub amount: i64,
    pub pct: f64,
    pub category_ids: Vec<i64>,
}

#[derive(Debug, Serialize)]
pub struct Stage {
    pub key: String,
    pub label: String,
    pub amount: i64,
    pub pct: f64,
    /// "gross" or "net": what `pct` is measured against.
    pub pct_of: String,
    pub previous: i64,
    pub change: i64,
    pub category_ids: Vec<i64>,
}

#[derive(Debug, Serialize)]
pub struct BudgetRow {
    pub category_id: i64,
    pub name: String,
    pub flow: String,
    pub budget: i64,
    pub spent: i64,
    pub remaining: i64,
    pub pct: f64,
    /// none | ok | warn | over
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CategoryAmount {
    pub category_id: i64,
    pub name: String,
    pub amount: i64,
}

#[derive(Debug, Serialize)]
pub struct Comparison {
    pub has_previous: bool,
    pub consumption_pct_delta: f64,
    pub kept_pct_delta: f64,
    /// improving | steady | declining | unknown
    pub trend: String,
}

#[derive(Debug, Serialize)]
pub struct XpLine {
    pub label: String,
    pub count: i64,
    pub xp: i64,
}

#[derive(Debug, Serialize)]
pub struct Achievement {
    pub key: String,
    pub title: String,
    pub hint: String,
    pub unlocked: bool,
}

#[derive(Debug, Serialize)]
pub struct Mission {
    pub key: String,
    pub title: String,
    pub hint: String,
    /// not_started | in_progress | completed | missed
    pub status: String,
    pub xp: i64,
}

#[derive(Debug, Serialize)]
pub struct Gamification {
    pub xp: i64,
    pub level: i64,
    pub title: String,
    pub xp_into_level: i64,
    pub xp_per_level: i64,
    pub xp_lines: Vec<XpLine>,
    pub achievements: Vec<Achievement>,
    pub missions: Vec<Mission>,
}

#[derive(Debug, Serialize)]
pub struct Dashboard {
    pub month: String,
    pub has_data: bool,
    pub summary: Summary,
    pub previous: Summary,
    pub slices: Vec<Slice>,
    pub top_categories: Vec<Slice>,
    pub flow: Vec<Stage>,
    pub budgets: Vec<BudgetRow>,
    pub investment_items: Vec<CategoryAmount>,
    pub saving_items: Vec<CategoryAmount>,
    pub invest_target: i64,
    pub savings_target: i64,
    pub settings: Settings,
    pub history: Vec<Summary>,
    pub comparison: Comparison,
    pub gamification: Gamification,
}

fn pct(part: i64, whole: i64) -> f64 {
    if whole > 0 {
        part as f64 * 100.0 / whole as f64
    } else {
        0.0
    }
}

pub fn shift_month(month: &str, delta: i32) -> Option<String> {
    let date = NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d").ok()?;
    let shifted = if delta >= 0 {
        date.checked_add_months(Months::new(delta as u32))?
    } else {
        date.checked_sub_months(Months::new(delta.unsigned_abs()))?
    };
    Some(shifted.format("%Y-%m").to_string())
}

fn state_for(utilization: f64, base: i64, allocated: i64) -> &'static str {
    if base <= 0 {
        return if allocated > 0 { "no_income" } else { "empty" };
    }
    if utilization <= 50.0 {
        "healthy"
    } else if utilization <= 75.0 {
        "moderate"
    } else if utilization <= 90.0 {
        "high"
    } else if utilization <= 100.0 {
        "almost_full"
    } else {
        "over"
    }
}

impl Context {
    fn flow_of(&self, id: i64) -> &str {
        self.categories
            .iter()
            .find(|c| c.id == id)
            .map_or("", |c| c.flow.as_str())
    }

    fn ids_with_flow(&self, flows: &[&str]) -> Vec<i64> {
        self.categories
            .iter()
            .filter(|c| flows.contains(&c.flow.as_str()))
            .map(|c| c.id)
            .collect()
    }

    fn id_named(&self, name: &str) -> Option<i64> {
        self.categories.iter().find(|c| c.name == name).map(|c| c.id)
    }

    fn input(&self, month: &str) -> MonthInput {
        self.months.get(month).cloned().unwrap_or_default()
    }

    fn flow_total(&self, input: &MonthInput, flow: &str) -> i64 {
        input
            .totals
            .iter()
            .filter(|(id, _)| self.flow_of(**id) == flow)
            .map(|(_, amount)| *amount)
            .sum()
    }

    pub fn summarize(&self, month: &str) -> Summary {
        let input = self.input(month);
        let (gross, deductions, source) = match input.plan {
            Some((gross, deductions)) => (gross, deductions, "setup"),
            None => {
                let gross = self.flow_total(&input, "salary");
                let deductions = self.flow_total(&input, "deduction");
                let source = if gross > 0 || deductions > 0 {
                    "transactions"
                } else {
                    "none"
                };
                (gross, deductions, source)
            }
        };
        let net = (gross - deductions).max(0);
        let other_income = self.flow_total(&input, "income");
        let refunds = self.flow_total(&input, "refund");
        let base = net + other_income;
        let fixed = self.flow_total(&input, "fixed");
        let variable = self.flow_total(&input, "variable");
        let consumed = (fixed + variable - refunds).max(0);
        let investments = self.flow_total(&input, "investment");
        let savings = self.flow_total(&input, "saving");
        let allocated = consumed + investments + savings;
        let utilization_pct = pct(allocated, base);
        Summary {
            month: month.to_string(),
            source: source.to_string(),
            gross,
            deductions,
            net,
            other_income,
            refunds,
            base,
            fixed,
            variable,
            consumed,
            investments,
            savings,
            allocated,
            remaining: base - allocated,
            utilization_pct,
            consumption_pct: pct(consumed, base),
            investment_pct: pct(investments, base),
            savings_pct: pct(savings, base),
            state: state_for(utilization_pct, base, allocated).to_string(),
            tx_count: input.tx_count,
            tracked_days: input.tracked_days,
        }
    }

    fn slices(&self, month: &str, s: &Summary) -> Vec<Slice> {
        let input = self.input(month);
        let mut out: Vec<Slice> = self
            .categories
            .iter()
            .filter(|c| c.flow == "fixed" || c.flow == "variable")
            .filter_map(|c| {
                let amount = *input.totals.get(&c.id)?;
                (amount > 0).then(|| Slice {
                    key: format!("cat-{}", c.id),
                    label: c.name.clone(),
                    group: c.flow.clone(),
                    amount,
                    pct: pct(amount, s.base),
                    category_ids: vec![c.id],
                })
            })
            .collect();
        out.sort_by(|a, b| b.amount.cmp(&a.amount).then(a.label.cmp(&b.label)));
        for (key, label, flow, amount) in [
            ("investments", "Investments", "investment", s.investments),
            ("savings", "Savings", "saving", s.savings),
        ] {
            if amount > 0 {
                out.push(Slice {
                    key: key.into(),
                    label: label.into(),
                    group: flow.into(),
                    amount,
                    pct: pct(amount, s.base),
                    category_ids: self.ids_with_flow(&[flow]),
                });
            }
        }
        if s.remaining > 0 {
            out.push(Slice {
                key: "remaining".into(),
                label: "Remaining".into(),
                group: "remaining".into(),
                amount: s.remaining,
                pct: pct(s.remaining, s.base),
                category_ids: vec![],
            });
        }
        out
    }

    fn stage(
        &self,
        key: &str,
        label: &str,
        amount: i64,
        previous: i64,
        of_gross: bool,
        s: &Summary,
        flows: &[&str],
    ) -> Stage {
        let whole = if of_gross { s.gross } else { s.base };
        Stage {
            key: key.into(),
            label: label.into(),
            amount,
            pct: pct(amount, whole),
            pct_of: if of_gross { "gross" } else { "net" }.into(),
            previous,
            change: amount - previous,
            category_ids: self.ids_with_flow(flows),
        }
    }

    fn flow_stages(&self, s: &Summary, p: &Summary) -> Vec<Stage> {
        vec![
            self.stage("gross", "Gross salary", s.gross, p.gross, true, s, &["salary"]),
            self.stage("deductions", "Deductions", s.deductions, p.deductions, true, s, &["deduction"]),
            self.stage("net", "Net salary", s.net, p.net, true, s, &["salary"]),
            self.stage("fixed", "Fixed expenses", s.fixed, p.fixed, false, s, &["fixed"]),
            self.stage("variable", "Variable expenses", s.variable, p.variable, false, s, &["variable"]),
            self.stage("investments", "Investments", s.investments, p.investments, false, s, &["investment"]),
            self.stage("savings", "Savings", s.savings, p.savings, false, s, &["saving"]),
            self.stage("remaining", "Remaining balance", s.remaining, p.remaining, false, s, &[]),
        ]
    }

    fn budget_rows(&self, month: &str) -> Vec<BudgetRow> {
        let input = self.input(month);
        let warn = self.settings.budget_warn_pct as f64;
        self.categories
            .iter()
            .filter(|c| c.flow == "fixed" || c.flow == "variable")
            .map(|c| {
                let budget = self.budgets.get(&c.id).copied().unwrap_or(0);
                let spent = input.totals.get(&c.id).copied().unwrap_or(0);
                let used = pct(spent, budget);
                let status = if budget == 0 {
                    "none"
                } else if spent > budget {
                    "over"
                } else if used >= warn {
                    "warn"
                } else {
                    "ok"
                };
                BudgetRow {
                    category_id: c.id,
                    name: c.name.clone(),
                    flow: c.flow.clone(),
                    budget,
                    spent,
                    remaining: budget - spent,
                    pct: used,
                    status: status.into(),
                }
            })
            .collect()
    }

    fn items(&self, month: &str, flow: &str) -> Vec<CategoryAmount> {
        let input = self.input(month);
        let mut items: Vec<CategoryAmount> = self
            .categories
            .iter()
            .filter(|c| c.flow == flow)
            .map(|c| CategoryAmount {
                category_id: c.id,
                name: c.name.clone(),
                amount: input.totals.get(&c.id).copied().unwrap_or(0),
            })
            .collect();
        items.sort_by(|a, b| b.amount.cmp(&a.amount).then(a.name.cmp(&b.name)));
        items
    }

    fn goal_met(&self, s: &Summary, amount: i64, goal_pct: i64) -> bool {
        goal_pct > 0 && s.base > 0 && amount * 100 >= goal_pct * s.base
    }

    fn facts(&self, month: &str, today: &str) -> MonthFacts {
        let s = self.summarize(month);
        let input = self.input(month);
        let mut budgets_set = 0;
        let mut budgets_over = 0;
        let mut budgets_within = 0;
        for (id, budget) in &self.budgets {
            let flow = self.flow_of(*id);
            if flow != "fixed" && flow != "variable" {
                continue;
            }
            budgets_set += 1;
            let spent = input.totals.get(id).copied().unwrap_or(0);
            if spent > *budget {
                budgets_over += 1;
            } else if spent > 0 {
                budgets_within += 1;
            }
        }
        let savings_met = self.goal_met(&s, s.savings, self.settings.savings_goal_pct);
        let invest_met = self.goal_met(&s, s.investments, self.settings.invest_goal_pct);
        MonthFacts {
            month: month.to_string(),
            closed: month < today,
            has_data: input.tx_count > 0 || input.plan.is_some(),
            budgets_set,
            budgets_over,
            budgets_within,
            savings_met,
            invest_met,
            summary: s,
        }
    }

    fn category_budget(&self, month: &str, names: &[&str]) -> (i64, i64) {
        let input = self.input(month);
        let mut budget = 0;
        let mut spent = 0;
        for name in names {
            if let Some(id) = self.id_named(name) {
                if let Some(b) = self.budgets.get(&id) {
                    budget += b;
                    spent += input.totals.get(&id).copied().unwrap_or(0);
                }
            }
        }
        (budget, spent)
    }

    fn missions(&self, f: &MonthFacts) -> Vec<Mission> {
        let s = &f.summary;
        let budget_mission = |key: &str, title: &str, names: &[&str], what: &str| {
            let (budget, spent) = self.category_budget(&f.month, names);
            let status = if budget == 0 {
                "not_started"
            } else if spent > budget {
                "missed"
            } else if f.closed {
                "completed"
            } else {
                "in_progress"
            };
            Mission {
                key: key.into(),
                title: title.into(),
                hint: if budget == 0 {
                    format!("Set a {what} budget to start this mission")
                } else {
                    format!("Keep {what} spending within its budget until month end")
                },
                status: status.into(),
                xp: XP_MISSION,
            }
        };
        let target_mission = |key: &str, title: &str, hint: &str, amount: i64, met: bool| {
            let status = if met {
                "completed"
            } else if f.closed && f.has_data {
                "missed"
            } else if amount > 0 {
                "in_progress"
            } else {
                "not_started"
            };
            Mission {
                key: key.into(),
                title: title.into(),
                hint: hint.into(),
                status: status.into(),
                xp: XP_MISSION,
            }
        };
        let tracking_status = if s.tracked_days >= TRACKING_DAYS_GOAL {
            "completed"
        } else if f.closed && f.has_data {
            "missed"
        } else if s.tx_count > 0 {
            "in_progress"
        } else {
            "not_started"
        };
        vec![
            budget_mission("food", "Stay within food budget", &["Food & dining", "Groceries"], "food"),
            budget_mission("shopping", "Avoid unnecessary shopping", &["Shopping"], "shopping"),
            Mission {
                key: "tracking".into(),
                title: "Complete all expense entries".into(),
                hint: format!("Record expenses on {TRACKING_DAYS_GOAL} different days"),
                status: tracking_status.into(),
                xp: XP_MISSION,
            },
            target_mission(
                "savings",
                "Save a target amount",
                &format!("Save {}% of your income", self.settings.savings_goal_pct),
                s.savings,
                f.savings_met,
            ),
            target_mission(
                "sip",
                "Complete planned SIP contributions",
                &format!("Invest {}% of your income", self.settings.invest_goal_pct),
                s.investments,
                f.invest_met,
            ),
        ]
    }

    fn gamification(&self, selected: &str, today: &str) -> Gamification {
        let all: Vec<MonthFacts> = self
            .months
            .keys()
            .map(|m| self.facts(m, today))
            .filter(|f| f.has_data)
            .collect();
        let days: i64 = all.iter().map(|f| f.summary.tracked_days).sum();
        let tracked_months = all
            .iter()
            .filter(|f| f.closed && f.summary.tracked_days >= TRACKING_DAYS_GOAL)
            .count() as i64;
        let budget_wins: i64 = all.iter().filter(|f| f.closed).map(|f| f.budgets_within).sum();
        let savings_goals = all.iter().filter(|f| f.savings_met).count() as i64;
        let invest_goals = all.iter().filter(|f| f.invest_met).count() as i64;
        let missions_done: i64 = all
            .iter()
            .map(|f| self.missions(f).iter().filter(|m| m.status == "completed").count() as i64)
            .sum();
        let xp_lines = vec![
            XpLine { label: "Days with expenses recorded (+5)".into(), count: days, xp: days * XP_DAY },
            XpLine { label: "Months fully tracked (+30)".into(), count: tracked_months, xp: tracked_months * XP_TRACKING_MONTH },
            XpLine { label: "Budgets kept in finished months (+20)".into(), count: budget_wins, xp: budget_wins * XP_BUDGET },
            XpLine { label: "Savings goals reached (+50)".into(), count: savings_goals, xp: savings_goals * XP_SAVINGS },
            XpLine { label: "Investment targets reached (+50)".into(), count: invest_goals, xp: invest_goals * XP_INVEST },
            XpLine { label: "Missions completed (+40)".into(), count: missions_done, xp: missions_done * XP_MISSION },
        ];
        let xp: i64 = xp_lines.iter().map(|l| l.xp).sum();
        let level = xp / XP_PER_LEVEL + 1;

        let mut streak = 0;
        let mut best_streak = 0;
        let mut previous: Option<&str> = None;
        for f in &all {
            let consecutive = previous.is_some_and(|p| shift_month(p, 1).as_deref() == Some(f.month.as_str()));
            streak = if f.savings_met { if consecutive { streak + 1 } else { 1 } } else { 0 };
            best_streak = best_streak.max(streak);
            previous = Some(&f.month);
        }
        let master = |f: &MonthFacts| {
            f.closed
                && f.summary.tracked_days >= TRACKING_DAYS_GOAL
                && f.budgets_set > 0
                && f.budgets_over == 0
                && f.savings_met
                && f.invest_met
        };
        let achievements = vec![
            ("first_transaction", "First Transaction", "Record your first transaction", all.iter().any(|f| f.summary.tx_count > 0)),
            ("expense_tracker", "Expense Tracker", "Record expenses on 20 different days in a month", all.iter().any(|f| f.summary.tracked_days >= TRACKING_DAYS_GOAL)),
            ("budget_guardian", "Budget Guardian", "Finish a month within every budget you set", all.iter().any(|f| f.closed && f.budgets_set > 0 && f.budgets_over == 0)),
            ("consistent_saver", "Consistent Saver", "Reach your savings goal 3 months in a row", best_streak >= 3),
            ("investment_starter", "Investment Starter", "Record your first investment", all.iter().any(|f| f.summary.investments > 0)),
            ("zero_overspending", "Zero Overspending", "Finish a month with everything allocated within your income", all.iter().any(|f| f.closed && f.summary.base > 0 && f.summary.allocated <= f.summary.base)),
            ("finance_master", "Monthly Finance Master", "Track, stay in budget, save and invest on target in one month", all.iter().any(master)),
        ]
        .into_iter()
        .map(|(key, title, hint, unlocked)| Achievement {
            key: key.into(),
            title: title.into(),
            hint: hint.into(),
            unlocked,
        })
        .collect();

        let selected_facts = self.facts(selected, today);
        Gamification {
            xp,
            level,
            title: TITLES[((level - 1) as usize).min(TITLES.len() - 1)].into(),
            xp_into_level: xp % XP_PER_LEVEL,
            xp_per_level: XP_PER_LEVEL,
            xp_lines,
            achievements,
            missions: self.missions(&selected_facts),
        }
    }

    pub fn dashboard(&self, month: &str, today: &str) -> Dashboard {
        let summary = self.summarize(month);
        let previous = shift_month(month, -1)
            .map(|m| self.summarize(&m))
            .unwrap_or_else(|| self.summarize("0000-00"));
        let history: Vec<Summary> = (0..6)
            .rev()
            .filter_map(|back| shift_month(month, -back))
            .map(|m| self.summarize(&m))
            .collect();
        let slices = self.slices(month, &summary);
        let top_categories = slices
            .iter()
            .filter(|s| s.key.starts_with("cat-"))
            .take(5)
            .map(|s| Slice { key: s.key.clone(), label: s.label.clone(), group: s.group.clone(), amount: s.amount, pct: s.pct, category_ids: s.category_ids.clone() })
            .collect();
        let has_previous = previous.base > 0 || previous.allocated > 0;
        let consumption_pct_delta = summary.consumption_pct - previous.consumption_pct;
        let kept_pct_delta = if summary.base > 0 && previous.base > 0 {
            (100.0 - summary.consumption_pct) - (100.0 - previous.consumption_pct)
        } else {
            0.0
        };
        let trend = if !has_previous || summary.base == 0 {
            "unknown"
        } else if kept_pct_delta > 1.0 {
            "improving"
        } else if kept_pct_delta < -1.0 {
            "declining"
        } else {
            "steady"
        };
        let settings = self.settings;
        Dashboard {
            month: month.to_string(),
            has_data: summary.tx_count > 0 || summary.source == "setup",
            flow: self.flow_stages(&summary, &previous),
            budgets: self.budget_rows(month),
            investment_items: self.items(month, "investment"),
            saving_items: self.items(month, "saving"),
            invest_target: summary.base * settings.invest_goal_pct / 100,
            savings_target: summary.base * settings.savings_goal_pct / 100,
            settings,
            history,
            comparison: Comparison {
                has_previous,
                consumption_pct_delta,
                kept_pct_delta,
                trend: trend.into(),
            },
            gamification: self.gamification(month, today),
            slices,
            top_categories,
            previous,
            summary,
        }
    }
}

struct MonthFacts {
    month: String,
    closed: bool,
    has_data: bool,
    budgets_set: i64,
    budgets_over: i64,
    budgets_within: i64,
    savings_met: bool,
    invest_met: bool,
    summary: Summary,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SALARY: i64 = 1;
    const OTHER: i64 = 2;
    const REFUND: i64 = 3;
    const TDS: i64 = 4;
    const RENT: i64 = 5;
    const FOOD: i64 = 6;
    const SIP: i64 = 7;
    const EMERGENCY: i64 = 8;
    const TRANSFER: i64 = 9;
    const SHOPPING: i64 = 10;

    fn ctx() -> Context {
        let c = |id, name: &str, flow: &str| CategoryInfo { id, name: name.into(), flow: flow.into() };
        Context {
            categories: vec![
                c(SALARY, "Salary", "salary"),
                c(OTHER, "Other income", "income"),
                c(REFUND, "Refund / reversal", "refund"),
                c(TDS, "Income tax / TDS", "deduction"),
                c(RENT, "Rent / Housing", "fixed"),
                c(FOOD, "Food & dining", "variable"),
                c(SIP, "SIP / Mutual funds", "investment"),
                c(EMERGENCY, "Emergency fund", "saving"),
                c(TRANSFER, "Transfer out", "transfer"),
                c(SHOPPING, "Shopping", "variable"),
            ],
            months: BTreeMap::new(),
            budgets: HashMap::new(),
            settings: Settings { budget_warn_pct: 80, savings_goal_pct: 10, invest_goal_pct: 10 },
        }
    }

    fn month(ctx: &mut Context, name: &str, plan: Option<(i64, i64)>, totals: &[(i64, i64)], days: i64) {
        ctx.months.insert(
            name.into(),
            MonthInput {
                plan,
                totals: totals.iter().copied().collect(),
                tracked_days: days,
                tx_count: totals.len() as i64 + plan.is_some() as i64,
            },
        );
    }

    #[test]
    fn net_is_gross_minus_deductions_and_allocations_stay_separate() {
        let mut c = ctx();
        // Gross 1,20,000 less 20,000 deductions = 1,00,000 net.
        month(&mut c, "2026-10", Some((12_000_000, 2_000_000)), &[
            (RENT, 2_000_000), (FOOD, 800_000), (SIP, 2_000_000), (EMERGENCY, 1_000_000),
            (SALARY, 9_900_000), // ignored: salary setup owns income
            (TDS, 500_000),      // ignored: deductions are never expenses
        ], 10);
        let s = c.summarize("2026-10");
        assert_eq!(s.source, "setup");
        assert_eq!((s.gross, s.deductions, s.net), (12_000_000, 2_000_000, 10_000_000));
        assert_eq!((s.fixed, s.variable, s.consumed), (2_000_000, 800_000, 2_800_000));
        assert_eq!((s.investments, s.savings), (2_000_000, 1_000_000));
        assert_eq!(s.allocated, 5_800_000);
        assert_eq!(s.remaining, 4_200_000);
        assert!((s.utilization_pct - 58.0).abs() < 1e-9);
        assert_eq!(s.state, "moderate");
        assert!((s.consumption_pct - 28.0).abs() < 1e-9);
    }

    #[test]
    fn without_setup_income_and_deduction_entries_are_used() {
        let mut c = ctx();
        month(&mut c, "2026-10", None, &[(SALARY, 10_000_000), (TDS, 1_000_000), (FOOD, 500_000)], 3);
        let s = c.summarize("2026-10");
        assert_eq!(s.source, "transactions");
        assert_eq!((s.gross, s.deductions, s.net), (10_000_000, 1_000_000, 9_000_000));
        assert_eq!(s.consumed, 500_000);
        assert_eq!(c.summarize("2026-09").state, "empty");
    }

    #[test]
    fn refunds_reduce_consumption_and_transfers_are_ignored() {
        let mut c = ctx();
        month(&mut c, "2026-10", Some((10_000_000, 0)), &[
            (FOOD, 1_000_000), (REFUND, 300_000), (TRANSFER, 5_000_000), (OTHER, 200_000),
        ], 2);
        let s = c.summarize("2026-10");
        assert_eq!(s.consumed, 700_000);
        assert_eq!(s.allocated, 700_000, "transfers must not count as spending");
        assert_eq!(s.other_income, 200_000);
        assert_eq!(s.base, 10_200_000, "refunds are not income");
    }

    #[test]
    fn utilization_states_follow_the_bands() {
        for (allocated, state) in [(5_000_000, "healthy"), (7_500_000, "moderate"), (9_000_000, "high"), (10_000_000, "almost_full"), (10_000_100, "over")] {
            let mut c = ctx();
            month(&mut c, "2026-10", Some((10_000_000, 0)), &[(FOOD, allocated)], 1);
            assert_eq!(c.summarize("2026-10").state, state, "{allocated}");
        }
        let mut c = ctx();
        month(&mut c, "2026-10", None, &[(FOOD, 100)], 1);
        let s = c.summarize("2026-10");
        assert_eq!(s.state, "no_income");
        assert_eq!(s.utilization_pct, 0.0);
    }

    #[test]
    fn slices_budgets_and_comparison() {
        let mut c = ctx();
        c.budgets.insert(FOOD, 1_000_000);
        c.budgets.insert(RENT, 2_000_000);
        month(&mut c, "2026-09", Some((10_000_000, 0)), &[(RENT, 2_000_000), (FOOD, 2_000_000)], 5);
        month(&mut c, "2026-10", Some((10_000_000, 0)), &[(RENT, 2_000_000), (FOOD, 900_000), (SIP, 1_000_000)], 5);
        let d = c.dashboard("2026-10", "2026-10");
        let keys: Vec<_> = d.slices.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(keys, ["cat-5", "cat-6", "investments", "remaining"]);
        let total: i64 = d.slices.iter().map(|s| s.amount).sum();
        assert_eq!(total, d.summary.base, "slices must add up to the money available");
        let food = d.budgets.iter().find(|b| b.category_id == FOOD).unwrap();
        assert_eq!((food.status.as_str(), food.remaining), ("warn", 100_000));
        assert_eq!(d.budgets.iter().find(|b| b.category_id == RENT).unwrap().status, "warn");
        assert_eq!(d.budgets.iter().find(|b| b.category_id == SHOPPING).unwrap().status, "none");
        assert_eq!(d.comparison.trend, "improving");
        assert_eq!(d.flow.len(), 8);
        let food_stage = d.flow.iter().find(|s| s.key == "variable").unwrap();
        assert_eq!((food_stage.previous, food_stage.change), (2_000_000, -1_100_000));
        assert_eq!(d.history.len(), 6);
        assert_eq!(d.history[0].month, "2026-05");
        assert_eq!(d.history[5].month, "2026-10");
        assert!(shift_month("2026-13", 1).is_none());
    }

    #[test]
    fn dashboard_json_has_the_fields_the_frontend_reads() {
        let mut c = ctx();
        c.budgets.insert(FOOD, 1_000_000);
        month(&mut c, "2026-09", Some((10_000_000, 1_000_000)), &[(RENT, 2_000_000), (FOOD, 2_500_000), (SIP, 500_000)], 12);
        month(&mut c, "2026-10", Some((12_000_000, 2_000_000)), &[(RENT, 2_000_000), (FOOD, 900_000), (SHOPPING, 700_000), (SIP, 2_000_000), (EMERGENCY, 1_000_000)], 21);
        let json = serde_json::to_value(c.dashboard("2026-10", "2026-10")).unwrap();
        for key in ["month", "has_data", "summary", "previous", "slices", "top_categories", "flow", "budgets", "investment_items", "saving_items", "invest_target", "savings_target", "settings", "history", "comparison", "gamification"] {
            assert!(json.get(key).is_some(), "missing {key}");
        }
        for key in ["gross", "deductions", "net", "base", "consumed", "investments", "savings", "remaining", "utilization_pct", "state", "source"] {
            assert!(json["summary"].get(key).is_some(), "summary missing {key}");
        }
        for key in ["level", "title", "xp", "xp_into_level", "xp_per_level", "xp_lines", "achievements", "missions"] {
            assert!(json["gamification"].get(key).is_some(), "gamification missing {key}");
        }
        assert!(json["summary"]["utilization_pct"].is_f64());
        if let Some(path) = std::env::var_os("PRINT_DASHBOARD_FIXTURE") {
            std::fs::write(path, json.to_string()).unwrap();
        }
    }

    #[test]
    fn xp_missions_and_achievements_come_from_real_data() {
        let mut c = ctx();
        c.budgets.insert(FOOD, 1_000_000);
        for (name, days) in [("2026-07", 22), ("2026-08", 22), ("2026-09", 22)] {
            month(&mut c, name, Some((10_000_000, 0)), &[(RENT, 2_000_000), (FOOD, 800_000), (SIP, 1_000_000), (EMERGENCY, 1_000_000)], days);
        }
        month(&mut c, "2026-10", Some((10_000_000, 0)), &[(FOOD, 100_000)], 2);
        let g = c.dashboard("2026-10", "2026-10").gamification;
        // 68 tracked days x5, 3 tracked months x30, 3 food budgets x20, 3+0 savings goals x50,
        // 3 invest goals x50, 3 months x (food + tracking + savings + sip = 4 missions) x40.
        let expected = 68 * 5 + 3 * 30 + 3 * 20 + 3 * 50 + 3 * 50 + 12 * 40;
        assert_eq!(g.xp, expected);
        assert_eq!(g.level, expected / XP_PER_LEVEL + 1);
        let unlocked: Vec<_> = g.achievements.iter().filter(|a| a.unlocked).map(|a| a.key.as_str()).collect();
        for key in ["first_transaction", "expense_tracker", "budget_guardian", "consistent_saver", "investment_starter", "zero_overspending", "finance_master"] {
            assert!(unlocked.contains(&key), "{key} should be unlocked: {unlocked:?}");
        }
        let food = g.missions.iter().find(|m| m.key == "food").unwrap();
        assert_eq!(food.status, "in_progress", "current month is still open");
        assert_eq!(g.missions.iter().find(|m| m.key == "shopping").unwrap().status, "not_started");
        let empty = ctx().dashboard("2026-10", "2026-10").gamification;
        assert_eq!(empty.xp, 0);
        assert!(empty.achievements.iter().all(|a| !a.unlocked));
    }
}
