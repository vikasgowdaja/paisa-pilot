export type Flow = 'salary' | 'income' | 'refund' | 'transfer' | 'deduction' | 'fixed' | 'variable' | 'investment' | 'saving'
export type UtilState = 'healthy' | 'moderate' | 'high' | 'almost_full' | 'over' | 'no_income' | 'empty'

export interface Summary {
  month: string; source: 'setup' | 'transactions' | 'none'
  gross: number; deductions: number; net: number; other_income: number; refunds: number; base: number
  fixed: number; variable: number; consumed: number; investments: number; savings: number
  allocated: number; remaining: number
  utilization_pct: number; consumption_pct: number; investment_pct: number; savings_pct: number
  state: UtilState; tx_count: number; tracked_days: number
}
export interface Slice { key: string; label: string; group: string; amount: number; pct: number; category_ids: number[] }
export interface Stage { key: string; label: string; amount: number; pct: number; pct_of: 'gross' | 'net'; previous: number; change: number; category_ids: number[] }
export interface BudgetRow { category_id: number; name: string; flow: Flow; budget: number; spent: number; remaining: number; pct: number; status: 'none' | 'ok' | 'warn' | 'over' }
export interface CategoryAmount { category_id: number; name: string; amount: number }
export interface Settings { budget_warn_pct: number; savings_goal_pct: number; invest_goal_pct: number }
export interface Achievement { key: string; title: string; hint: string; unlocked: boolean }
export interface Mission { key: string; title: string; hint: string; status: 'not_started' | 'in_progress' | 'completed' | 'missed'; xp: number }
export interface Gamification {
  xp: number; level: number; title: string; xp_into_level: number; xp_per_level: number
  xp_lines: { label: string; count: number; xp: number }[]
  achievements: Achievement[]; missions: Mission[]
}
export interface DashboardData {
  month: string; has_data: boolean; summary: Summary; previous: Summary
  slices: Slice[]; top_categories: Slice[]; flow: Stage[]; budgets: BudgetRow[]
  investment_items: CategoryAmount[]; saving_items: CategoryAmount[]
  invest_target: number; savings_target: number; settings: Settings; history: Summary[]
  comparison: { has_previous: boolean; consumption_pct_delta: number; kept_pct_delta: number; trend: 'improving' | 'steady' | 'declining' | 'unknown' }
  gamification: Gamification
}

/** Amounts arrive from Rust as integer paise. */
export function inr(paise: number) {
  const whole = paise % 100 === 0
  return (paise / 100).toLocaleString('en-IN', {
    style: 'currency', currency: 'INR',
    minimumFractionDigits: whole ? 0 : 2, maximumFractionDigits: whole ? 0 : 2,
  })
}
export function pctText(value: number) {
  const rounded = value >= 10 ? Math.round(value) : Math.round(value * 10) / 10
  return `${rounded}%`
}
export function monthName(month: string, style: 'short' | 'long' = 'short') {
  return new Date(`${month}-01T12:00:00`).toLocaleDateString('en-IN', style === 'long' ? { month: 'long', year: 'numeric' } : { month: 'short' })
}
export function previousMonth(month: string) {
  const [y, m] = month.split('-').map(Number)
  const date = new Date(y, m - 2, 1)
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`
}
export function paiseToInput(paise: number) {
  return paise === 0 ? '' : (paise / 100).toFixed(paise % 100 === 0 ? 0 : 2)
}

export const SLICE_COLORS = ['#e0705a', '#d9a441', '#9b7bd0', '#e58bb4', '#6cb4e0', '#c38f5b', '#8aa84a', '#d96b6b', '#5fb3a6', '#b0a0e0']
export const GROUP_COLORS: Record<string, string> = { investment: '#3f86b0', saving: '#22a6a0', remaining: '#2f8f62' }
export function sliceColor(slice: Slice, index: number) {
  return GROUP_COLORS[slice.group] ?? SLICE_COLORS[index % SLICE_COLORS.length]
}
export const STATE_LABELS: Record<UtilState, string> = {
  healthy: 'Healthy spending progress', moderate: 'Moderate utilization', high: 'High utilization',
  almost_full: 'Almost fully allocated', over: 'Overspending warning', no_income: 'No salary recorded', empty: 'Nothing recorded yet',
}
