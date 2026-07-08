import { useEffect, useState } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'

function fmtTime(s: string): string {
  const n = Number(s)
  if (!isNaN(n) && n > 1e8) {
    return new Date(n * 1000).toLocaleString()
  }
  const d = new Date(s)
  if (!isNaN(d.getTime())) return d.toLocaleString()
  return s
}

const PACKS = ['ella', 'isla', 'aera']

export default function GamesPage() {
  const { token, logout } = useAuth()
  const navigate = useNavigate()
  const location = useLocation()
  const [sessions, setSessions] = useState<api.SessionInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [creating, setCreating] = useState(false)
  const [deleting, setDeleting] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  async function load() {
    if (!token) return
    setError(null)
    try {
      const list = await api.listSessions(token)
      setSessions(list)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'failed to load')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [token, location.key])

  async function doDelete(sessionId: string, e: React.MouseEvent) {
    e.stopPropagation()
    if (!token || deleting) return
    if (!confirm('Delete this session?')) return
    setDeleting(sessionId)
    try {
      await api.deleteSession(token, sessionId)
      setSessions(prev => prev.filter(s => s.session_id !== sessionId))
    } catch (err: unknown) {
      alert(err instanceof Error ? err.message : 'failed to delete')
    } finally {
      setDeleting(null)
    }
  }

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
          ) : error ? (
            <p className="text-love text-sm">{error}</p>
          ) : sessions.length === 0 ? (
            <p className="text-muted">No sessions yet.</p>
          ) : (
            <div className="space-y-2">
              {sessions.map(s => (
                <div key={s.session_id} className="flex items-center px-4 py-3 rounded bg-overlay hover:bg-subtle group">
                  <div
                    onClick={() => navigate(`/games/${s.session_id}`)}
                    className="flex-1 flex items-center justify-between cursor-pointer"
                  >
                    <span className="text-text capitalize">{s.pack_id}</span>
                    <span className="text-faint text-xs">{fmtTime(s.updated_at)}</span>
                  </div>
                  <button
                    onClick={e => doDelete(s.session_id, e)}
                    disabled={deleting === s.session_id}
                    className="ml-3 text-xs text-muted hover:text-love opacity-0 group-hover:opacity-100 disabled:opacity-50 cursor-pointer shrink-0"
                  >
                    {deleting === s.session_id ? '...' : '✕'}
                  </button>
                </div>
              ))}
            </div>
          )}
        </section>
      </main>
    </div>
  )
}
