import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import Button from '../components/Button'
import Card from '../components/Card'
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
      <header className="flex items-center justify-between px-6 py-4 border-b border-subtle">
        <div className="flex items-center gap-2">
          <span className="w-2.5 h-2.5 rounded-full bg-rose animate-pulse" />
          <h1 className="text-xl font-bold tracking-wide text-rose">Cinder</h1>
        </div>
        <Button variant="ghost" onClick={logout}>Log out</Button>
      </header>

      <main className="max-w-3xl mx-auto px-4 py-8">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h2 className="text-xl font-bold text-text">Choose Your Adventure</h2>
            <p className="text-sm text-muted mt-0.5">Select a story pack or resume an active play</p>
          </div>
        </div>

        {loading ? (
          <Skeleton lines={4} />
        ) : error ? (
          <p className="text-love text-sm">{error}</p>
        ) : packs.length === 0 ? (
          <p className="text-muted">No games available.</p>
        ) : (
          <div className="grid gap-4 sm:grid-cols-2">
            {packs.map(pack => {
              const packPlays = plays.filter(p => p.pack_id === pack.id)
              const latestPlay = packPlays[0]

              return (
                <Card
                  key={pack.id}
                  className="p-5 cursor-pointer hover:border-text/30 hover:shadow-lg hover:-translate-y-0.5 transition-all duration-200 flex flex-col justify-between group"
                >
                  <button
                    onClick={() => navigate(`/games/pack/${pack.id}`)}
                    className="w-full text-left cursor-pointer flex-1 flex flex-col justify-between"
                  >
                    <div>
                      <div className="flex items-center justify-between gap-2 mb-2">
                        <div className="flex items-center gap-2 min-w-0">
                          <span
                            className="inline-block w-3 h-3 rounded-full shrink-0 shadow-xs"
                            style={{ backgroundColor: pack.theme?.pine || '#5a7a64' }}
                            aria-hidden="true"
                          />
                          <h3 className="text-text font-bold text-lg group-hover:text-foam transition-colors truncate">
                            {pack.title}
                          </h3>
                        </div>

                        {packPlays.length > 0 && (
                          <span className="px-2 py-0.5 rounded-full text-xs font-medium bg-pine/15 text-foam border border-pine/30 flex items-center gap-1.5 shrink-0">
                            <span className="w-1.5 h-1.5 rounded-full bg-foam animate-pulse" />
                            {packPlays.length} {packPlays.length === 1 ? 'play' : 'plays'}
                          </span>
                        )}
                      </div>

                      {pack.tags && pack.tags.length > 0 && (
                        <div className="flex flex-wrap gap-1.5 my-2.5">
                          {pack.tags.map(tag => (
                            <span
                              key={tag}
                              className="px-2 py-0.5 rounded text-[11px] font-medium bg-overlay text-muted border border-subtle/80 tracking-tight"
                            >
                              {tag}
                            </span>
                          ))}
                        </div>
                      )}

                      {pack.tagline && (
                        <p className="text-text/75 text-sm leading-relaxed mb-4 italic line-clamp-2">
                          "{pack.tagline}"
                        </p>
                      )}
                    </div>

                    <div className="pt-3 border-t border-subtle/40 flex items-center justify-between text-xs text-muted">
                      {latestPlay ? (
                        <span className="truncate max-w-[170px] text-faint">
                          Day {latestPlay.day_number} &bull; {latestPlay.current_room_name || 'In progress'}
                        </span>
                      ) : (
                        <span className="text-faint">Ready to begin</span>
                      )}
                      <span className="font-medium text-foam group-hover:translate-x-1 transition-transform duration-150 inline-flex items-center gap-1">
                        {latestPlay ? 'Resume' : 'Play'} &rarr;
                      </span>
                    </div>
                  </button>
                </Card>
              )
            })}
          </div>
        )}
      </main>
    </div>
  )
}
