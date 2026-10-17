import { useEffect, useState } from 'react'
import { useLocation, useNavigate, useParams } from 'react-router-dom'
import { useAuth } from '../auth'
import { useI18n } from '../i18n'
import * as api from '../api'
import Button from '../components/Button'
import ConfirmDialog from '../components/ConfirmDialog'
import Skeleton from '../components/Skeleton'
import { useToast } from '../components/Toast'
import { toErrorMessage } from '../utils/error'
import { themeVars } from '../utils/theme'

function fmtTime(s: string, locale: string): string {
  const n = Number(s)
  if (!isNaN(n) && n > 1e8) {
    return new Date(n * 1000).toLocaleString(locale)
  }
  const d = new Date(s)
  if (!isNaN(d.getTime())) return d.toLocaleString(locale)
  return s
}

export default function PackDetailPage() {
  const { packId } = useParams<{ packId: string }>()
  const { token, logout } = useAuth()
  const { locale, t } = useI18n()
  const navigate = useNavigate()
  const location = useLocation()
  const { showToast } = useToast()
  const [pack, setPack] = useState<api.PackInfo | null>(null)
  const [plays, setPlays] = useState<api.PlayInfo[]>([])
  const [bookmarks, setBookmarks] = useState<api.BookmarkInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [creating, setCreating] = useState(false)
  const [deleting, setDeleting] = useState<string | null>(null)
  const [deletingBookmark, setDeletingBookmark] = useState<string | null>(null)
  const [confirmDeleteBookmark, setConfirmDeleteBookmark] = useState<string | null>(null)
  const [resumingBookmark, setResumingBookmark] = useState<string | null>(null)
  const [confirmResumeBookmark, setConfirmResumeBookmark] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null)

  async function load() {
    if (!token || !packId) return
    setError(null)
    try {
      const [packs, allPlays, packBookmarks] = await Promise.all([
        api.listPacks(token),
        api.listPlays(token),
        api.listPackBookmarks(token, packId).catch(() => [] as api.BookmarkInfo[]),
      ])
      setPack(packs.find(p => p.id === packId) ?? null)
      setPlays(allPlays.filter(p => p.pack_id === packId))
      setBookmarks(packBookmarks)
    } catch (err) {
      setError(toErrorMessage(err, t('common.failedToLoad')))
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
      showToast(toErrorMessage(err, t('pack.failedDelete')), 'error')
    } finally {
      setDeleting(null)
    }
  }

  async function doResumeBookmark(bookmarkId: string) {
    if (!token || resumingBookmark) return
    setConfirmResumeBookmark(null)
    setResumingBookmark(bookmarkId)
    try {
      const res = await api.resumeBookmark(token, bookmarkId)
      showToast(t('pack.toastReturned'), 'info')
      navigate(`/games/${res.play_id}`, { state: { title: res.snapshot?.title } })
    } catch (err: unknown) {
      showToast(toErrorMessage(err, t('pack.failedReturn')), 'error')
    } finally {
      setResumingBookmark(null)
    }
  }

  async function doDeleteBookmark(bookmarkId: string) {
    if (!token || deletingBookmark) return
    setConfirmDeleteBookmark(null)
    setDeletingBookmark(bookmarkId)
    try {
      await api.deleteBookmark(token, bookmarkId)
      setBookmarks(prev => prev.filter(b => b.id !== bookmarkId))
      showToast(t('pack.toastBookmarkDeleted'), 'info')
    } catch (err: unknown) {
      showToast(toErrorMessage(err, t('pack.failedDeleteBookmark')), 'error')
    } finally {
      setDeletingBookmark(null)
    }
  }

  async function create() {
    if (!token || !packId) return
    setCreating(true)
    try {
      const play = await api.createPlay(token, packId, locale)
      navigate(`/games/${play.play_id}`, { state: { title: play.title, intro_text: play.intro_text } })
    } catch (err: unknown) {
      showToast(toErrorMessage(err, t('pack.failedCreate')), 'error')
    } finally {
      setCreating(false)
    }
  }

  const supported = pack ? pack.locale_options.some(o => o.code === locale) : true
  const localeLabel = pack?.locale_options.find(o => o.code === locale)?.label ?? locale

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
            &lsaquo; {t('common.library')}
          </button>
          <Button variant="ghost" size="sm" onClick={logout} className="text-xs text-muted hover:text-text cursor-pointer">
            {t('common.logout')}
          </Button>
        </div>
      </header>

      <main className="max-w-2xl mx-auto px-4 sm:px-6 py-8 space-y-10">
        {loading ? (
          <Skeleton lines={3} />
        ) : error ? (
          <p className="text-love text-sm">{error}</p>
        ) : !pack ? (
          <p className="text-muted italic">{t('pack.notFound')}</p>
        ) : (
          <>
            <article>
              <div className="mb-2">
                <span className="text-[10px] font-mono uppercase tracking-widest text-muted">
                  {t('pack.storyPack', {
                    tags: pack.tags?.join(' \u2022 ') || t('pack.interactiveFiction'),
                  })}
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
                {plays.length > 0 ? (
                  <div className="flex flex-col sm:flex-row sm:items-center gap-3">
                    <Button
                      variant="primary"
                      onClick={() => navigate(`/games/${plays[0].play_id}`)}
                      className="px-6 py-2.5 text-sm font-semibold tracking-wide shadow-xs cursor-pointer"
                    >
                      {t('pack.resumeReading')}
                    </Button>
                    <span className="text-xs text-muted">
                      {t('pack.inProgressNote')}
                    </span>
                  </div>
                ) : (
                  <div className="flex flex-col gap-2">
                    <Button
                      variant="primary"
                      onClick={create}
                      disabled={creating || !supported}
                      className="px-6 py-2.5 text-sm font-semibold tracking-wide shadow-xs cursor-pointer self-start"
                    >
                      {creating ? t('pack.opening') : t('pack.begin')}
                    </Button>
                    {!supported && (
                      <span className="text-xs text-muted">
                        {t('pack.unsupportedNote', { language: localeLabel })}
                      </span>
                    )}
                  </div>
                )}
              </div>
            </article>

            <section>
              <div className="flex items-center justify-between mb-4">
                <div>
                  <span className="text-[10px] font-mono uppercase tracking-widest text-muted block mb-0.5">
                    {t('pack.reading')}
                  </span>
                  <h2 className="text-xl font-bold font-prose text-text tracking-tight">
                    {t('pack.ongoing')}
                  </h2>
                </div>
                {plays.length > 0 && (
                  <span className="text-[10px] font-mono uppercase tracking-widest text-foam bg-pine/15 px-2.5 py-1 rounded border border-pine/30">
                    {t('common.active')}
                  </span>
                )}
              </div>

              {plays.length === 0 ? (
                <div className="text-center py-8 px-4 border border-dashed border-subtle/60 rounded-xl">
                  <p className="text-muted text-sm italic mb-1.5">{t('pack.noActive')}</p>
                  <p className="text-faint text-xs">{t('pack.beginHint')}</p>
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
                              ? p.current_room_name
                              : t('pack.started', { time: fmtTime(p.created_at, locale) })}
                          </p>
                          <p className="text-xs text-muted/70 font-mono mt-0.5">
                            {[
                              p.day_number !== null && p.day_number !== undefined ? t('pack.day', { day: p.day_number }) : null,
                              t('pack.updated', { time: fmtTime(p.updated_at, locale) }),
                            ].filter(Boolean).join(' • ')}
                          </p>
                        </div>
                        <span className="text-xs font-medium text-foam group-hover:translate-x-1 transition-transform duration-150 inline-flex items-center gap-1 shrink-0">
                          {t('pack.resume')}
                        </span>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setConfirmDelete(p.play_id)}
                        disabled={deleting === p.play_id}
                        className="opacity-0 group-hover:opacity-100 text-muted hover:text-love transition-opacity text-xs"
                        title={t('pack.deleteReadingTitle')}
                      >
                        {deleting === p.play_id ? '…' : '✕'}
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </section>

            <section>
              <div className="flex items-center justify-between mb-4">
                <div>
                  <span className="text-[10px] font-mono uppercase tracking-widest text-muted block mb-0.5">
                    {t('pack.bookmarks')}
                  </span>
                  <h2 className="text-xl font-bold font-prose text-text tracking-tight">
                    {t('pack.bookmarks')}
                  </h2>
                </div>
                {bookmarks.length > 0 && (
                  <span className="text-[10px] font-mono uppercase tracking-widest text-foam bg-pine/15 px-2.5 py-1 rounded border border-pine/30">
                    {t(bookmarks.length === 1 ? 'pack.bookmarkOne' : 'pack.bookmarkMany', {
                      count: bookmarks.length,
                    })}
                  </span>
                )}
              </div>

              {bookmarks.length === 0 ? (
                <div className="text-center py-8 px-4 border border-dashed border-subtle/60 rounded-xl">
                  <p className="text-muted text-sm italic mb-1.5">{t('pack.noBookmarks')}</p>
                  <p className="text-faint text-xs">{t('pack.bookmarkHint')}</p>
                </div>
              ) : (
                <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40">
                  {bookmarks.map(b => (
                    <div
                      key={b.id}
                      className="py-3.5 flex items-center justify-between gap-4 group transition-colors"
                    >
                      <div
                        onClick={() => setConfirmResumeBookmark(b.id)}
                        className="flex-1 flex items-center justify-between gap-3 min-w-0 cursor-pointer"
                      >
                        <div className="min-w-0">
                          <p className="text-sm font-medium text-text group-hover:text-foam transition-colors truncate">
                            {b.current_room_name || t('pack.bookmarkFallback')}
                          </p>
                          <p className="text-xs text-muted/70 font-mono mt-0.5">
                            {[
                              b.day_number !== null && b.day_number !== undefined ? t('pack.day', { day: b.day_number }) : null,
                              b.time_label,
                              b.turn_number !== null && b.turn_number !== undefined ? t('pack.turn', { turn: b.turn_number }) : null,
                              fmtTime(b.created_at, locale),
                            ].filter(Boolean).join(' • ')}
                          </p>
                        </div>
                        <span className="text-xs font-medium text-foam group-hover:translate-x-1 transition-transform duration-150 inline-flex items-center gap-1 shrink-0">
                          {t('pack.returnTo')}
                        </span>
                      </div>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => setConfirmDeleteBookmark(b.id)}
                        disabled={deletingBookmark === b.id}
                        className="opacity-0 group-hover:opacity-100 text-muted hover:text-love transition-opacity text-xs"
                        title={t('pack.deleteBookmarkTitle')}
                      >
                        {deletingBookmark === b.id ? '…' : '✕'}
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
          title={t('pack.confirmDeleteTitle')}
          message={t('pack.confirmDeleteMessage')}
          confirmLabel={t('pack.confirmDelete')}
          onConfirm={() => doDelete(confirmDelete)}
          onCancel={() => setConfirmDelete(null)}
        />
      )}

      {confirmResumeBookmark && (
        <ConfirmDialog
          title={t('pack.confirmResumeTitle')}
          message={t('pack.confirmResumeMessage')}
          confirmLabel={t('pack.confirmReturn')}
          onConfirm={() => doResumeBookmark(confirmResumeBookmark)}
          onCancel={() => setConfirmResumeBookmark(null)}
        />
      )}

      {confirmDeleteBookmark && (
        <ConfirmDialog
          title={t('pack.confirmDeleteBookmarkTitle')}
          message={t('pack.confirmDeleteBookmarkMessage')}
          confirmLabel={t('pack.confirmDelete')}
          onConfirm={() => doDeleteBookmark(confirmDeleteBookmark)}
          onCancel={() => setConfirmDeleteBookmark(null)}
        />
      )}
    </div>
  )
}
