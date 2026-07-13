import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import Button from '../components/Button'
import Card from '../components/Card'
import Skeleton from '../components/Skeleton'

export default function GamesPage() {
  const { token, logout } = useAuth()
  const navigate = useNavigate()
  const [packs, setPacks] = useState<api.PackInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!token) return
    api.listPacks(token)
      .then(setPacks)
      .catch(err => setError(err instanceof Error ? err.message : 'failed to load'))
      .finally(() => setLoading(false))
  }, [token])

  return (
    <div className="min-h-screen bg-surface">
      <header className="flex items-center justify-between px-6 py-4 border-b border-subtle">
        <h1 className="text-xl font-bold text-rose">Cinder</h1>
        <Button variant="ghost" onClick={logout}>Log out</Button>
      </header>

      <main className="max-w-2xl mx-auto px-4 py-8">
        <h2 className="text-lg font-semibold text-text mb-4">Games</h2>
        {loading ? (
          <Skeleton lines={3} />
        ) : error ? (
          <p className="text-love text-sm">{error}</p>
        ) : packs.length === 0 ? (
          <p className="text-muted">No games available.</p>
        ) : (
          <div className="grid gap-3 sm:grid-cols-2">
            {packs.map(pack => (
              <Card
                key={pack.id}
                className="p-4 cursor-pointer hover:brightness-110 transition duration-200"
              >
                <button
                  onClick={() => navigate(`/games/pack/${pack.id}`)}
                  className="w-full text-left cursor-pointer"
                >
                  <div className="flex items-center gap-2 mb-2">
                    <span
                      className="inline-block w-3 h-3 rounded-full shrink-0"
                      style={{ backgroundColor: pack.theme.pine }}
                      aria-hidden="true"
                    />
                    <h3 className="text-text font-semibold">{pack.title}</h3>
                  </div>
                  {pack.tagline && (
                    <p className="text-muted text-sm">{pack.tagline}</p>
                  )}
                </button>
              </Card>
            ))}
          </div>
        )}
      </main>
    </div>
  )
}
