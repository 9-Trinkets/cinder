import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../auth'
import { useI18n } from '../i18n'
import * as api from '../api'
import Button from '../components/Button'
import Skeleton from '../components/Skeleton'
import LanguagePicker from '../components/LanguagePicker'
import { toErrorMessage } from '../utils/error'

export default function GamesPage() {
  const { token, logout } = useAuth()
  const { locale, t } = useI18n()
  const navigate = useNavigate()
  const [packs, setPacks] = useState<api.PackInfo[]>([])
  const [plays, setPlays] = useState<api.PlayInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!token) return
    Promise.all([
      api.listPacks(token),
      api.listPlays(token).catch(() => [] as api.PlayInfo[]),
    ])
      .then(([packList, playList]) => {
        setPacks(packList)
        setPlays(playList)
      })
      .catch(err => setError(toErrorMessage(err, t('common.failedToLoad'))))
      .finally(() => setLoading(false))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token])

  return (
    <div className="min-h-screen bg-surface font-prose">
      <header className="border-b border-subtle/60 bg-surface/90">
        <div className="max-w-2xl mx-auto px-4 sm:px-6 py-4 flex items-center justify-between gap-3">
          <div className="flex items-center gap-2.5">
            <span className="w-2 h-2 rounded-full bg-rose shrink-0" aria-hidden="true" />
            <span className="font-mono text-xs font-semibold tracking-widest uppercase text-rose">
              Cinder
            </span>
          </div>
          <div className="flex items-center gap-4">
            <LanguagePicker />
            <Button variant="ghost" size="sm" onClick={logout} className="text-xs text-muted hover:text-text cursor-pointer">
              {t('common.logout')}
            </Button>
          </div>
        </div>
      </header>

      <main className="max-w-2xl mx-auto px-4 sm:px-6 py-8">
        <div className="mb-8 pb-6 border-b border-subtle/50">
          <span className="text-[10px] font-mono uppercase tracking-widest text-muted block mb-1.5">
            {t('games.anthology')}
          </span>
          <h1 className="text-2xl sm:text-3xl font-bold font-prose text-text tracking-tight">
            {t('games.heading')}
          </h1>
          <p className="text-sm sm:text-base text-muted font-prose mt-1.5 leading-relaxed">
            {t('games.subtitle')}
          </p>
        </div>

        {loading ? (
          <Skeleton lines={4} />
        ) : error ? (
          <p className="text-love text-sm">{error}</p>
        ) : packs.length === 0 ? (
          <p className="text-muted italic">{t('games.empty')}</p>
        ) : (
          <div className="divide-y divide-subtle/50">
            {packs.map(pack => {
              const packPlays = plays.filter(p => p.pack_id === pack.id)
              const latestPlay = packPlays[0]
              const supported = pack.locale_options.some(o => o.code === locale)
              const localeLabel = pack.locale_options.find(o => o.code === locale)?.label ?? locale

              return (
                <article
                  key={pack.id}
                  onClick={() => supported && navigate(`/games/pack/${pack.id}`)}
                  aria-disabled={!supported}
                  className={`py-6 sm:py-8 group transition-colors duration-150 ${
                    supported ? 'cursor-pointer' : 'cursor-not-allowed opacity-50'
                  }`}
                >
                  <div className="flex items-start justify-between gap-4 mb-2">
                    <div className="min-w-0">
                      {pack.tags && pack.tags.length > 0 && (
                        <p className="font-mono text-[10px] uppercase tracking-widest text-muted mb-1.5 truncate">
                          {pack.tags.join(' \u2022 ')}
                        </p>
                      )}
                      <h2
                        className={`text-xl sm:text-2xl font-bold font-prose text-text tracking-tight truncate ${
                          supported ? 'group-hover:text-foam transition-colors' : ''
                        }`}
                      >
                        {pack.title}
                      </h2>
                    </div>

                    {!supported ? (
                      <span className="shrink-0 text-[10px] font-mono uppercase tracking-widest text-muted bg-overlay px-2.5 py-1 rounded border border-subtle/60 whitespace-nowrap">
                        {t('games.unsupportedBadge', { language: localeLabel })}
                      </span>
                    ) : packPlays.length > 0 ? (
                      <span className="shrink-0 text-[10px] font-mono uppercase tracking-widest text-foam bg-pine/15 px-2.5 py-1 rounded border border-pine/30">
                        {t(packPlays.length === 1 ? 'games.chronicleOne' : 'games.chronicleMany', {
                          count: packPlays.length,
                        })}
                      </span>
                    ) : (
                      <span className="shrink-0 text-[10px] font-mono uppercase tracking-widest text-muted/70 bg-overlay px-2.5 py-1 rounded border border-subtle/60">
                        {t('games.unopened')}
                      </span>
                    )}
                  </div>

                  {pack.tagline && (
                    <blockquote className="my-3 pl-3.5 border-l-2 border-subtle/60 text-sm sm:text-base italic text-text/80 leading-relaxed font-prose">
                      &ldquo;{pack.tagline}&rdquo;
                    </blockquote>
                  )}

                  <div className="mt-4 flex items-center justify-between text-xs text-muted">
                    {latestPlay ? (
                      <span className="text-faint font-mono text-xs truncate max-w-[280px]">
                        {t('games.lastReading', {
                          day: latestPlay.day_number,
                          room: latestPlay.current_room_name || t('games.inProgress'),
                        })}
                      </span>
                    ) : (
                      <span className="text-muted/60 text-xs">{t('games.awaiting')}</span>
                    )}
                    {supported && (
                      <span className="font-medium text-foam group-hover:translate-x-1 transition-transform duration-150 inline-flex items-center gap-1.5 shrink-0">
                        {latestPlay ? t('games.resume') : t('games.read')} &rsaquo;
                      </span>
                    )}
                  </div>
                </article>
              )
            })}
          </div>
        )}
      </main>
    </div>
  )
}
