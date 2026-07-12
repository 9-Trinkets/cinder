import { useEffect, useState } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import Button from '../components/Button'
import Card from '../components/Card'
import ConfirmDialog from '../components/ConfirmDialog'
import Skeleton from '../components/Skeleton'
import { useToast } from '../components/Toast'

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
  const { showToast } = useToast()
  const [sessions, setSessions] = useState<api.SessionInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [creating, setCreating] = useState(false)
  const [deleting, setDeleting] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null)

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

  async function doDelete(sessionId: string) {
    if (!token || deleting) return
    setConfirmDelete(null)
    setDeleting(sessionId)
    try {
      await api.deleteSession(token, sessionId)
      setSessions(prev => prev.filter(s => s.session_id !== sessionId))
    } catch (err: unknown) {
      showToast(err instanceof Error ? err.message : 'failed to delete', 'error')
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
      showToast(err instanceof Error ? err.message : 'failed to create session', 'error')
    } finally {
      setCreating(false)
    }
  }

  return (
    <div className="min-h-screen bg-surface">
      <header className="flex items-center justify-between px-6 py-4 border-b border-subtle">
        <h1 className="text-xl font-bold text-rose">Cinder</h1>
        <Button variant="ghost" onClick={logout}>Log out</Button>
      </header>

      <main className="max-w-2xl mx-auto px-4 py-8 space-y-8">
        <section>
          <h2 className="text-lg font-semibold text-text mb-4">New Game</h2>
          <div className="flex gap-3">
            {PACKS.map(p => (
              <Button
                key={p}
                onClick={() => create(p)}
                disabled={creating}
                className="px-5 py-3 capitalize"
              >
                {p}
              </Button>
            ))}
          </div>
        </section>

        <section>
          <h2 className="text-lg font-semibold text-text mb-4">Sessions</h2>
          {loading ? (
            <Skeleton lines={3} />
          ) : error ? (
            <p className="text-love text-sm">{error}</p>
          ) : sessions.length === 0 ? (
            <p className="text-muted">No sessions yet.</p>
          ) : (
            <div className="space-y-2">
              {sessions.map(s => (
                <Card key={s.session_id} className="flex items-center px-4 py-3 group">
                  <div
                    onClick={() => navigate(`/games/${s.session_id}`)}
                    className="flex-1 flex items-center justify-between cursor-pointer"
                  >
                    <span className="text-text capitalize">{s.pack_id}</span>
                    <span className="text-faint text-xs">{fmtTime(s.updated_at)}</span>
                  </div>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setConfirmDelete(s.session_id)}
                    disabled={deleting === s.session_id}
                    className="ml-3 opacity-0 group-hover:opacity-100"
                  >
                    {deleting === s.session_id ? '...' : '✕'}
                  </Button>
                </Card>
              ))}
            </div>
          )}
        </section>
      </main>

      {confirmDelete && (
        <ConfirmDialog
          title="Delete session"
          message="Delete this session? This cannot be undone."
          confirmLabel="Delete"
          onConfirm={() => doDelete(confirmDelete)}
          onCancel={() => setConfirmDelete(null)}
        />
      )}
    </div>
  )
}
