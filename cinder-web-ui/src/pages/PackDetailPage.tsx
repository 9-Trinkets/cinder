import { useEffect, useState } from 'react'
import { useLocation, useNavigate, useParams } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import Button from '../components/Button'
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
      <header className="border-b border-subtle/60 bg-surface/90">
        <div className="max-w-2xl mx-auto px-4 sm:px-6 py-4 flex items-center justify-between">
          <button
            onClick={() => navigate('/games')}
            className="text-xs font-mono uppercase tracking-widest text-muted hover:text-text cursor-pointer flex items-center gap-1.5 transition-colors"
          >
            &lsaquo; Library
          </button>
          <Button variant="ghost" size="sm" onClick={logout} className="text-xs text-muted hover:text-text cursor-pointer">
            Log out
          </Button>
        </div>
      </header>

      <main className="max-w-2xl mx-auto px-4 sm:px-6 py-8 space-y-10">
        {loading ? (
          <Skeleton lines={3} />
        ) : error ? (
          <p className="text-love text-sm">{error}</p>
        ) : !pack ? (
          <p className="text-muted italic">Chronicle not found.</p>
        ) : (
          <>
            <article>
              <div className="mb-2">
                <span className="text-[10px] font-mono uppercase tracking-widest text-muted">
                  Story Pack &bull; {pack.tags?.join(' \u2022 ') || 'Interactive Fiction'}
                </span>
              </div>

              <h1 className="text-3xl sm:text-4xl font-bold font-prose text-text tracking-tight mb-3">
                {pack.title}
              </h1>

              {pack.tagline && (
                <blockquote className="my-4 pl-4 border-l-2 border-rose/60 text-base sm:text-lg italic text-text/90 font-prose leading-relaxed">
                  &ldquo;{pack.tagline}&rdquo;
                </blockquote>
              )}

              {pack.description && (
                <div className="text-base leading-[1.8] text-text/85 font-prose whitespace-pre-line my-6">
                  {pack.description}
                </div>
              )}

              <div className="pt-2 pb-6 border-b border-subtle/50">
                <Button
                  variant="primary"
                  onClick={create}
                  disabled={creating}
                  className="px-6 py-2.5 text-sm font-semibold tracking-wide shadow-xs cursor-pointer"
                >
                  {creating ? 'Opening Chronicle…' : '+ Begin New Chronicle'}
                </Button>
              </div>
            </article>

            <section>
              <div className="flex items-center justify-between mb-4">
                <div>
                  <span className="text-[10px] font-mono uppercase tracking-widest text-muted block mb-0.5">
                    Archive
                  </span>
                  <h2 className="text-xl font-bold font-prose text-text tracking-tight">
                    Open Chronicles
                  </h2>
                </div>
                {plays.length > 0 && (
                  <span className="text-[10px] font-mono uppercase tracking-widest text-foam bg-pine/15 px-2.5 py-1 rounded border border-pine/30">
                    {plays.length} {plays.length === 1 ? 'Reading' : 'Readings'}
                  </span>
                )}
              </div>

              {plays.length === 0 ? (
                <div className="text-center py-10 px-4 border border-dashed border-subtle/60 rounded-xl">
                  <p className="text-muted text-sm italic mb-1.5">No open chronicles for this tale.</p>
                  <p className="text-faint text-xs">Begin a new chronicle above to start reading.</p>
                </div>
              ) : (
                <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40">
                  {plays.map(p => (
                    <div
                      key={p.play_id}
                      className="py-3.5 flex items-center justify-between gap-4 group transition-colors"
                    >
                      <div
                        onClick={() => navigate(`/games/${p.play_id}`)}
                        className="flex-1 flex items-center justify-between gap-3 min-w-0 cursor-pointer"
                      >
                        <div className="min-w-0">
                          <p className="text-sm font-medium text-text group-hover:text-foam transition-colors truncate">
                            {p.current_room_name
                              ? `Day ${p.day_number} — ${p.current_room_name}`
                              : `Chronicle started ${fmtTime(p.created_at)}`}
                          </p>
                          <p className="text-xs text-muted/70 font-mono mt-0.5">
                            Updated {fmtTime(p.updated_at)}
                          </p>
                        </div>
                        <span className="text-xs font-medium text-foam group-hover:translate-x-1 transition-transform duration-150 inline-flex items-center gap-1 shrink-0">
                          Resume &rsaquo;
                        </span>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setConfirmDelete(p.play_id)}
                        disabled={deleting === p.play_id}
                        className="opacity-0 group-hover:opacity-100 text-muted hover:text-love transition-opacity text-xs"
                        title="Delete chronicle"
                      >
                        {deleting === p.play_id ? '…' : '✕'}
                      </Button>
                    </div>
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
