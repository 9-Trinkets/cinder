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
import { themeVars } from '../utils/theme'

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
    <div
      style={pack?.theme ? themeVars(pack.theme) : undefined}
      className="min-h-screen bg-surface font-prose"
    >
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
            <section className="bg-surface/50 border border-subtle/80 rounded-xl p-6 shadow-xs">
              <div className="flex items-center gap-2.5 mb-2">
                <span
                  className="inline-block w-4 h-4 rounded-full shrink-0 shadow-xs"
                  style={{ backgroundColor: pack.theme.pine }}
                  aria-hidden="true"
                />
                <h1 className="text-2xl font-bold text-text tracking-tight">{pack.title}</h1>
              </div>

              {pack.tags && pack.tags.length > 0 && (
                <div className="flex flex-wrap gap-1.5 mb-3">
                  {pack.tags.map(tag => (
                    <span
                      key={tag}
                      className="px-2.5 py-0.5 rounded-md text-xs font-medium bg-overlay text-foam border border-subtle"
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              )}

              {pack.tagline && (
                <p className="text-iris font-medium text-base mb-3 italic">
                  "{pack.tagline}"
                </p>
              )}

              {pack.description && (
                <div className="text-text/85 text-sm leading-relaxed whitespace-pre-line bg-overlay/30 p-4 rounded-lg border border-subtle/40 mb-5">
                  {pack.description}
                </div>
              )}

              <div>
                <Button variant="primary" onClick={create} disabled={creating} className="px-5 py-2 font-semibold shadow-xs">
                  {creating ? 'Starting…' : '+ Start New Game'}
                </Button>
              </div>
            </section>

            <section>
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-lg font-bold text-text">Plays</h2>
                {plays.length > 0 && (
                  <span className="text-xs font-medium text-muted bg-overlay px-2 py-0.5 rounded border border-subtle">
                    {plays.length} {plays.length === 1 ? 'play' : 'plays'}
                  </span>
                )}
              </div>

              {plays.length === 0 ? (
                <div className="text-center py-8 px-4 rounded-xl border border-dashed border-subtle bg-overlay/20">
                  <p className="text-muted text-sm mb-2">No active plays yet.</p>
                  <p className="text-faint text-xs">Click "+ Start New Game" above to begin your journey!</p>
                </div>
              ) : (
                <div className="space-y-2">
                  {plays.map(p => (
                    <Card
                      key={p.play_id}
                      className="flex items-center px-4 py-3 group hover:border-text/30 hover:bg-highlight-low/10 transition-all duration-150"
                    >
                      <div
                        onClick={() => navigate(`/games/${p.play_id}`)}
                        className="flex-1 flex items-center justify-between cursor-pointer gap-3 min-w-0"
                      >
                        <div className="flex items-center gap-2.5 min-w-0">
                          <span className="w-2 h-2 rounded-full bg-foam shrink-0 animate-pulse" />
                          <span className="text-text font-medium text-sm truncate">
                            {p.current_room_name
                              ? `Day ${p.day_number} — ${p.current_room_name}`
                              : `Play started ${fmtTime(p.created_at)}`}
                          </span>
                        </div>
                        <div className="flex items-center gap-3 shrink-0">
                          <span className="text-faint text-xs font-mono">{fmtTime(p.updated_at)}</span>
                          <span className="text-xs font-medium text-foam group-hover:translate-x-0.5 transition-transform duration-150">
                            Resume &rarr;
                          </span>
                        </div>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setConfirmDelete(p.play_id)}
                        disabled={deleting === p.play_id}
                        className="ml-2 opacity-0 group-hover:opacity-100 text-muted hover:text-love transition-opacity"
                        title="Delete play"
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
