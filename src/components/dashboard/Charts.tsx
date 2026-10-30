import { inr, monthName } from '../../lib/finance'
import type { Summary } from '../../lib/finance'

export function IncomeExpenseBars({ history }: { history: Summary[] }) {
  const max = Math.max(1, ...history.flatMap(m => [m.base, m.consumed]))
  if (history.every(m => m.base === 0 && m.consumed === 0)) {
    return <p className="fd-empty-note">Record income and expenses to see this chart.</p>
  }
  return (
    <div>
      <div className="fd-bars" role="img" aria-label={`Income versus expenses: ${history.map(m => `${monthName(m.month)} income ${inr(m.base)}, expenses ${inr(m.consumed)}`).join('; ')}`}>
        {history.map(m => (
          <div className="fd-bar-col" key={m.month}>
            <div className="fd-bar-pair">
              <i className="income" style={{ height: `${(m.base / max) * 100}%` }} title={`${monthName(m.month)} income ${inr(m.base)}`} />
              <i className="expense" style={{ height: `${(m.consumed / max) * 100}%` }} title={`${monthName(m.month)} expenses ${inr(m.consumed)}`} />
            </div>
            <small>{monthName(m.month)}</small>
          </div>
        ))}
      </div>
      <div className="fd-line-key"><span><i style={{ background: '#2f8f62' }} />Income</span><span><i style={{ background: '#e0705a' }} />Expenses</span></div>
    </div>
  )
}

export function TrendLines({ history }: { history: Summary[] }) {
  const width = 300
  const height = 120
  const pad = 14
  const max = Math.max(1, ...history.flatMap(m => [m.investments, m.savings]))
  if (history.every(m => m.investments === 0 && m.savings === 0)) {
    return <p className="fd-empty-note">Log investments or savings to see their trend.</p>
  }
  const x = (index: number) => pad + (index * (width - pad * 2)) / Math.max(1, history.length - 1)
  const y = (value: number) => height - pad - (value / max) * (height - pad * 2)
  const line = (pick: (m: Summary) => number) => history.map((m, i) => `${x(i)},${y(pick(m))}`).join(' ')
  const series = [
    { name: 'Investments', color: '#3f86b0', pick: (m: Summary) => m.investments },
    { name: 'Savings', color: '#22a6a0', pick: (m: Summary) => m.savings },
  ]
  return (
    <div>
      <svg viewBox={`0 0 ${width} ${height}`} className="fd-lines" role="img"
        aria-label={`Investment and savings trend: ${history.map(m => `${monthName(m.month)} investments ${inr(m.investments)}, savings ${inr(m.savings)}`).join('; ')}`}>
        {[0, 0.5, 1].map(f => <line key={f} x1={pad} x2={width - pad} y1={y(max * f)} y2={y(max * f)} className="fd-grid" />)}
        {series.map(s => (
          <g key={s.name}>
            <polyline points={line(s.pick)} fill="none" stroke={s.color} strokeWidth="2.5" strokeLinejoin="round" strokeLinecap="round" />
            {history.map((m, i) => <circle key={m.month} cx={x(i)} cy={y(s.pick(m))} r="3.5" fill={s.color}><title>{`${s.name} ${monthName(m.month)}: ${inr(s.pick(m))}`}</title></circle>)}
          </g>
        ))}
      </svg>
      <div className="fd-line-axis">{history.map(m => <small key={m.month}>{monthName(m.month)}</small>)}</div>
      <div className="fd-line-key">{series.map(s => <span key={s.name}><i style={{ background: s.color }} />{s.name}</span>)}</div>
    </div>
  )
}
