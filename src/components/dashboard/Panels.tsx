import { useState } from 'react'
import { ArrowDown, ArrowRight, Award, Check, Flame, Lock, PiggyBank, Rocket, Sparkles, Target, Trophy, TrendingDown, TrendingUp } from 'lucide-react'
import { inr, monthName, paiseToInput, pctText } from '../../lib/finance'
import type { Achievement, BudgetRow, DashboardData, Mission, Stage, Summary } from '../../lib/finance'

export type Call = (command: string, args: Record<string, unknown>, done?: string) => Promise<boolean>

const cleanNumber = (value: string) => value.replace(/[,\s₹]/g, '')
const toPaise = (value: string) => Math.round((parseFloat(cleanNumber(value)) || 0) * 100)

export function SalarySetup({ month, summary, previous, call }: { month: string; summary: Summary; previous: Summary; call: Call }) {
  const fromSetup = summary.source === 'setup'
  const [gross, setGross] = useState(fromSetup ? paiseToInput(summary.gross) : '')
  const [deductions, setDeductions] = useState(fromSetup ? paiseToInput(summary.deductions) : '')
  const net = Math.max(0, toPaise(gross) - toPaise(deductions))
  const lastMonth = previous.source === 'setup'
  return (
    <section className="fd-card fd-setup">
      <header><div><h2>Salary setup · {monthName(month, 'long')}</h2>
        <p>{fromSetup ? 'Your gross salary and deductions for this month.' : summary.source === 'transactions' ? 'Currently derived from your income entries. Enter gross and deductions for an exact picture.' : 'Start your month: enter your gross salary and deductions.'}</p></div>
      {fromSetup && <span className="fd-chip ok"><Check size={12} /> Active</span>}</header>
      <form onSubmit={event => { event.preventDefault(); void call('set_salary', { month, gross: cleanNumber(gross), deductions: cleanNumber(deductions) }, 'Salary saved.') }}>
        <label>Gross salary (₹)<input inputMode="decimal" value={gross} onChange={e => setGross(e.target.value)} placeholder="e.g. 120000" required /></label>
        <label>Total deductions (₹)<input inputMode="decimal" value={deductions} onChange={e => setDeductions(e.target.value)} placeholder="Tax, PF, insurance…" /></label>
        <div className="fd-net"><span>Net salary</span><strong>{inr(net)}</strong><small>Gross − deductions</small></div>
        <div className="fd-actions">
          <button type="submit" className="fd-btn primary">Save salary</button>
          {lastMonth && !fromSetup && <button type="button" className="fd-btn" onClick={() => { setGross(paiseToInput(previous.gross)); setDeductions(paiseToInput(previous.deductions)) }}>Copy last month</button>}
          {fromSetup && <button type="button" className="fd-btn danger" onClick={() => { setGross(''); setDeductions(''); void call('clear_salary', { month }, 'Salary setup removed.') }}>Remove</button>}
        </div>
      </form>
    </section>
  )
}

function BudgetInput({ row, call }: { row: BudgetRow; call: Call }) {
  const initial = paiseToInput(row.budget)
  const [value, setValue] = useState(initial)
  const save = () => { if (value !== initial) void call('set_budget', { categoryId: row.category_id, amount: cleanNumber(value) }) }
  return <input className="fd-budget-input" inputMode="decimal" aria-label={`Monthly budget for ${row.name}`} placeholder="Set budget" value={value}
    onChange={e => setValue(e.target.value)} onBlur={save} onKeyDown={e => { if (e.key === 'Enter') e.currentTarget.blur() }} />
}

export function BudgetPanel({ data, call }: { data: DashboardData; call: Call }) {
  const shown = data.budgets.filter(b => b.budget > 0 || b.spent > 0)
  const rest = data.budgets.filter(b => b.budget === 0 && b.spent === 0)
  const [warn, setWarn] = useState(String(data.settings.budget_warn_pct))
  const [savings, setSavings] = useState(String(data.settings.savings_goal_pct))
  const [invest, setInvest] = useState(String(data.settings.invest_goal_pct))
  const [addId, setAddId] = useState('')
  const [addAmount, setAddAmount] = useState('')
  return (
    <section className="fd-card">
      <header><div><h2>Category budgets</h2><p>Set a monthly limit for each expense category. Warns at {data.settings.budget_warn_pct}%.</p></div></header>
      {shown.length === 0 && <p className="fd-empty-note">No budgets yet. Pick a category below to set your first one.</p>}
      <ul className="fd-budgets">
        {shown.map(row => (
          <li key={row.category_id} className={row.status}>
            <div className="fd-budget-top"><strong>{row.name}</strong><BudgetInput key={`${row.category_id}-${row.budget}`} row={row} call={call} /></div>
            {row.budget > 0 ? (
              <>
                <div className="fd-progress"><i className={row.status} style={{ width: `${Math.min(100, row.pct)}%` }} /></div>
                <div className="fd-budget-meta"><span>{inr(row.spent)} spent</span>
                  <span>{row.remaining >= 0 ? `${inr(row.remaining)} left` : `${inr(-row.remaining)} over`}</span><span className="fd-pct">{pctText(row.pct)}</span></div>
                {row.status === 'warn' && <p className="fd-warning">Close to your limit — {pctText(row.pct)} used.</p>}
                {row.status === 'over' && <p className="fd-warning over">Over budget by {inr(-row.remaining)}.</p>}
              </>
            ) : <div className="fd-budget-meta"><span>{inr(row.spent)} spent · no budget</span></div>}
          </li>
        ))}
      </ul>
      {rest.length > 0 && (
        <form className="fd-add-budget" onSubmit={event => { event.preventDefault(); if (addId && toPaise(addAmount) > 0) { void call('set_budget', { categoryId: Number(addId), amount: cleanNumber(addAmount) }, 'Budget added.'); setAddId(''); setAddAmount('') } }}>
          <select className="fd-select" aria-label="Category to budget" value={addId} onChange={e => setAddId(e.target.value)}>
            <option value="">+ Budget for…</option>
            {rest.map(r => <option key={r.category_id} value={r.category_id}>{r.name}</option>)}
          </select>
          <input className="fd-budget-input" inputMode="decimal" aria-label="Budget amount" placeholder="Amount ₹" value={addAmount} onChange={e => setAddAmount(e.target.value)} />
          <button type="submit" className="fd-btn" disabled={!addId || toPaise(addAmount) <= 0}>Add</button>
        </form>
      )}
      <form className="fd-targets" onSubmit={event => { event.preventDefault(); void call('save_finance_settings', { budgetWarnPct: Number(warn), savingsGoalPct: Number(savings), investGoalPct: Number(invest) }, 'Targets saved.') }}>
        <label>Warn at %<input type="number" min={1} max={100} value={warn} onChange={e => setWarn(e.target.value)} /></label>
        <label>Savings goal %<input type="number" min={0} max={100} value={savings} onChange={e => setSavings(e.target.value)} /></label>
        <label>Invest goal %<input type="number" min={0} max={100} value={invest} onChange={e => setInvest(e.target.value)} /></label>
        <button type="submit" className="fd-btn">Save targets</button>
      </form>
    </section>
  )
}

function Change({ stage }: { stage: Stage }) {
  const costly = stage.key === 'fixed' || stage.key === 'variable' || stage.key === 'deductions'
  if (stage.change === 0) return <small className="fd-change">Same as last month</small>
  const up = stage.change > 0
  const good = costly ? !up : up
  return <small className={`fd-change ${good ? 'good' : 'bad'}`}>{up ? <TrendingUp size={12} /> : <TrendingDown size={12} />}{inr(Math.abs(stage.change))} {up ? 'more' : 'less'} than last month</small>
}

export function FlowPanel({ stages, selectedKey, onSelect }: { stages: Stage[]; selectedKey: string | null; onSelect: (stage: Stage) => void }) {
  return (
    <section className="fd-card">
      <header><div><h2>Your money journey</h2><p>From gross salary to what is left. Tap a stage to see its transactions.</p></div></header>
      <ol className="fd-flow">
        {stages.map((stage, index) => (
          <li key={stage.key}>
            <button type="button" className={selectedKey === `stage-${stage.key}` ? 'selected' : ''} onClick={() => onSelect(stage)}>
              <span className="fd-flow-label">{stage.label}</span>
              <strong className={stage.amount < 0 ? 'neg' : ''}>{inr(stage.amount)}</strong>
              <small>{stage.pct_of === 'gross' ? `${pctText(stage.pct)} of gross` : `${pctText(stage.pct)} of net`}</small>
              <Change stage={stage} />
            </button>
            {index < stages.length - 1 && <span className="fd-flow-arrow" aria-hidden="true"><ArrowRight className="h" size={16} /><ArrowDown className="v" size={16} /></span>}
          </li>
        ))}
      </ol>
    </section>
  )
}

export function LevelCard({ game }: { game: DashboardData['gamification'] }) {
  return (
    <section className="fd-card fd-level">
      <div className="fd-level-badge"><Trophy size={22} /></div>
      <div className="fd-level-body">
        <span className="fd-eyebrow">LEVEL {game.level}</span>
        <h2>{game.title}</h2>
        <div className="fd-progress"><i className="xp" style={{ width: `${(game.xp_into_level / game.xp_per_level) * 100}%` }} /></div>
        <p>{game.xp_into_level.toLocaleString('en-IN')} / {game.xp_per_level.toLocaleString('en-IN')} XP · {game.xp.toLocaleString('en-IN')} XP total</p>
      </div>
    </section>
  )
}

export function XpBreakdown({ game }: { game: DashboardData['gamification'] }) {
  return (
    <ul className="fd-xp">
      {game.xp_lines.map(line => <li key={line.label}><span>{line.label}</span><b>{line.count}</b><strong>+{line.xp} XP</strong></li>)}
    </ul>
  )
}

const MISSION_LABEL: Record<Mission['status'], string> = { not_started: 'Not started', in_progress: 'In progress', completed: 'Completed', missed: 'Missed' }

export function MissionsPanel({ missions }: { missions: Mission[] }) {
  return (
    <section className="fd-card">
      <header><div><h2>Financial missions</h2><p>Monthly challenges. XP is awarded only when they are genuinely completed.</p></div><Target size={18} /></header>
      <ul className="fd-missions">
        {missions.map(m => (
          <li key={m.key} className={m.status}>
            <span className="fd-mission-icon">{m.status === 'completed' ? <Check size={16} /> : <Rocket size={16} />}</span>
            <div><strong>{m.title}</strong><small>{m.hint}</small></div>
            <span className={`fd-chip ${m.status}`}>{MISSION_LABEL[m.status]}</span><b>+{m.xp} XP</b>
          </li>
        ))}
      </ul>
    </section>
  )
}

const ACHIEVEMENT_ICONS: Record<string, typeof Award> = {
  first_transaction: Sparkles, expense_tracker: Award, budget_guardian: Trophy, consistent_saver: PiggyBank,
  investment_starter: TrendingUp, zero_overspending: Target, finance_master: Flame,
}

export function AchievementsPanel({ achievements }: { achievements: Achievement[] }) {
  const unlocked = achievements.filter(a => a.unlocked).length
  return (
    <section className="fd-card">
      <header><div><h2>Achievements</h2><p>{unlocked} of {achievements.length} unlocked from your real activity</p></div><Trophy size={18} /></header>
      <div className="fd-badges">
        {achievements.map(a => {
          const Icon = ACHIEVEMENT_ICONS[a.key] ?? Award
          return (
            <div key={a.key} className={`fd-badge ${a.unlocked ? 'unlocked' : ''}`} title={a.hint}>
              <span>{a.unlocked ? <Icon size={20} /> : <Lock size={16} />}</span>
              <strong>{a.title}</strong><small>{a.unlocked ? 'Unlocked' : a.hint}</small>
            </div>
          )
        })}
      </div>
    </section>
  )
}
