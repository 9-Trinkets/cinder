import { useState, type FormEvent } from 'react'
import { useAuth } from '../auth'
import Button from '../components/Button'
import Input from '../components/Input'

export default function LoginPage() {
  const { login, signup } = useAuth()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)

  async function handleSubmit(e: FormEvent, mode: 'login' | 'signup') {
    e.preventDefault()
    if (!username || !password) return
    setBusy(true)
    setError('')
    try {
      if (mode === 'login') {
        await login(username, password)
      } else {
        await signup(username, password)
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Request failed')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-surface">
      <form className="flex flex-col gap-4 w-80 p-8 rounded-lg border border-subtle bg-overlay">
        <h1 className="text-xl font-bold text-iris text-center">Cinder</h1>
        <Input
          placeholder="Username"
          value={username}
          onChange={e => setUsername(e.target.value)}
          disabled={busy}
        />
        <Input
          type="password"
          placeholder="Password"
          value={password}
          onChange={e => setPassword(e.target.value)}
          disabled={busy}
        />
        {error && <p className="text-love text-sm">{error}</p>}
        <div className="flex gap-2">
          <Button
            variant="primary"
            size="md"
            className="flex-1"
            onClick={e => handleSubmit(e, 'login')}
            disabled={busy || !username || !password}
          >
            {busy ? '...' : 'Login'}
          </Button>
          <Button
            variant="secondary"
            size="md"
            className="flex-1"
            onClick={e => handleSubmit(e, 'signup')}
            disabled={busy || !username || !password}
          >
            {busy ? '...' : 'Sign up'}
          </Button>
        </div>
      </form>
    </div>
  )
}
