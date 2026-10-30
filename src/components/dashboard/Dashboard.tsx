import { useEffect, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { Moon, PiggyBank, Plus, Sun, TrendingDown, TrendingUp, Wallet, X } from 'lucide-react'
import { STATE_LABELS, inr, monthName, pctText } from '../../lib/finance'
import type { DashboardData, Slice, Stage } from '../../lib/finance'
import { DonutChart } from './DonutChart'
import { IncomeExpenseBars, TrendLines } from './Charts'
import { AchievementsPanel, BudgetPanel, FlowPanel, LevelCard, MissionsPanel, SalarySetup, XpBreakdown } from './Panels'
import type { Call } from './Panels'
import './Dashboard.css'

interface Tx { id: number; date: string; amount_display: string; kind: 'income' | 'expense'; description: string; category_id: number }
interface Cat { id: number; name: string }
interface Props {
  month: string
  onMonthChange: (month: string) => void
  refreshKey: unknown
  transactions: Tx[] | null
  categories: Cat[]
  onAddTransaction: () => void
}
interface Selection { month: string; key: string; label: string; ids: number[] }

function errorText(error: unknown) { return error instanceof Error ? error.message : String(error) }
function greeting() {
  const hour = new Date().getHours()
  return hour < 12 ? 'Good morning' : hour < 17 ? 'Good afternoon' : 'Good evening'
}

export function Dashboard({ month, onMonthChange, refreshKey, transactions, categories, onAddTransaction }: Props) {
  const [state, setState] = useState<{ month: string; data?: DashboardData; error?: string } | null>(null)
  const [version, setVersion] = useState(0)
  const [selection, setSelection] = useState<Selection | null>(null)
  const [notice, setNotice] = useState('')
  const [problem, setProblem] = useState('')
  const [theme, setTheme] = useState<'dark' | 'light'>(() => (localStorage.getItem('paisapilot-theme') === 'light' ? 'light' : 'dark'))

  useEffect(() => {
    let cancelled = false
    const request = isTauri()
      ? invoke<DashboardData>('finance_dashboard', { month })
      : Promise.reject(new Error('Open the PaisaPilot app to use the dashboard. Browser preview is not connected to Rust.'))
    request.then(
      data => { if (!cancelled) setState({ month, data }) },
      error => { if (!cancelled) setState({ month, error: errorText(error) }) },
    )
    return () => { cancelled = true }
  }, [month, version, refreshKey])

  const call: Call = async (command, args, done) => {
    try {
      await invoke(command, args)
      setProblem(''); setNotice(done ?? 'Saved.'); setVersion(v => v + 1)
      return true
    } catch (error) { setNotice(''); setProblem(errorText(error)); return false }
  }
  function toggleTheme() {
    const next = theme === 'dark' ? 'light' : 'dark'
    setTheme(next); localStorage.setItem('paisapilot-theme', next)
  }

  const header = (
    <div className="fd-top">
      <div><span className="fd-eyebrow">{greeting().toUpperCase()}</span><h1>{monthName(month, 'long')}</h1></div>
      <div className="fd-top-actions">
        <label className="fd-month"><span className="sr-only">Select month</span>
          <input aria-label="Select month" type="month" value={month} min="0001-01" max="9999-12" onChange={e => { if (e.target.value && e.target.value !== month) onMonthChange(e.target.value) }} /></label>
        <button type="button" className="fd-btn icon" onClick={toggleTheme} aria-label={`Switch to ${theme === 'dark' ? 'light' : 'dark'} theme`}>{theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}</button>
        <button type="button" className="fd-btn primary" onClick={onAddTransaction}><Plus size={16} /> Add transaction</button>
      </div>
    </div>
  )

  let body
  if (!state || state.month !== month) body = <p className="fd-empty-note">Loading your dashboard…</p>
  else if (state.error || !state.data) body = <p className="fd-empty-note error" role="alert">{state.error}</p>
  else {
    const data = state.data
    const s = data.summary
    const sel = selection && selection.month === month ? selection : null
    const select = (key: string, label: string, ids: number[]) => setSelection(sel?.key === key ? null : { month, key, label, ids })
    const selectedTx = sel && transactions ? transactions.filter(t => sel.ids.includes(t.category_id)) : []
    const recent = (transactions ?? []).slice(0, 6)
    const categoryName = (id: number) => categories.find(c => c.id === id)?.name ?? ''
    const over = s.remaining < 0
    const utilPct = Math.round(s.utilization_pct)
    const noIncome = s.state === 'no_income' || s.state === 'empty'
    const cmp = data.comparison
    const metrics = [
      { label: 'Gross salary', value: s.gross, tone: '' },
      { label: 'Deductions', value: s.deductions, tone: 'warn' },
      { label: 'Net salary', value: s.net, tone: 'good' },
      { label: 'Expenses', value: s.consumed, tone: 'bad' },
      { label: 'Investments', value: s.investments, tone: 'blue' },
      { label: 'Savings', value: s.savings, tone: 'teal' },
      { label: 'Remaining', value: s.remaining, tone: over ? 'bad' : 'good' },
    ]
    body = (
      <>
        {notice && <p className="fd-toast" role="status">{notice}</p>}
        {problem && <p className="fd-toast error" role="alert">{problem}</p>}

        <div className="fd-overview">
          <section className="fd-metrics" aria-label="Salary overview">
            {metrics.map(m => <div key={m.label} className={`fd-metric ${m.tone}`}><span>{m.label}</span><strong>{inr(m.value)}</strong></div>)}
          </section>
          <LevelCard game={data.gamification} />
        </div>

        <div className="fd-grid-2">
          <section className={`fd-card fd-util ${s.state}`}>
            <span className="fd-eyebrow">SALARY UTILIZATION</span>
            {noIncome ? (
              <><h2 className="fd-util-title">{s.state === 'empty' ? 'Nothing recorded yet' : 'No salary recorded'}</h2>
                <p className="fd-muted">{s.state === 'empty' ? 'Enter your salary and add transactions to see how much of it you use.' : 'Spending was recorded but there is no salary. Enter your salary to see utilization.'}</p></>
            ) : (
              <>
                <h2 className="fd-util-title"><span>{utilPct}%</span> of your salary utilized</h2>
                <div className="fd-meter" role="progressbar" aria-label="Salary utilized" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.min(100, utilPct)}>
                  <i style={{ width: `${Math.min(100, s.utilization_pct)}%` }} />
                  {[50, 75, 90].map(p => <b key={p} style={{ left: `${p}%` }} />)}
                </div>
                <div className="fd-meter-scale"><span>0</span><span>50%</span><span>75%</span><span>90%</span><span>100%</span></div>
                <p className="fd-util-line">{inr(s.allocated)} utilized out of {inr(s.base)}</p>
                <p className={`fd-util-state ${s.state}`}>{STATE_LABELS[s.state]} · {over ? `${inr(-s.remaining)} overspent` : `${inr(s.remaining)} remaining`}</p>
                <div className="fd-pills">
                  <span>Consumption <b>{pctText(s.consumption_pct)}</b></span><span>Investments <b>{pctText(s.investment_pct)}</b></span>
                  <span>Savings <b>{pctText(s.savings_pct)}</b></span><span>Total allocated <b>{pctText(s.utilization_pct)}</b></span>
                </div>
                <p className="fd-muted small">Remaining money is unallocated, not treated as spent.{s.source === 'transactions' && ' Based on your income entries — set up your salary for exact figures.'}</p>
              </>
            )}
          </section>
          <SalarySetup key={`${month}-${s.source}-${s.gross}-${s.deductions}`} month={month} summary={s} previous={data.previous} call={call} />
        </div>

        <div className="fd-grid-2">
          <section className="fd-card">
            <header><div><h2>Where your money goes</h2><p>Tap a segment to see its transactions.</p></div></header>
            {data.slices.length === 0
              ? <p className="fd-empty-note">No spending or allocations this month. <button type="button" className="fd-link" onClick={onAddTransaction}>Add a transaction</button></p>
              : <DonutChart slices={data.slices} summary={s} selectedKey={sel?.key ?? null} onSelect={(slice: Slice) => select(slice.key, slice.label, slice.category_ids)} />}
          </section>
          <div className="fd-stack">
            <section className={`fd-card fd-remaining ${over ? 'over' : ''}`}>
              <Wallet size={18} /><span>{over ? 'Overspent' : 'Remaining balance'}</span><strong>{inr(Math.abs(s.remaining))}</strong>
              <small>= Net {inr(s.base)} − expenses {inr(s.consumed)} − investments {inr(s.investments)} − savings {inr(s.savings)}</small>
            </section>
            {([
              { title: 'Investments', icon: TrendingUp, total: s.investments, pct: s.investment_pct, target: data.invest_target, goal: data.settings.invest_goal_pct, items: data.investment_items, color: 'blue' },
              { title: 'Savings', icon: PiggyBank, total: s.savings, pct: s.savings_pct, target: data.savings_target, goal: data.settings.savings_goal_pct, items: data.saving_items, color: 'teal' },
            ] as const).map(card => (
              <section className="fd-card fd-allocation" key={card.title}>
                <header><div><h2>{card.title} this month</h2><p>{pctText(card.pct)} of net · kept separate from expenses</p></div><card.icon size={18} /></header>
                <strong className="fd-big">{inr(card.total)}</strong>
                {card.goal > 0 && s.base > 0 ? (
                  <><div className="fd-progress"><i className={card.color} style={{ width: `${Math.min(100, card.target > 0 ? (card.total / card.target) * 100 : 0)}%` }} /></div>
                    <small className="fd-muted">{card.total >= card.target ? 'Goal reached' : `${inr(card.target - card.total)} to reach your ${card.goal}% goal`} ({inr(card.target)})</small></>
                ) : <small className="fd-muted">Set a goal % under budget targets to track progress.</small>}
                <ul className="fd-items">{card.items.filter(i => i.amount > 0).map(i => <li key={i.category_id}><span>{i.name}</span><b>{inr(i.amount)}</b></li>)}</ul>
              </section>
            ))}
          </div>
        </div>

        <FlowPanel stages={data.flow} selectedKey={sel?.key ?? null} onSelect={(stage: Stage) => select(`stage-${stage.key}`, stage.label, stage.category_ids)} />

        <div className="fd-grid-2">
          <BudgetPanel key={`${data.settings.budget_warn_pct}-${data.settings.savings_goal_pct}-${data.settings.invest_goal_pct}`} data={data} call={call} />
          <section className="fd-card">
            <header><div><h2>{sel ? sel.label : 'Recent transactions'}</h2><p>{sel ? `${selectedTx.length} transaction${selectedTx.length === 1 ? '' : 's'} in ${monthName(month, 'long')}` : monthName(month, 'long')}</p></div>
              {sel && <button type="button" className="fd-btn icon" onClick={() => setSelection(null)} aria-label="Clear selection"><X size={14} /></button>}</header>
            {transactions === null ? <p className="fd-empty-note">Loading transactions…</p> : (
              <ul className="fd-tx">
                {(sel ? selectedTx : recent).map(t => (
                  <li key={t.id}><div><strong>{t.description}</strong><small>{t.date} · {categoryName(t.category_id)}</small></div><b className={t.kind}>{t.kind === 'income' ? '+' : '−'}{t.amount_display}</b></li>
                ))}
                {(sel ? selectedTx : recent).length === 0 && <li className="fd-empty-note">{sel ? (sel.ids.length === 0 ? 'Remaining money is unallocated, so it has no transactions.' : 'No transactions here this month. Salary and deductions come from your salary setup.') : 'No transactions this month yet.'}</li>}
              </ul>
            )}
          </section>
        </div>

        <section className="fd-card">
          <header><div><h2>Monthly performance</h2><p>Compared with {monthName(data.previous.month, 'long')} and the last six months</p></div></header>
          <p className={`fd-verdict ${cmp.trend}`}>
            {cmp.trend === 'improving' && <><TrendingUp size={16} /> Improving — you kept {pctText(Math.abs(cmp.kept_pct_delta))} more of your income than last month.</>}
            {cmp.trend === 'declining' && <><TrendingDown size={16} /> Slipping — you kept {pctText(Math.abs(cmp.kept_pct_delta))} less of your income than last month.</>}
            {cmp.trend === 'steady' && <>Steady — about the same share of income kept as last month.</>}
            {cmp.trend === 'unknown' && <>Add income and expenses for last month and this month to see if you are improving.</>}
          </p>
          <div className="fd-compare">
            {[
              ['Expenses', s.consumed, data.previous.consumed, true], ['Investments', s.investments, data.previous.investments, false],
              ['Savings', s.savings, data.previous.savings, false], ['Remaining', s.remaining, data.previous.remaining, false],
            ].map(([label, now, before, costly]) => {
              const diff = (now as number) - (before as number)
              return <div key={label as string}><span>{label as string}</span><strong>{inr(now as number)}</strong>
                <small className={diff === 0 ? '' : (diff > 0) !== (costly as boolean) ? 'good' : 'bad'}>{diff === 0 ? 'No change' : `${diff > 0 ? '▲' : '▼'} ${inr(Math.abs(diff))}`}</small></div>
            })}
          </div>
          <div className="fd-grid-3">
            <div><h3>Income vs expenses</h3><IncomeExpenseBars history={data.history} /></div>
            <div><h3>Savings &amp; investment trend</h3><TrendLines history={data.history} /></div>
            <div><h3>Highest spending</h3>
              {data.top_categories.length === 0 ? <p className="fd-empty-note">No spending yet.</p> : (
                <ul className="fd-top-cats">{data.top_categories.map(c => (
                  <li key={c.key}><div><span>{c.label}</span><b>{inr(c.amount)}</b></div><div className="fd-progress"><i style={{ width: `${Math.min(100, c.pct)}%` }} /></div></li>))}</ul>)}
            </div>
          </div>
        </section>

        <div className="fd-grid-2">
          <MissionsPanel missions={data.gamification.missions} />
          <AchievementsPanel achievements={data.gamification.achievements} />
        </div>
        <section className="fd-card"><header><div><h2>How you earned XP</h2><p>Every point comes from recorded activity, so it cannot be gamed with clicks.</p></div></header><XpBreakdown game={data.gamification} /></section>
      </>
    )
  }

  return <div className="fd" data-theme={theme}>{header}{body}</div>
}
