import { useEffect, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { Award, Flame, Lock, PiggyBank, Rocket, Sparkles, Target, TrendingUp, Trophy } from 'lucide-react'
import './Quest.css'

interface QuestMonth { month: string; income: number; deductions: number; expenses: number; investments: number }
interface QuestCategory { name: string; bucket: string; amount: number }
interface QuestData { current: QuestMonth; categories: QuestCategory[]; history: QuestMonth[]; transaction_count: number }
interface Props { month: string; onMonthChange: (month: string) => void; refreshKey: unknown }

const COLORS = { deduction: '#d9a441', expense: '#e0705a', investment: '#3f86b0', remaining: '#2f8f62' } as const
const TITLES = ['Rookie', 'Saver', 'Planner', 'Strategist', 'Wealth Builder', 'Money Pilot']
const SAVINGS_GOAL = 20
const INVEST_GOAL = 10

function clamp(value: number, min: number, max: number) { return Math.min(max, Math.max(min, value)) }
function share(part: number, whole: number) { return whole > 0 ? (part / whole) * 100 : 0 }
function rupees(paise: number) {
  return (paise / 100).toLocaleString('en-IN', { style: 'currency', currency: 'INR', maximumFractionDigits: 0 })
}
function shortMonth(month: string) {
  return new Date(`${month}-01T12:00:00`).toLocaleDateString('en-IN', { month: 'short' })
}
function errorText(error: unknown) { return error instanceof Error ? error.message : String(error) }

// "kept" is what is left after deductions and lifestyle expenses: savings plus investments.
function stats(m: QuestMonth) {
  const used = m.deductions + m.expenses + m.investments
  const takeHome = m.income - m.deductions
  const kept = m.income - m.deductions - m.expenses
  return {
    used,
    remaining: m.income - used,
    usedPct: m.income > 0 ? share(used, m.income) : used > 0 ? 100 : 0,
    keptPct: share(kept, m.income),
    score: takeHome > 0 ? clamp(share(kept, takeHome), 0, 100) : 0,
  }
}

function tone(usedPct: number) { return usedPct <= 70 ? 'great' : usedPct <= 90 ? 'ok' : usedPct <= 100 ? 'warn' : 'over' }

function Donut({ m }: { m: QuestMonth }) {
  const s = stats(m)
  const parts = [
    { label: 'Deductions', value: m.deductions, color: COLORS.deduction },
    { label: 'Expenses', value: m.expenses, color: COLORS.expense },
    { label: 'Investments', value: m.investments, color: COLORS.investment },
    { label: 'Left over', value: Math.max(0, s.remaining), color: COLORS.remaining },
  ]
  const total = parts.reduce((sum, p) => sum + p.value, 0)
  let offset = 0
  const label = parts.map(p => `${p.label} ${share(p.value, total).toFixed(0)}%`).join(', ')
  return (
    <div className="quest-donut">
      <svg viewBox="0 0 120 120" role="img" aria-label={`Salary split: ${label}`}>
        <circle cx="60" cy="60" r="46" fill="none" stroke="#eef1e8" strokeWidth="16" />
        {total > 0 && parts.map(p => {
          const length = share(p.value, total)
          const circle = (
            <circle key={p.label} cx="60" cy="60" r="46" fill="none" stroke={p.color} strokeWidth="16"
              pathLength="100" strokeDasharray={`${length} ${100 - length}`} strokeDashoffset={-offset}
              transform="rotate(-90 60 60)" />
          )
          offset += length
          return length > 0 ? circle : null
        })}
      </svg>
      <div className={`quest-donut-label ${tone(s.usedPct)}`}>
        <strong>{s.usedPct.toFixed(0)}%</strong>
        <span>of salary used</span>
      </div>
    </div>
  )
}

function Legend({ m }: { m: QuestMonth }) {
  const s = stats(m)
  const rows = [
    { label: 'Deductions', value: m.deductions, color: COLORS.deduction },
    { label: 'Expenses', value: m.expenses, color: COLORS.expense },
    { label: 'Investments', value: m.investments, color: COLORS.investment },
    { label: 'Left over', value: Math.max(0, s.remaining), color: COLORS.remaining },
  ]
  return (
    <ul className="quest-legend">
      {rows.map(r => (
        <li key={r.label}><span className="dot" style={{ background: r.color }} /><span>{r.label}</span>
          <strong>{rupees(r.value)}</strong><small>{share(r.value, Math.max(m.income, s.used)).toFixed(0)}%</small></li>
      ))}
    </ul>
  )
}

function Flow({ m }: { m: QuestMonth }) {
  const s = stats(m)
  const afterDeductions = m.income - m.deductions
  const afterExpenses = afterDeductions - m.expenses
  const rows = [
    { stage: 'Start', label: 'Salary', amount: m.income, left: m.income, color: COLORS.remaining, sign: '' },
    { stage: 'Stage 1', label: 'Deductions', amount: m.deductions, left: afterDeductions, color: COLORS.deduction, sign: '−' },
    { stage: 'Stage 2', label: 'Expenses', amount: m.expenses, left: afterExpenses, color: COLORS.expense, sign: '−' },
    { stage: 'Stage 3', label: 'Investments', amount: m.investments, left: s.remaining, color: COLORS.investment, sign: '−' },
  ]
  return (
    <div className="quest-flow">
      {rows.map(r => (
        <div className="quest-flow-row" key={r.label}>
          <div className="quest-flow-head"><span className="quest-stage">{r.stage}</span><strong>{r.label}</strong>
            <span className="quest-amount">{r.sign}{rupees(r.amount)}</span>
            <small>{r.sign ? `${share(r.amount, m.income).toFixed(0)}% of salary` : '100%'}</small></div>
          <div className="quest-bar" aria-hidden="true"><i style={{ width: `${clamp(share(r.left, m.income), 0, 100)}%`, background: r.color }} /></div>
          <small className="quest-left">{r.sign ? `${rupees(r.left)} left` : 'Your starting point'}</small>
        </div>
      ))}
      <div className={`quest-final ${s.remaining < 0 ? 'over' : ''}`}>
        <Target size={16} /> {s.remaining < 0 ? 'Overspent by' : 'Left to keep'} <strong>{rupees(Math.abs(s.remaining))}</strong>
      </div>
    </div>
  )
}

function History({ history }: { history: QuestMonth[] }) {
  return (
    <div className="quest-history" role="img" aria-label="Monthly salary usage for the last six months">
      {history.map(m => {
        const s = stats(m)
        const whole = Math.max(m.income, s.used)
        const parts = [
          [m.deductions, COLORS.deduction], [m.expenses, COLORS.expense],
          [m.investments, COLORS.investment], [Math.max(0, s.remaining), COLORS.remaining],
        ] as const
        return (
          <div className="quest-history-col" key={m.month}>
            <span className={`quest-history-pct ${m.income > 0 || s.used > 0 ? tone(s.usedPct) : ''}`}>{whole > 0 ? `${s.usedPct.toFixed(0)}%` : '–'}</span>
            <div className="quest-history-stack">
              {parts.map(([value, color], index) => value > 0 && <i key={index} style={{ height: `${share(value, whole)}%`, background: color }} />)}
            </div>
            <small>{shortMonth(m.month)}</small>
          </div>
        )
      })}
    </div>
  )
}

export function Quest({ month, onMonthChange, refreshKey }: Props) {
  const [state, setState] = useState<{ month: string; data?: QuestData; error?: string } | null>(null)

  useEffect(() => {
    let cancelled = false
    const request = isTauri()
      ? invoke<QuestData>('quest_summary', { month })
      : Promise.reject(new Error('Open the PaisaPilot app to use Money Quest. Browser preview is not connected to Rust.'))
    request.then(
      data => { if (!cancelled) setState({ month, data }) },
      error => { if (!cancelled) setState({ month, error: errorText(error) }) },
    )
    return () => { cancelled = true }
  }, [month, refreshKey])

  const picker = (
    <div className="period-row">
      <div><span className="section-label">MONEY QUEST</span><span className="period-note"> Turn your salary into a game you can win</span></div>
      <label className="month-picker"><span className="sr-only">Select month</span>
        <input aria-label="Select month" type="month" value={month} min="0001-01" max="9999-12"
          onChange={e => { if (e.target.value && e.target.value !== month) onMonthChange(e.target.value) }} /></label>
    </div>
  )

  if (!state || state.month !== month) return <>{picker}<p className="quest-note">Loading your quest…</p></>
  if (state.error || !state.data) return <>{picker}<p className="quest-note error" role="alert">{state.error}</p></>

  const { current, categories, history } = state.data
  const s = stats(current)
  const hasIncome = current.income > 0

  let streak = 0
  let index = history.length - 1
  if (history[index].income === 0) index -= 1
  while (index >= 0 && history[index].income > 0 && stats(history[index]).keptPct >= SAVINGS_GOAL) { streak += 1; index -= 1 }

  const xp = Math.round(history.reduce((sum, m) => sum + stats(m).score, 0))
  const level = Math.floor(xp / 100) + 1
  const title = TITLES[Math.min(level - 1, TITLES.length - 1)]
  const investPct = share(current.investments, current.income)
  const remainingPct = share(s.remaining, current.income)

  const badges = [
    { title: 'First Step', hint: 'Log your first transaction', icon: Sparkles, unlocked: state.data.transaction_count >= 1 },
    { title: 'Diligent Tracker', hint: 'Log 10 transactions in a month', icon: Award, unlocked: state.data.transaction_count >= 10 },
    { title: 'Budget Boss', hint: 'Use 80% of salary or less', icon: Trophy, unlocked: hasIncome && s.usedPct <= 80 },
    { title: 'Investor', hint: `Invest ${INVEST_GOAL}% of salary`, icon: TrendingUp, unlocked: hasIncome && investPct >= INVEST_GOAL },
    { title: 'Super Saver', hint: `Keep ${SAVINGS_GOAL}% of salary`, icon: PiggyBank, unlocked: hasIncome && remainingPct >= SAVINGS_GOAL },
    { title: 'Hot Streak', hint: `Keep ${SAVINGS_GOAL}% for 3 months running`, icon: Flame, unlocked: streak >= 3 },
  ]

  let tip = 'Great month! Keep it up to extend your streak.'
  if (!hasIncome) tip = 'Add your salary as an Income transaction to begin your quest.'
  else if (s.remaining < 0) tip = `You are ${rupees(-s.remaining)} over your salary. Pause non-essential spending to recover.`
  else if (investPct < INVEST_GOAL) tip = `Invest ${rupees((INVEST_GOAL / 100) * current.income - current.investments)} more to unlock Investor.`
  else if (remainingPct < SAVINGS_GOAL) tip = `Keep ${rupees((SAVINGS_GOAL / 100) * current.income - s.remaining)} more to unlock Super Saver.`

  const headline = !hasIncome ? 'Your quest starts with your salary.'
    : s.usedPct > 100 ? `Overspent by ${rupees(-s.remaining)}. Time to reset.`
    : s.usedPct > 90 ? `${s.usedPct.toFixed(0)}% of your salary is used. Almost out!`
    : s.usedPct > 70 ? `${s.usedPct.toFixed(0)}% of your salary is used. Stay mindful.`
    : `Only ${s.usedPct.toFixed(0)}% used — you are in the green.`
  const topCategories = categories.slice(0, 6)
  const colorFor = (bucket: string) => bucket === 'deduction' ? COLORS.deduction : bucket === 'investment' ? COLORS.investment : COLORS.expense

  return (
    <>
      {picker}
      <section className={`panel quest-hero ${hasIncome ? tone(s.usedPct) : ''}`}>
        <div className="quest-hero-text">
          <div className="eyebrow">SALARY METER</div>
          <h2>{headline}</h2>
          <div className="quest-meter" role="progressbar" aria-label="Salary used" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(clamp(s.usedPct, 0, 100))}>
            <i style={{ width: `${clamp(s.usedPct, 0, 100)}%` }} />
            <b style={{ left: '70%' }} /><b style={{ left: '90%' }} />
          </div>
          <div className="quest-meter-scale"><span>0%</span><span>70% safe</span><span>90% danger</span><span>100%</span></div>
          <p className="quest-tip"><Rocket size={14} /> {tip}</p>
        </div>
        <div className="quest-hero-chart"><Donut m={current} /><Legend m={current} /></div>
      </section>

      <div className="quest-grid">
        <section className="panel quest-card">
          <div className="panel-heading"><div><h2>Salary journey</h2><p>From salary to what is left, stage by stage</p></div></div>
          <Flow m={current} />
        </section>

        <section className="panel quest-card">
          <div className="panel-heading"><div><h2>Level {level} · {title}</h2><p>Score {Math.round(s.score)}/100 this month · {xp} XP in 6 months</p></div><Trophy size={20} /></div>
          <div className="quest-level">
            <div className="quest-bar" aria-label={`Level progress ${xp % 100}%`}><i style={{ width: `${xp % 100}%`, background: COLORS.investment }} /></div>
            <small>{100 - (xp % 100)} XP to level {level + 1}</small>
            <div className="quest-streak"><Flame size={18} /><strong>{streak}</strong><span>{streak === 1 ? 'month' : 'months'} keeping {SAVINGS_GOAL}%+ of salary</span></div>
          </div>
          <div className="quest-badges">
            {badges.map(b => (
              <div key={b.title} className={`quest-badge ${b.unlocked ? 'unlocked' : ''}`} title={b.hint}>
                <span>{b.unlocked ? <b.icon size={18} /> : <Lock size={16} />}</span><strong>{b.title}</strong><small>{b.unlocked ? 'Unlocked' : b.hint}</small>
              </div>
            ))}
          </div>
        </section>

        <section className="panel quest-card">
          <div className="panel-heading"><div><h2>Last 6 months</h2><p>Share of salary used each month</p></div></div>
          <History history={history} />
        </section>

        <section className="panel quest-card">
          <div className="panel-heading"><div><h2>Where it went</h2><p>Top categories as a share of salary</p></div></div>
          {topCategories.length === 0 ? <p className="quest-note">No spending logged this month yet.</p> : (
            <div className="quest-cats">
              {topCategories.map(c => (
                <div key={c.name}><div className="quest-cat-head"><span>{c.name}</span><strong>{rupees(c.amount)}</strong>
                  <small>{hasIncome ? `${share(c.amount, current.income).toFixed(0)}%` : ''}</small></div>
                <div className="quest-bar" aria-hidden="true"><i style={{ width: `${clamp(hasIncome ? share(c.amount, current.income) : 0, 0, 100)}%`, background: colorFor(c.bucket) }} /></div></div>
              ))}
            </div>
          )}
        </section>
      </div>
    </>
  )
}
