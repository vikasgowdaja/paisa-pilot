import { useState } from 'react'
import { inr, pctText, sliceColor } from '../../lib/finance'
import type { Slice, Summary } from '../../lib/finance'

interface DonutProps {
  slices: Slice[]
  summary: Summary
  selectedKey: string | null
  onSelect: (slice: Slice) => void
}

export function DonutChart({ slices, summary, selectedKey, onSelect }: DonutProps) {
  const [hover, setHover] = useState<string | null>(null)
  const total = slices.reduce((sum, s) => sum + s.amount, 0)
  const focus = hover ?? selectedKey
  const active = slices.find(s => s.key === focus)
  let offset = 0
  const radius = 46
  const summaryLabel = slices.map(s => `${s.label} ${inr(s.amount)}`).join(', ')

  return (
    <div className="fd-donut-wrap">
      <div className="fd-donut">
        <svg viewBox="0 0 120 120" role="group" aria-label={`Where your money goes: ${summaryLabel || 'nothing recorded'}`}>
          <circle cx="60" cy="60" r={radius} className="fd-donut-track" />
          {total > 0 && slices.map((slice, index) => {
            const length = (slice.amount / total) * 100
            const dash = Math.max(0, length - (slices.length > 1 ? 0.6 : 0))
            const start = offset
            offset += length
            return (
              <circle
                key={slice.key} cx="60" cy="60" r={radius} className={`fd-donut-seg ${focus && focus !== slice.key ? 'dim' : ''}`}
                stroke={sliceColor(slice, index)} pathLength="100" strokeDasharray={`${dash} ${100 - dash}`}
                strokeDashoffset={-start} transform="rotate(-90 60 60)"
                tabIndex={0} role="button"
                aria-label={`${slice.label}: ${inr(slice.amount)}, ${pctText(slice.pct)} of your money. Press to see transactions.`}
                onMouseEnter={() => setHover(slice.key)} onMouseLeave={() => setHover(null)}
                onFocus={() => setHover(slice.key)} onBlur={() => setHover(null)}
                onClick={() => onSelect(slice)}
                onKeyDown={e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onSelect(slice) } }}
              ><title>{`${slice.label} · ${inr(slice.amount)} · ${pctText(slice.pct)}`}</title></circle>
            )
          })}
        </svg>
        <div className="fd-donut-center">
          {active ? (
            <><span className="fd-donut-name">{active.label}</span><strong>{inr(active.amount)}</strong><span>{pctText(active.pct)} of net</span></>
          ) : (
            <><span className="fd-donut-name">Net available</span><strong>{inr(summary.base)}</strong><span>{pctText(summary.utilization_pct)} allocated</span></>
          )}
        </div>
      </div>
      <ul className="fd-legend">
        {slices.map((slice, index) => (
          <li key={slice.key} className={selectedKey === slice.key ? 'selected' : ''}>
            <button type="button" onClick={() => onSelect(slice)} onMouseEnter={() => setHover(slice.key)} onMouseLeave={() => setHover(null)}>
              <i style={{ background: sliceColor(slice, index) }} />
              <span>{slice.label}</span><strong>{inr(slice.amount)}</strong><small>{pctText(slice.pct)}</small>
            </button>
          </li>
        ))}
      </ul>
    </div>
  )
}
