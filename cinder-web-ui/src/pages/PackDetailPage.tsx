import { useEffect, useState } from 'react'
import { useLocation, useNavigate, useParams } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import Button from '../components/Button'
import Card from '../components/Card'
import ConfirmDialog from '../components/ConfirmDialog'
import Skeleton from '../components/Skeleton'
import { useToast } from '../components/Toast'
import { toErrorMessage } from '../utils/error'

function fmtTime(s: string): string {
  const n = Number(s)
  if (!isNaN(n) && n > 1e8) {
    return new Date(n * 1000).toLocaleString()
  }
  const d = new Date(s)
  if (!isNaN(d.getTime())) return d.toLocaleString()
  return s
}

export default function PackDetailPage() {
  const { packId } = useParams<{ packId: string }>()
  const { token, logout } = useAuth()
  const navigate = useNavigate()
  const location = useLocation()
  const { showToast } = useToast()
  const [pack, setPack] = useState<api.PackInfo | null>(null)
  const [plays, setPlays] = useState<api.PlayInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [creating, setCreating] = useState(false)
  const [deleting, setDeleting] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null)

  async function load() {
    if (!token || !packId) return
    setError(null)
    try {
      const [packs, allPlays] = await Promise.all([
        api.listPacks(token),
        api.listPlays(token),
      ])
      setPack(packs.find(p => p.id === packId) ?? null)
      setPlays(allPlays.filter(p => p.pack_id === packId))
    } catch (err) {
      setError(toErrorMessage(err, 'failed to load'))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [token, packId, location.key])

  async function doDelete(playId: string) {
    if (!token || deleting) return
    setConfirmDelete(null)
    setDeleting(playId)
    try {
      await api.deletePlay(token, playId)
      setPlays(prev => prev.filter(p => p.play_id !== playId))
    } catch (err: unknown) {
      showToast(toErrorMessage(err, 'failed to delete'), 'error')
    } finally {
      setDeleting(null)
    }
  }

  async function create() {
    if (!token || !packId) return
    setCreating(true)
    try {
      const play = await api.createPlay(token, packId)
      navigate(`/games/${play.play_id}`, { state: { title: play.title, intro_text: play.intro_text } })
    } catch (err: unknown) {
      showToast(toErrorMessage(err, 'failed to create play'), 'error')
    } finally {
      setCreating(false)
    }
  }

  return (
    <div className="min-h-screen bg-surface">
      <header className="flex items-center justify-between px-6 py-4 border-b border-subtle">
        <button onClick={() => navigate('/games')} className="text-sm text-muted hover:text-text cursor-pointer">&larr; Games</button>
        <Button variant="ghost" onClick={logout}>Log out</Button>
      </header>

      <main className="max-w-2xl mx-auto px-4 py-8 space-y-8">
        {loading ? (
          <Skeleton lines={3} />
        ) : error ? (
          <p className="text-love text-sm">{error}</p>
        ) : !pack ? (
          <p className="text-muted">Game not found.</p>
        ) : (
          <>
            <section>
              <div className="flex items-center gap-2 mb-2">
                <span
                  className="inline-block w-4 h-4 rounded-full shrink-0"
                  style={{ backgroundColor: pack.theme.pine }}
                  aria-hidden="true"
                />
                <h1 className="text-xl font-bold text-text">{pack.title}</h1>
              </div>
              {pack.tagline && <p className="text-muted mb-2">{pack.tagline}</p>}
              {pack.description && <p className="text-text text-sm leading-relaxed">{pack.description}</p>}
              <div className="mt-4">
                <Button variant="primary" onClick={create} disabled={creating}>
                  {creating ? 'Starting…' : 'New Game'}
                </Button>
              </div>
            </section>

            <section>
              <h2 className="text-lg font-semibold text-text mb-4">Plays</h2>
              {plays.length === 0 ? (
                <p className="text-muted">No plays yet.</p>
              ) : (
                <div className="space-y-2">
                  {plays.map(p => (
                    <Card key={p.play_id} className="flex items-center px-4 py-3 group">
                      <div
                        onClick={() => navigate(`/games/${p.play_id}`)}
                        className="flex-1 flex items-center justify-between cursor-pointer"
                      >
                        <span className="text-text">
                          {p.current_room_name
                            ? `Day ${p.day_number} — ${p.current_room_name}`
                            : `Play started ${fmtTime(p.created_at)}`}
                        </span>
                        <span className="text-faint text-xs">{fmtTime(p.updated_at)}</span>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setConfirmDelete(p.play_id)}
                        disabled={deleting === p.play_id}
                        className="ml-3 opacity-0 group-hover:opacity-100"
                      >
                        {deleting === p.play_id ? '...' : '✕'}
                      </Button>
                    </Card>
                  ))}
                </div>
              )}
            </section>
          </>
        )}
      </main>

      {confirmDelete && (
        <ConfirmDialog
          title="Delete play"
          message="Delete this play? This cannot be undone."
          confirmLabel="Delete"
          onConfirm={() => doDelete(confirmDelete)}
          onCancel={() => setConfirmDelete(null)}
        />
      )}
    </div>
  )
}
