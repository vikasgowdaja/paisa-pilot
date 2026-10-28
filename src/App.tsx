import { useCallback, useEffect, useRef, useState } from 'react'
import type { FormEvent } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { ArrowDownLeft, ArrowUpRight, ArrowLeftRight, ChevronRight, Download, FolderLock, LayoutDashboard, Leaf, Menu, Pencil, Plus, Search, Settings, ShieldCheck, Tags, Trash2, Upload, Wallet, X } from 'lucide-react'
import { Button } from './components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from './components/ui/dialog'
import './App.css'

type Page = 'Dashboard' | 'Transactions' | 'Accounts' | 'Categories' | 'Settings'
type Kind = 'income' | 'expense'
interface Transaction {
  id: number; date: string; amount_display: string; amount_input: string; kind: Kind
  description: string; category_id: number; account_id: number
}
interface Account { id: number; name: string; balance: string }
interface Category { id: number; name: string; kind: Kind }
interface Snapshot {
  transactions: Transaction[]; accounts: Account[]; categories: Category[]
  income: string; expenses: string; balance: string; total_balance: string; database_path: string
}
interface Draft {
  id: number | null; date: string; amount: string; kind: Kind
  description: string; category_id: number; account_id: number
}
const pages = [
  { name: 'Dashboard', icon: LayoutDashboard },
  { name: 'Transactions', icon: ArrowLeftRight },
  { name: 'Accounts', icon: Wallet },
  { name: 'Categories', icon: Tags },
  { name: 'Settings', icon: Settings },
] as const
function today() {
  const now = new Date()
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
}
function errorText(error: unknown) { return error instanceof Error ? error.message : String(error) }

function App() {
  const [page, setPage] = useState<Page>('Dashboard')
  const [collapsed, setCollapsed] = useState(false)
  const [month, setMonth] = useState(today().slice(0, 7))
  const [data, setData] = useState<Snapshot | null>(null)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [query, setQuery] = useState('')
  const [kind, setKind] = useState('')
  const [account, setAccount] = useState('')
  const [category, setCategory] = useState('')
  const [draft, setDraft] = useState<Draft | null>(null)
  const [formError, setFormError] = useState('')
  const [deleting, setDeleting] = useState<Transaction | null>(null)
  const [accountDialog, setAccountDialog] = useState(false)
  const [accountName, setAccountName] = useState('')
  const importRef = useRef<HTMLInputElement>(null)
  const request = useRef(0)

  const refresh = useCallback(async () => {
    const current = ++request.current
    try {
      const result = await (isTauri()
        ? invoke<Snapshot>('snapshot', { month })
        : Promise.reject(new Error('Open the PaisaPilot desktop app to access your database. Browser preview is not connected to Rust.')))
      if (current === request.current) { setData(result); setError('') }
    } catch (e) {
      if (current === request.current) { setData(null); setError(errorText(e)) }
    } finally { if (current === request.current) setLoading(false) }
  }, [month])

  // Refresh only updates state after the asynchronous desktop command settles.
  // eslint-disable-next-line react-hooks/set-state-in-effect
  useEffect(() => { void refresh() }, [refresh])

  function startTransaction(transaction?: Transaction) {
    if (!data) return
    setFormError('')
    setDraft(transaction ? { ...transaction, amount: transaction.amount_input } : {
      id: null, date: today(), amount: '', kind: 'expense', description: '',
      category_id: data.categories.find(c => c.kind === 'expense')!.id,
      account_id: data.accounts[0].id,
    })
  }

  async function saveTransaction(event: FormEvent) {
    event.preventDefault()
    if (!draft || busy) return
    setBusy(true); setFormError('')
    try {
      await invoke('save_transaction', { input: draft })
      setDraft(null)
      setNotice(`Transaction ${draft.id ? 'updated' : 'added'}. ${draft.date.slice(0, 7) !== month ? 'Switch to its month to view it.' : ''}`)
      await refresh()
    } catch (e) { setFormError(errorText(e)) } finally { setBusy(false) }
  }

  async function removeTransaction() {
    if (!deleting || busy) return
    setBusy(true); setFormError('')
    try {
      await invoke('delete_transaction', { id: deleting.id })
      setDeleting(null); setNotice('Transaction deleted.'); await refresh()
    } catch (e) { setFormError(errorText(e)) } finally { setBusy(false) }
  }

  async function saveAccount(event: FormEvent) {
    event.preventDefault()
    if (busy) return
    setBusy(true); setFormError('')
    try {
      await invoke('add_account', { name: accountName })
      setAccountDialog(false); setAccountName(''); setNotice('Account created.'); await refresh()
    } catch (e) { setFormError(errorText(e)) } finally { setBusy(false) }
  }

  async function exportCsv() {
    if (busy) return
    setBusy(true); setError('')
    try {
      const content = await invoke<string>('export_csv')
      const url = URL.createObjectURL(new Blob([content], { type: 'text/csv;charset=utf-8' }))
      const link = document.createElement('a')
      link.href = url; link.download = `paisapilot-${today()}.csv`; link.click()
      window.setTimeout(() => URL.revokeObjectURL(url), 10000)
      setNotice('CSV export requested. All dates and accounts are included.')
    } catch (e) { setError(errorText(e)) } finally { setBusy(false) }
  }

  async function importCsv(file: File) {
    setBusy(true); setError('')
    try {
      if (file.size > 5_000_000) throw new Error('CSV must be smaller than 5 MB.')
      const result = await invoke<{ imported: number; skipped: number }>('import_csv', { content: await file.text() })
      setNotice(`Imported ${result.imported} transactions; skipped ${result.skipped} duplicate source IDs.`)
      await refresh()
    } catch (e) { setError(errorText(e)) } finally { setBusy(false) }
  }

  const transactions = data?.transactions.filter(t =>
    (!kind || t.kind === kind) && (!account || String(t.account_id) === account) &&
    (!category || String(t.category_id) === category) &&
    `${t.description} ${data.categories.find(c => c.id === t.category_id)?.name ?? ''} ${t.date}`.toLowerCase().includes(query.toLowerCase()),
  ) ?? []
  const hasFilters = Boolean(query || kind || account || category)
  const visible = page === 'Dashboard' ? (data?.transactions.slice(0, 6) ?? []) : transactions
  const monthLabel = new Date(`${month}-01T12:00:00`).toLocaleDateString('en-IN', { month: 'long', year: 'numeric' })

  return (
    <div className={`app-shell ${collapsed ? 'collapsed' : ''}`}>
      <aside className="sidebar">
        <div className="brand"><div className="brand-icon"><Leaf size={22} /></div><span>Paisa<span className="font-normal">Pilot</span><small>YOUR MONEY, IN FOCUS</small></span></div>
        <div className="nav-caption">WORKSPACE</div>
        <nav aria-label="Main navigation">{pages.map(({ name, icon: Icon }) =>
          <button key={name} title={name} className={`nav-item ${page === name ? 'active' : ''}`} onClick={() => setPage(name)}><Icon size={19} /><span>{name}</span>{page === name && <span className="nav-dot" />}</button>,
        )}</nav>
        <div className="sidebar-bottom"><div className="privacy-card"><ShieldCheck size={23} /><strong>Private by design</strong><p>Your finances stay on this device. No cloud. No noise.</p></div><div className="local-status"><span /> Local workspace <FolderLock size={15} /></div></div>
      </aside>
      <div className="workspace">
        <header className="topbar"><div className="flex items-center gap-3"><Button variant="ghost" size="icon" aria-label="Toggle sidebar" onClick={() => setCollapsed(!collapsed)}><Menu /></Button><span className="breadcrumb">Workspace <ChevronRight size={13} /> <strong>{page}</strong></span></div><span className="local-badge"><span /> Stored on your device</span></header>
        <main>
          <div className="page-heading"><div><div className="eyebrow">A LITTLE CLARITY. EVERY DAY.</div><h1>{page === 'Dashboard' ? 'Your money, at a glance.' : page}</h1><p>{page === 'Dashboard' ? 'Make room for what matters. Keep track of the rest.' : page === 'Transactions' ? 'Every rupee, with a place and a purpose.' : page === 'Accounts' ? 'A clear view of where your money lives.' : page === 'Categories' ? 'Simple categories for everyday life.' : 'Your workspace. Your data. Your control.'}</p></div>{(page === 'Dashboard' || page === 'Transactions') && <Button disabled={!data || busy} onClick={() => startTransaction()}><Plus /> Add transaction</Button>}{page === 'Accounts' && <Button disabled={!data || busy} onClick={() => { setFormError(''); setAccountDialog(true) }}><Plus /> Add account</Button>}</div>
          {error && <div role="alert" className="message error">{error}<Button variant="ghost" size="sm" onClick={() => { setLoading(true); void refresh() }}>Retry</Button></div>}
          {notice && <div role="status" className="message success">{notice}<button aria-label="Dismiss notification" onClick={() => setNotice('')}><X size={16} /></button></div>}
          {(page === 'Dashboard' || page === 'Transactions') && <>
            <div className="period-row"><div><span className="section-label">MONTHLY OVERVIEW</span><span className="period-note"> A fresh perspective on your finances</span></div><label className="month-picker"><span className="sr-only">Select month</span><input aria-label="Select month" type="month" value={month} min="0001-01" max="9999-12" onChange={e => { if (e.target.value && e.target.value !== month) { setLoading(true); setData(null); setMonth(e.target.value) } }} /></label></div>
            <div className="summary-grid" aria-busy={loading}>
              <Summary title="Monthly income" value={data?.income} detail="Money coming in" type="income" />
              <Summary title="Monthly expenses" value={data?.expenses} detail="Money going out" type="expense" />
              <Summary title="Monthly balance" value={data?.balance} detail="Income minus expenses" type="balance" />
            </div>
            <section className="panel transaction-panel">
              <div className="panel-heading"><div><h2>{page === 'Dashboard' ? 'Recent transactions' : 'All transactions'}</h2><p>{monthLabel}{data && ` · ${data.transactions.length} transactions`}</p></div>{page === 'Dashboard' ? <Button variant="ghost" size="sm" onClick={() => setPage('Transactions')}>View all <ChevronRight /></Button> : <Button variant="outline" size="sm" disabled={!data || busy} onClick={() => void exportCsv()}><Download /> Export all</Button>}</div>
              {page === 'Transactions' && <div className="filters"><label className="search-field"><Search size={16} /><input placeholder="Search descriptions, dates, categories..." aria-label="Search transactions" value={query} onChange={e => setQuery(e.target.value)} /></label><select aria-label="Transaction type filter" value={kind} onChange={e => setKind(e.target.value)}><option value="">All types</option><option value="income">Income</option><option value="expense">Expenses</option></select><select aria-label="Account filter" value={account} onChange={e => setAccount(e.target.value)}><option value="">All accounts</option>{data?.accounts.map(a => <option key={a.id} value={a.id}>{a.name}</option>)}</select><select aria-label="Category filter" value={category} onChange={e => setCategory(e.target.value)}><option value="">All categories</option>{data?.categories.map(c => <option key={c.id} value={c.id}>{c.name}</option>)}</select>{hasFilters && <Button size="sm" variant="ghost" onClick={() => { setQuery(''); setKind(''); setAccount(''); setCategory('') }}>Clear</Button>}</div>}
              {loading ? <div className="empty-state"><p>Loading your local workspace...</p></div> : !data ? <div className="empty-state"><FolderLock size={32} /><h3>Desktop connection required</h3><p>Your financial data is only available through the Rust desktop app.</p></div> : visible.length === 0 ? <div className="empty-state"><div className="empty-icon"><ArrowLeftRight size={27} /></div><h3>{hasFilters && page === 'Transactions' ? 'No matching transactions' : 'A fresh start for your finances'}</h3><p>{hasFilters && page === 'Transactions' ? 'Try another search or clear the filters.' : `No transactions for ${monthLabel}. Add your first income or expense to get started.`}</p><Button variant="outline" onClick={() => startTransaction()}><Plus /> Add your first transaction</Button></div> :
                <div className="table-scroll"><table><thead><tr><th>TRANSACTION</th><th>CATEGORY</th><th>ACCOUNT</th><th>DATE</th><th className="text-right">AMOUNT</th><th><span className="sr-only">Actions</span></th></tr></thead><tbody>{visible.map(t => <tr key={t.id}><td><div className="transaction-title"><span className={`transaction-icon ${t.kind}`}>{t.kind === 'income' ? <ArrowDownLeft size={18} /> : <ArrowUpRight size={18} />}</span><div><strong>{t.description}</strong><small>{t.kind === 'income' ? 'Income' : 'Expense'}</small></div></div></td><td><span className="category-badge">{data.categories.find(c => c.id === t.category_id)?.name}</span></td><td>{data.accounts.find(a => a.id === t.account_id)?.name}</td><td className="date-cell">{new Date(`${t.date}T12:00:00`).toLocaleDateString('en-IN', { day: '2-digit', month: 'short' })}</td><td className={`amount ${t.kind}`}>{t.kind === 'expense' ? '-' : '+'}{t.amount_display}</td><td><div className="row-actions"><Button variant="ghost" size="icon" aria-label={`Edit ${t.description}`} onClick={() => startTransaction(t)}><Pencil /></Button><Button variant="ghost" size="icon" aria-label={`Delete ${t.description}`} onClick={() => { setFormError(''); setDeleting(t) }}><Trash2 /></Button></div></td></tr>)}</tbody></table></div>}
              <div className="panel-footer"><ShieldCheck size={14} /><span>Real transactions. Saved locally. Always yours.</span>{page === 'Transactions' && data && <span className="ml-auto">{transactions.length} matching</span>}</div>
            </section>
          </>}
          {page === 'Dashboard' && <div className="bottom-grid"><section className="panel accounts-overview"><div className="panel-heading"><div><h2>Your accounts</h2><p>All-time recorded balances</p></div><Button variant="ghost" size="sm" onClick={() => setPage('Accounts')}>Manage <ChevronRight /></Button></div>{data ? data.accounts.map(a => <div className="account-row" key={a.id}><span className="account-icon"><Wallet size={19} /></span><strong>{a.name}</strong><span className="ml-auto font-semibold tabular-nums">{a.balance}</span></div>) : <p className="p-6 text-muted-foreground">Connect the desktop app to view accounts.</p>}</section><section className="quiet-card"><div className="quiet-illustration"><Leaf size={36} /></div><span className="eyebrow">SMALL HABITS, LASTING CHANGE</span><h2>A little tracking.<br />A lot more peace of mind.</h2><p>Your financial story starts with one transaction. Take it one day at a time.</p><span className="quiet-footer"><ShieldCheck size={15} /> No sign-up. No cloud. Just you.</span></section></div>}
          {page === 'Accounts' && <><div className="total-strip"><Wallet size={24} /><div><p>Total across all accounts</p><h2>{data?.total_balance ?? '—'}</h2></div><span>All-time recorded transactions · INR</span></div><div className="account-grid">{data?.accounts.map(a => <section className="panel account-tile" key={a.id}><div className="account-icon"><Wallet size={23} /></div><h2>{a.name}</h2><p>Recorded balance</p><strong>{a.balance}</strong></section>)}</div><p className="help-note">Accounts start at zero. Add an income transaction to record an opening balance. Transfers are not automated.</p></>}
          {page === 'Categories' && <div className="category-grid">{(['expense', 'income'] as const).map(type => <section className="panel" key={type}><div className="panel-heading"><div><h2>{type === 'expense' ? 'Expense categories' : 'Income categories'}</h2><p>Built-in categories to keep things simple</p></div><Tags size={20} /></div><div className="category-list">{data?.categories.filter(c => c.kind === type).map(c => <div key={c.id}><span className={`transaction-icon ${type}`}><Tags size={17} /></span><strong>{c.name}</strong></div>)}</div></section>)}</div>}
          {page === 'Settings' && <div className="settings-stack"><section className="panel"><div className="panel-heading"><div><h2>Data & privacy</h2><p>No account or internet connection needed for daily use.</p></div><ShieldCheck size={23} /></div><div className="settings-content"><h3>Local SQLite database</h3><code>{data?.database_path ?? 'Open in the desktop app to locate the database.'}</code><p>Back up this database file with PaisaPilot fully closed. It contains your financial records and is not encrypted. Protect it using your Windows account and device encryption.</p></div></section><section className="panel"><div className="panel-heading"><div><h2>Import & export</h2><p>Move your records without giving up control.</p></div><ArrowLeftRight size={21} /></div><div className="settings-content"><div className="flex flex-wrap gap-3"><Button disabled={!data || busy} variant="outline" onClick={() => void exportCsv()}><Download /> Export all transactions</Button><Button disabled={!data || busy} variant="outline" onClick={() => importRef.current?.click()}><Upload /> Import CSV</Button></div><p>CSV headers (in order): <code>source_id,date,amount,kind,description,category,account</code></p><p>Amounts are in rupees (e.g. 125.50); dates use YYYY-MM-DD. Use income or expense for kind. Account and category names must already exist. Every row requires a unique, stable source ID. Reimporting an ID skips it, even if its fields changed. Invalid rows cancel the whole import. Maximum 5 MB.</p><p>Exports include all months and accounts. Treat exported files as private. They are data files, not spreadsheets; do not enable formulas from untrusted imports.</p></div></section><div className="version-line"><Leaf size={17} /> PaisaPilot 1.0.0 <span>Built with Rust, Tauri & SQLite</span></div></div>}
          <input ref={importRef} className="hidden" type="file" accept=".csv,text/csv" aria-label="Import CSV file" onChange={e => { const file = e.target.files?.[0]; if (file) void importCsv(file); e.target.value = '' }} />
          <footer className="workspace-footer"><span>PaisaPilot <span className="footer-dot">·</span> A calmer relationship with money.</span><span>INR <span className="footer-dot">·</span> Local-first</span></footer>
        </main>
      </div>
      <Dialog open={draft !== null} onOpenChange={open => { if (!open && !busy) setDraft(null) }}>
        <DialogContent><DialogTitle className="text-xl font-semibold">{draft?.id ? 'Edit transaction' : 'Add transaction'}</DialogTitle><DialogDescription className="mt-1 text-sm text-muted-foreground">A small step toward a clearer picture.</DialogDescription>{draft && <form onSubmit={e => void saveTransaction(e)} className="transaction-form">
          <div className="type-toggle">{(['expense', 'income'] as const).map(type => <button key={type} type="button" className={draft.kind === type ? 'selected' : ''} onClick={() => setDraft({ ...draft, kind: type, category_id: data!.categories.find(c => c.kind === type)!.id })}>{type === 'expense' ? <ArrowUpRight size={16} /> : <ArrowDownLeft size={16} />}{type === 'expense' ? 'Expense' : 'Income'}</button>)}</div>
          <div className="form-grid"><label>Amount (INR)<input autoFocus required inputMode="decimal" placeholder="0.00" maxLength={14} value={draft.amount} onChange={e => setDraft({ ...draft, amount: e.target.value })} /></label><label>Date<input required type="date" min="0001-01-01" max="9999-12-31" value={draft.date} onChange={e => setDraft({ ...draft, date: e.target.value })} /></label></div>
          <label>Description<input required maxLength={240} placeholder="What was this for?" value={draft.description} onChange={e => setDraft({ ...draft, description: e.target.value })} /></label>
          <div className="form-grid"><label>Category<select value={draft.category_id} onChange={e => setDraft({ ...draft, category_id: Number(e.target.value) })}>{data?.categories.filter(c => c.kind === draft.kind).map(c => <option key={c.id} value={c.id}>{c.name}</option>)}</select></label><label>Account<select value={draft.account_id} onChange={e => setDraft({ ...draft, account_id: Number(e.target.value) })}>{data?.accounts.map(a => <option key={a.id} value={a.id}>{a.name}</option>)}</select></label></div>
          {formError && <p role="alert" className="form-error">{formError}</p>}
          <div className="dialog-actions"><Button type="button" variant="outline" disabled={busy} onClick={() => setDraft(null)}>Cancel</Button><Button type="submit" disabled={busy}>{busy ? 'Saving...' : 'Save transaction'}</Button></div>
        </form>}</DialogContent>
      </Dialog>
      <Dialog open={deleting !== null} onOpenChange={open => { if (!open && !busy) setDeleting(null) }}><DialogContent><DialogTitle className="text-xl font-semibold">Delete this transaction?</DialogTitle><DialogDescription className="mt-3 text-sm text-muted-foreground">Permanently remove &quot;{deleting?.description}&quot; ({deleting?.amount_display})? This cannot be undone.</DialogDescription>{formError && <p role="alert" className="form-error mt-4">{formError}</p>}<div className="dialog-actions mt-6"><Button disabled={busy} variant="outline" onClick={() => setDeleting(null)}>Keep transaction</Button><Button disabled={busy} variant="destructive" onClick={() => void removeTransaction()}>{busy ? 'Deleting...' : 'Delete transaction'}</Button></div></DialogContent></Dialog>
      <Dialog open={accountDialog} onOpenChange={open => { if (!busy) setAccountDialog(open) }}><DialogContent><DialogTitle className="text-xl font-semibold">Add an account</DialogTitle><DialogDescription className="mt-1 text-sm text-muted-foreground">A bank account, wallet, or cash in hand.</DialogDescription><form className="transaction-form" onSubmit={e => void saveAccount(e)}><label>Account name<input autoFocus required maxLength={60} placeholder="e.g. Savings account" value={accountName} onChange={e => setAccountName(e.target.value)} /></label>{formError && <p className="form-error" role="alert">{formError}</p>}<div className="dialog-actions"><Button type="button" variant="outline" disabled={busy} onClick={() => setAccountDialog(false)}>Cancel</Button><Button type="submit" disabled={busy}>{busy ? 'Saving...' : 'Create account'}</Button></div></form></DialogContent></Dialog>
    </div>
  )
}

function Summary({ title, value, detail, type }: { title: string; value?: string; detail: string; type: 'income' | 'expense' | 'balance' }) {
  return <section className={`summary-card ${type}`}><div className="summary-top"><span>{title}</span><span className="summary-icon">{type === 'income' ? <ArrowDownLeft size={20} /> : type === 'expense' ? <ArrowUpRight size={20} /> : <Wallet size={20} />}</span></div><strong>{value ?? '—'}</strong><p>{detail}<span>INR</span></p></section>
}

export default App
