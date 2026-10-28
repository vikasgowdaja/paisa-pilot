import { useCallback, useEffect, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { Mail, RefreshCw, ShieldCheck } from 'lucide-react'
import { Button } from './ui/button'

interface Account { id: number; name: string }
interface Category { id: number; name: string; kind: 'income' | 'expense' }
interface Candidate {
  id: number; message_id: string; subject: string; sender: string; received_at_ms: number
  transaction_date: string; amount: string | null; currency: string; event_type: string
  kind: string; merchant: string; account_hint: string; reference: string
  description: string; status: string
}
interface Draft { date: string; amount: string; kind: '' | 'income' | 'expense'; description: string; category_id: string; account_id: string }
interface Props { accounts: Account[]; categories: Category[] }

function message(error: unknown) { return error instanceof Error ? error.message : String(error) }
function asDraft(candidate: Candidate, accounts: Account[], categories: Category[]): Draft {
  const kind = candidate.kind === 'income' || candidate.kind === 'expense' ? candidate.kind : ''
  return {
    date: candidate.transaction_date,
    amount: candidate.amount ?? '',
    kind,
    description: candidate.description,
    category_id: String(categories.find(category => category.kind === kind)?.id ?? ''),
    account_id: String(accounts[0]?.id ?? ''),
  }
}

export function GmailReview({ accounts, categories }: Props) {
  const [candidates, setCandidates] = useState<Candidate[]>([])
  const [drafts, setDrafts] = useState<Record<number, Draft>>({})
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const refresh = useCallback(async () => {
    try {
      const rows = await invoke<Candidate[]>('gmail_candidates')
      setCandidates(rows)
      setDrafts(current => {
        const next = { ...current }
        for (const candidate of rows) next[candidate.id] ??= asDraft(candidate, accounts, categories)
        return next
      })
      setError('')
      return true
    } catch (e) { setError(message(e)); return false }
  }, [accounts, categories])

  useEffect(() => {
    // The state update follows an asynchronous desktop query.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    if (isTauri()) void refresh()
  }, [refresh])

  async function sync() {
    if (busy) return
    setBusy(true); setError(''); setNotice('')
    try {
      const result = await invoke<{ added: number; skipped: number }>('gmail_sync')
      if (await refresh()) {
        setNotice(`Checked Gmail: ${result.added} new email${result.added === 1 ? '' : 's'} added for review; ${result.skipped} already processed or unavailable.`)
      }
    } catch (e) { setError(message(e)) } finally { setBusy(false) }
  }

  function update(candidateId: number, patch: Partial<Draft>) {
    setDrafts(current => {
      const currentDraft = current[candidateId]
      const next = { ...currentDraft, ...patch }
      if (patch.kind && patch.kind !== currentDraft.kind) {
        next.category_id = String(categories.find(category => category.kind === patch.kind)?.id ?? '')
      }
      return { ...current, [candidateId]: next }
    })
  }

  async function approve(candidateId: number) {
    if (busy) return
    const draft = drafts[candidateId]
    if (!draft?.kind || !draft.category_id || !draft.account_id) {
      setError('Choose an income/expense type, category, and account before adding this transaction.')
      return
    }
    setBusy(true); setError(''); setNotice('')
    try {
      const imported = await invoke<boolean>('approve_gmail_candidate', {
        input: {
          candidate_id: candidateId, date: draft.date, amount: draft.amount, kind: draft.kind,
          description: draft.description, category_id: Number(draft.category_id), account_id: Number(draft.account_id),
        },
      })
      if (await refresh()) {
        setNotice(imported ? 'Transaction approved and added to your ledger.' : 'Possible duplicate detected. No ledger entry was created.')
      }
    } catch (e) { setError(message(e)) } finally { setBusy(false) }
  }

  async function ignore(candidateId: number) {
    if (busy) return
    setBusy(true); setError(''); setNotice('')
    try {
      await invoke('ignore_gmail_candidate', { candidateId })
      if (await refresh()) setNotice('Email marked as handled; no ledger entry was created.')
    } catch (e) { setError(message(e)) } finally { setBusy(false) }
  }

  return (
    <section className="panel gmail-review">
      <div className="panel-heading">
        <div><h2>Transaction email review</h2><p>Manually sync up to 12 months of Gmail, then verify every candidate before import.</p></div>
        <Button disabled={!isTauri() || busy} onClick={() => void sync()}><RefreshCw /> {busy ? 'Working...' : 'Sync Gmail'}</Button>
      </div>
      {error && <div role="alert" className="message error">{error}</div>}
      {notice && <div role="status" className="message success">{notice}</div>}
      {!isTauri() ? <div className="empty-state"><Mail size={30} /><h3>Desktop connection required</h3><p>Open PaisaPilot Desktop to connect Gmail and review messages.</p></div> :
        candidates.length === 0 ? <div className="empty-state"><Mail size={30} /><h3>No emails waiting for review</h3><p>Connect Gmail in Settings, then sync. Matching emails are stored locally and will not enter the ledger until you approve them.</p><Button variant="outline" onClick={() => void refresh()}>Refresh candidates</Button></div> :
          <div className="gmail-candidate-list">{candidates.map(candidate => {
            const draft = drafts[candidate.id] ?? asDraft(candidate, accounts, categories)
            const blocked = candidate.event_type === 'transfer' || candidate.event_type === 'card_payment'
            return <article className="gmail-candidate" key={candidate.id}>
              <div className="gmail-candidate-heading">
                <div><span className="gmail-type">{candidate.event_type.replace('_', ' ')}</span><h3>{candidate.subject || candidate.merchant || 'Transaction email'}</h3><p>{candidate.sender} · {new Date(candidate.received_at_ms).toLocaleDateString('en-IN')}</p></div>
                <span className={`gmail-status ${candidate.status}`}>{candidate.status === 'duplicate' ? 'Possible duplicate' : 'Needs review'}</span>
              </div>
              <div className="gmail-evidence">
                {candidate.account_hint && <span>Account/card: {candidate.account_hint}</span>}
                {candidate.reference && <span>Reference: {candidate.reference}</span>}
                <span>Detected currency: {candidate.currency}</span>
              </div>
              {candidate.status === 'duplicate' ? <p className="gmail-warning">A matching ledger transaction or reference was found. This email cannot be imported again.</p> :
                <div className="gmail-edit-grid">
                  <label>Date<input type="date" value={draft.date} onChange={event => update(candidate.id, { date: event.target.value })} /></label>
                  <label>Amount (INR)<input inputMode="decimal" value={draft.amount} onChange={event => update(candidate.id, { amount: event.target.value })} /></label>
                  <label>Type<select value={draft.kind} onChange={event => update(candidate.id, { kind: event.target.value as Draft['kind'] })}><option value="">Choose type</option><option value="expense">Expense</option><option value="income">Income / refund</option></select></label>
                  <label>Category<select value={draft.category_id} onChange={event => update(candidate.id, { category_id: event.target.value })}><option value="">Choose category</option>{categories.filter(category => category.kind === draft.kind).map(category => <option key={category.id} value={category.id}>{category.name}</option>)}</select></label>
                  <label>Account<select value={draft.account_id} onChange={event => update(candidate.id, { account_id: event.target.value })}><option value="">Choose account</option>{accounts.map(account => <option key={account.id} value={account.id}>{account.name}</option>)}</select></label>
                  <label className="gmail-description">Description<input maxLength={240} value={draft.description} onChange={event => update(candidate.id, { description: event.target.value })} /></label>
                </div>}
              {blocked && candidate.status !== 'duplicate' && <p className="gmail-warning">Transfers and credit-card payments are identified but cannot be added to the current income/expense ledger, to avoid double-counting.</p>}
              <div className="gmail-actions">
                {candidate.status !== 'duplicate' && <Button disabled={busy || blocked || candidate.currency !== 'INR'} onClick={() => void approve(candidate.id)}>Approve and add</Button>}
                <Button disabled={busy} variant="outline" onClick={() => void ignore(candidate.id)}>{candidate.status === 'duplicate' ? 'Dismiss' : 'Skip'}</Button>
              </div>
            </article>
          })}</div>}
      <div className="panel-footer"><ShieldCheck size={14} /><span>Read-only Gmail access · Local parsing · Approval required</span></div>
    </section>
  )
}
