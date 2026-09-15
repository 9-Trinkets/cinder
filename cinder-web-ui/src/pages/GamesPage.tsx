import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import Button from '../components/Button'
import Skeleton from '../components/Skeleton'
import { toErrorMessage } from '../utils/error'

export default function GamesPage() {
  const { token, logout } = useAuth()
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
      .catch(err => setError(toErrorMessage(err, 'failed to load')))
      .finally(() => setLoading(false))
  }, [token])

  return (
    <div className="min-h-screen bg-surface font-prose">
      <header className="border-b border-subtle/60 bg-surface/90">
        <div className="max-w-2xl mx-auto px-4 sm:px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <span className="w-2 h-2 rounded-full bg-rose shrink-0" aria-hidden="true" />
            <span className="font-mono text-xs font-semibold tracking-widest uppercase text-rose">
              Cinder
            </span>
          </div>
          <Button variant="ghost" size="sm" onClick={logout} className="text-xs text-muted hover:text-text cursor-pointer">
            Log out
          </Button>
        </div>
      </header>

      <main className="max-w-2xl mx-auto px-4 sm:px-6 py-8">
        <div className="mb-8 pb-6 border-b border-subtle/50">
          <span className="text-[10px] font-mono uppercase tracking-widest text-muted block mb-1.5">
            Anthology &bull; Collected Tales
          </span>
          <h1 className="text-2xl sm:text-3xl font-bold font-prose text-text tracking-tight">
            The Library
          </h1>
          <p className="text-sm sm:text-base text-muted font-prose mt-1.5 leading-relaxed">
            Select a chronicle to explore, or resume an open journey.
          </p>
        </div>

        {loading ? (
          <Skeleton lines={4} />
        ) : error ? (
          <p className="text-love text-sm">{error}</p>
        ) : packs.length === 0 ? (
          <p className="text-muted italic">No tales are currently archived in this library.</p>
        ) : (
          <div className="divide-y divide-subtle/50">
            {packs.map(pack => {
              const packPlays = plays.filter(p => p.pack_id === pack.id)
              const latestPlay = packPlays[0]

              return (
                <article
                  key={pack.id}
                  onClick={() => navigate(`/games/pack/${pack.id}`)}
                  className="py-6 sm:py-8 group cursor-pointer transition-colors duration-150"
                >
                  <div className="flex items-start justify-between gap-4 mb-2">
                    <div className="min-w-0">
                      {pack.tags && pack.tags.length > 0 && (
                        <p className="font-mono text-[10px] uppercase tracking-widest text-muted mb-1.5 truncate">
                          {pack.tags.join(' \u2022 ')}
                        </p>
                      )}
                      <h2 className="text-xl sm:text-2xl font-bold font-prose text-text group-hover:text-foam transition-colors tracking-tight truncate">
                        {pack.title}
                      </h2>
                    </div>

                    {packPlays.length > 0 ? (
                      <span className="shrink-0 text-[10px] font-mono uppercase tracking-widest text-foam bg-pine/15 px-2.5 py-1 rounded border border-pine/30">
                        {packPlays.length} {packPlays.length === 1 ? 'Chronicle' : 'Chronicles'}
                      </span>
                    ) : (
                      <span className="shrink-0 text-[10px] font-mono uppercase tracking-widest text-muted/70 bg-overlay px-2.5 py-1 rounded border border-subtle/60">
                        Unopened
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
                        Last reading: Day {latestPlay.day_number} &bull; {latestPlay.current_room_name || 'In progress'}
                      </span>
                    ) : (
                      <span className="text-muted/60 text-xs">Awaiting first reading</span>
                    )}
                    <span className="font-medium text-foam group-hover:translate-x-1 transition-transform duration-150 inline-flex items-center gap-1.5 shrink-0">
                      {latestPlay ? 'Resume Tale' : 'Read Story'} &rsaquo;
                    </span>
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
