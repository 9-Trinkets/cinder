import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'

const PACKS = ['ella', 'isla', 'aera']

export default function GamesPage() {
  const { token, logout } = useAuth()
  const navigate = useNavigate()
  const [sessions, setSessions] = useState<api.SessionInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [creating, setCreating] = useState(false)

  async function load() {
    if (!token) return
    try {
      const list = await api.listSessions(token)
      setSessions(list)
    } catch {
      /* ignore */
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [token])

  async function create(packId: string) {
    if (!token) return
    setCreating(true)
    try {
      const session = await api.createSession(token, packId)
      navigate(`/games/${session.session_id}`, { state: { title: session.title, intro_text: session.intro_text } })
    } catch (err: unknown) {
      alert(err instanceof Error ? err.message : 'failed to create session')
    } finally {
      setCreating(false)
    }
  }

  return (
    <div className="min-h-screen bg-surface">
      <header className="flex items-center justify-between px-6 py-4 border-b border-subtle">
        <h1 className="text-xl font-bold text-rose">Cinder</h1>
        <button onClick={logout} className="text-sm text-muted hover:text-love cursor-pointer">Log out</button>
      </header>

      <main className="max-w-2xl mx-auto px-4 py-8 space-y-8">
        <section>
          <h2 className="text-lg font-semibold text-text mb-4">New Game</h2>
          <div className="flex gap-3">
            {PACKS.map(p => (
              <button
                key={p}
                onClick={() => create(p)}
                disabled={creating}
                className="px-5 py-3 rounded bg-subtle text-text border border-subtle hover:border-pine capitalize disabled:opacity-50 cursor-pointer"
              >
                {p}
              </button>
            ))}
          </div>
        </section>

        <section>
          <h2 className="text-lg font-semibold text-text mb-4">Sessions</h2>
          {loading ? (
            <p className="text-muted">Loading...</p>
          ) : sessions.length === 0 ? (
            <p className="text-muted">No sessions yet.</p>
          ) : (
            <div className="space-y-2">
              {sessions.map(s => (
                <div
                  key={s.session_id}
                  onClick={() => navigate(`/games/${s.session_id}`)}
                  className="flex items-center justify-between px-4 py-3 rounded bg-overlay hover:bg-subtle cursor-pointer"
                >
                  <span className="text-text capitalize">{s.pack_id}</span>
                  <span className="text-faint text-xs">{s.updated_at}</span>
                </div>
              ))}
            </div>
          )}
        </section>
      </main>
    </div>
  )
}
