import { useCallback, useEffect, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { Mail, Unlink } from 'lucide-react'
import { Button } from './ui/button'

interface AuthStatus { connected: boolean; email: string }

function message(error: unknown) { return error instanceof Error ? error.message : String(error) }

export function GmailSettings() {
  const [status, setStatus] = useState<AuthStatus>({ connected: false, email: '' })
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [notice, setNotice] = useState('')

  const refresh = useCallback(async () => {
    try {
      const auth = await invoke<AuthStatus>('gmail_auth_status')
      setStatus(auth)
      setError('')
    } catch (e) { setError(message(e)) }
  }, [])

  useEffect(() => {
    // The state update follows an asynchronous desktop query.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    if (isTauri()) void refresh()
  }, [refresh])

  async function connect() {
    if (busy) return
    setBusy(true); setError(''); setNotice('')
    try {
      const email = await invoke<string>('gmail_connect')
      setStatus({ connected: true, email })
      setNotice(`Connected to ${email}. Gmail access is read-only.`)
    } catch (e) { setError(message(e)) } finally { setBusy(false) }
  }

  async function disconnect() {
    if (busy) return
    setBusy(true); setError(''); setNotice('')
    try {
      await invoke('gmail_disconnect')
      setStatus({ connected: false, email: '' })
      setNotice('Gmail access removed from this device.')
    } catch (e) { setError(message(e)) } finally { setBusy(false) }
  }

  return (
    <section className="panel">
      <div className="panel-heading"><div><h2>Gmail transaction import</h2><p>Sign in with Google to review transaction alerts locally.</p></div><Mail size={21} /></div>
      <div className="settings-content gmail-settings">
        <div className="flex flex-wrap gap-3">
          <Button disabled={!isTauri() || busy || status.connected} onClick={() => void connect()}>{busy ? 'Connecting...' : 'Sign in with Google'}</Button>
          {status.connected && <Button disabled={busy} variant="outline" onClick={() => void disconnect()}><Unlink /> Disconnect {status.email}</Button>}
        </div>
        <p>Requests Gmail read-only access only. Messages are parsed on this device; tokens are stored in Windows Credential Manager. PaisaPilot never asks for your Google password or OTP. Email transactions are not added to your ledger until you approve them.</p>
        {!isTauri() && <p role="status">Open PaisaPilot Desktop to sign in; browser preview cannot access the secure token store.</p>}
        {error && <p role="alert" className="form-error">{error}<button className="gmail-retry" onClick={() => void refresh()}>Retry</button></p>}
        {notice && <p role="status" className="gmail-notice">{notice}</p>}
      </div>
    </section>
  )
}
