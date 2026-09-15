import { useState, type FormEvent } from 'react'
import { useAuth } from '../auth'
import Button from '../components/Button'
import Input from '../components/Input'
import { toErrorMessage } from '../utils/error'

export default function LoginPage() {
  const { login, signup } = useAuth()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const [mode, setMode] = useState<'login' | 'signup'>('login')

  async function handleSubmit(e: FormEvent, submitMode: 'login' | 'signup') {
    e.preventDefault()
    if (!username.trim() || !password.trim()) return
    setBusy(true)
    setError('')
    try {
      if (submitMode === 'login') {
        await login(username.trim(), password)
      } else {
        await signup(username.trim(), password)
      }
    } catch (err: unknown) {
      setError(toErrorMessage(err, 'Request failed'))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="min-h-screen bg-surface font-prose flex flex-col justify-between p-4 sm:p-8">
      <header className="w-full max-w-sm mx-auto flex items-center justify-between py-4">
        <div className="flex items-center gap-2.5">
          <span className="w-2 h-2 rounded-full bg-rose shrink-0" aria-hidden="true" />
          <span className="font-mono text-xs font-semibold tracking-widest uppercase text-rose">
            Cinder
          </span>
        </div>
        <span className="font-mono text-[10px] uppercase tracking-widest text-muted">
          Archive Access
        </span>
      </header>

      <main className="w-full max-w-sm mx-auto my-auto py-8">
        <div className="mb-6 text-center sm:text-left">
          <span className="text-[10px] font-mono uppercase tracking-widest text-muted block mb-1.5">
            Frontispiece &bull; Access
          </span>
          <h1 className="text-3xl font-bold font-prose text-text tracking-tight">
            {mode === 'login' ? 'Open the Library' : 'New Reader'}
          </h1>
          <p className="text-sm text-muted font-prose mt-1.5 leading-relaxed">
            {mode === 'login'
              ? 'Enter your credentials to resume reading your chronicles.'
              : 'Create your reader identity to begin exploring new worlds.'}
          </p>
        </div>

        <div className="flex border-b border-subtle/70 mb-6">
          <button
            type="button"
            onClick={() => { setMode('login'); setError('') }}
            className={`flex-1 pb-2.5 pt-1 text-center text-xs font-mono uppercase tracking-widest transition-colors cursor-pointer border-b-2 ${
              mode === 'login'
                ? 'border-foam text-foam font-semibold'
                : 'border-transparent text-muted hover:text-text'
            }`}
          >
            Sign In
          </button>
          <button
            type="button"
            onClick={() => { setMode('signup'); setError('') }}
            className={`flex-1 pb-2.5 pt-1 text-center text-xs font-mono uppercase tracking-widest transition-colors cursor-pointer border-b-2 ${
              mode === 'signup'
                ? 'border-foam text-foam font-semibold'
                : 'border-transparent text-muted hover:text-text'
            }`}
          >
            Register
          </button>
        </div>

        <form onSubmit={e => handleSubmit(e, mode)} className="space-y-4">
          {error && (
            <div className="p-3 rounded border border-love/40 bg-love/10 text-love text-xs font-mono leading-relaxed">
              {error}
            </div>
          )}

          <div>
            <label className="block text-xs font-mono uppercase tracking-wider text-muted mb-1.5">
              Username
            </label>
            <Input
              type="text"
              placeholder="Reader name"
              value={username}
              onChange={e => setUsername(e.target.value)}
              disabled={busy}
              autoFocus
              className="w-full font-mono text-sm py-2 px-3 bg-overlay/50 border-subtle focus:border-pine focus:bg-overlay"
            />
          </div>

          <div>
            <label className="block text-xs font-mono uppercase tracking-wider text-muted mb-1.5">
              Password
            </label>
            <Input
              type="password"
              placeholder="Passphrase"
              value={password}
              onChange={e => setPassword(e.target.value)}
              disabled={busy}
              className="w-full font-mono text-sm py-2 px-3 bg-overlay/50 border-subtle focus:border-pine focus:bg-overlay"
            />
          </div>

          <div className="pt-2">
            <Button
              variant="primary"
              size="md"
              className="w-full py-2.5 font-mono text-xs uppercase tracking-wider"
              disabled={busy || !username.trim() || !password.trim()}
              type="submit"
            >
              {busy
                ? 'Consulting Library...'
                : mode === 'login'
                ? 'Enter Library ›'
                : 'Create Account ›'}
            </Button>
          </div>

          <div className="pt-2 text-center">
            {mode === 'login' ? (
              <p className="text-xs text-muted">
                New reader?{' '}
                <button
                  type="button"
                  onClick={() => { setMode('signup'); setError('') }}
                  className="text-foam hover:underline font-medium cursor-pointer"
                >
                  Register an account
                </button>
              </p>
            ) : (
              <p className="text-xs text-muted">
                Already have a library account?{' '}
                <button
                  type="button"
                  onClick={() => { setMode('login'); setError('') }}
                  className="text-foam hover:underline font-medium cursor-pointer"
                >
                  Sign in
                </button>
              </p>
            )}
          </div>
        </form>
      </main>

      <footer className="w-full max-w-sm mx-auto py-4 text-center">
        <p className="text-[10px] font-mono text-muted/60 tracking-widest uppercase">
          Cinder &bull; Interactive Fiction System
        </p>
      </footer>
    </div>
  )
}
