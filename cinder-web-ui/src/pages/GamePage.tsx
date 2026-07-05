import { Component, useEffect, useState, useRef, type FormEvent, type ReactNode } from 'react'
import { useParams, useNavigate, useLocation } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import ShellMenu from '../components/ShellMenu'
import Modal from '../components/Modal'

class ErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null }
  static getDerivedStateFromError(error: Error) { return { error } }
  render() {
    if (this.state.error) {
      return (
        <div className="h-screen flex items-center justify-center bg-surface text-text p-8">
          <p className="text-love">Something went wrong. Please reload the page.</p>
        </div>
      )
    }
    return this.props.children
  }
}

interface Line {
  text: string
  key: number
}

interface PendingLine {
  text: string
  key: number
}

type MenuView = 'main' | 'about' | 'rooms' | 'follow' | 'language'

export default function GamePage() {
  const { id } = useParams<{ id: string }>()
  const { token, logout } = useAuth()
  const navigate = useNavigate()
  const location = useLocation()
  const sessionState = location.state as { title?: string; intro_text?: string } | null
  const [lines, setLines] = useState<Line[]>([])
  const [input, setInput] = useState('')
  const [gameOver, setGameOver] = useState(false)
  const [busy, setBusy] = useState(false)
  const [yelpReview, setYelpReview] = useState<api.YelpReviewData | null>(null)
  const [showMenu, setShowMenu] = useState(false)
  const [showLookModal, setShowLookModal] = useState(false)
  const [showTalkModal, setShowTalkModal] = useState(false)
  const [showOverflowModal, setShowOverflowModal] = useState(false)
  const [movie, setMovie] = useState<api.MovieData | null>(null)
  const [movieFrame, setMovieFrame] = useState(0)
  const [activeMenu, setActiveMenu] = useState<api.ActiveMenuData | null>(null)
  const [menuView, setMenuView] = useState<MenuView>('main')
  const [uiSnapshot, setUiSnapshot] = useState<api.UiSnapshot | null>(null)
  const channelSurfingOnly = useRef(false)
  const bottomRef = useRef<HTMLDivElement>(null)
  const nextKey = useRef(1)
  const initialized = useRef(false)
  const wsRef = useRef<WebSocket | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const [typewriterCharMs, setTypewriterCharMs] = useState(40)
  const [typewriterDisplay, setTypewriterDisplay] = useState<{ text: string; key: number } | null>(null)
  const pendingLines = useRef<PendingLine[]>([])
  const typewriterTimer = useRef<ReturnType<typeof setInterval> | null>(null)
  const charsRevealed = useRef(0)

  function startNextLine() {
    if (pendingLines.current.length === 0 || typewriterTimer.current) return
    const line = pendingLines.current[0]
    charsRevealed.current = 0
    setTypewriterDisplay({ text: '', key: line.key })
    typewriterTimer.current = setInterval(() => {
      charsRevealed.current++
      if (charsRevealed.current >= line.text.length) {
        if (typewriterTimer.current) {
          clearInterval(typewriterTimer.current)
          typewriterTimer.current = null
        }
        pendingLines.current.shift()
        setTypewriterDisplay(null)
        setLines(prev => [...prev, line])
        startNextLine()
      } else {
        setTypewriterDisplay({
          text: line.text.slice(0, charsRevealed.current),
          key: line.key,
        })
      }
    }, typewriterCharMs)
  }

  function flushTypewriter() {
    if (typewriterTimer.current) {
      clearInterval(typewriterTimer.current)
      typewriterTimer.current = null
    }
    if (pendingLines.current.length > 0) {
      setLines(prev => [...prev, pendingLines.current.shift()!])
    }
    while (pendingLines.current.length > 0) {
      const line = pendingLines.current.shift()!
      setLines(prev => [...prev, line])
    }
    setTypewriterDisplay(null)
  }

  function refreshSnapshot() {
    if (!token || !id) return
    api.fetchSessionUi(token, id).then(snap => {
      setUiSnapshot(snap)
      if (snap.active_menu) {
        setActiveMenu(snap.active_menu)
      } else {
        setActiveMenu(null)
      }
    }).catch(() => {})
  }

  useEffect(() => {
    return () => {
      if (typewriterTimer.current) clearInterval(typewriterTimer.current)
    }
  }, [])

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [lines, typewriterDisplay])

  useEffect(() => {
    if (initialized.current || !token || !id) return
    initialized.current = true
    setBusy(true)

    const entries: Line[] = []
    if (sessionState?.title) {
      entries.push({ text: `== ${sessionState.title} ==`, key: nextKey.current++ })
    }
    if (sessionState?.intro_text) {
      entries.push({ text: sessionState.intro_text, key: nextKey.current++ })
    }
    setLines(entries)

    api.fetchSessionUi(token, id)
      .then(snap => {
        channelSurfingOnly.current = snap.channel_surfing_only
        setUiSnapshot(snap)
      })
      .catch(() => {})

    api.fetchTranscript(token, id)
      .then(transcript => {
        if (transcript.length > 0) {
          setLines(prev => [
            ...prev,
            ...transcript.map(t => ({ text: t, key: nextKey.current++ })),
          ])
          setBusy(false)
          return false
        }
        return true
      })
      .catch(() => true)
      .then(shouldLook => {
        if (!shouldLook) return
        api.runCommand(token, id, 'look')
          .then(res => {
            setLines(prev => [...prev, { text: res.text, key: nextKey.current++ }])
            if (res.game_over) setGameOver(true)
          })
          .catch(err => {
            setLines(prev => [...prev, { text: `[error: ${err instanceof Error ? err.message : 'failed to load'}]`, key: nextKey.current++ }])
          })
          .finally(() => setBusy(false))
      })
  }, [token, id])

  useEffect(() => {
    if (!token || !id) return
    if (wsRef.current) return
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const wsUrl = `${protocol}//${window.location.host}/api/games/${id}/stream?token=${encodeURIComponent(token)}`
    const ws = new WebSocket(wsUrl)
    wsRef.current = ws
    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)
        if (data.type === 'settings') {
          setTypewriterCharMs(data.typewriter_char_ms ?? 40)
        } else if (data.type === 'movie') {
          setMovie(data as unknown as api.MovieData)
          setMovieFrame(0)
        } else if (data.type === 'tick' && data.text) {
          const paragraphs = data.text.split('\n\n')
            .map((p: string) => p.trim())
            .filter((p: string) => p.length > 0)
          if (paragraphs.length > 0) {
            for (const p of paragraphs) {
              pendingLines.current.push({ text: p, key: nextKey.current++ })
            }
            if (!typewriterTimer.current) {
              startNextLine()
            }
          }
        }
      } catch { /* ignore parse errors */ }
    }
    ws.onclose = () => { wsRef.current = null }
    return () => { ws.close(); wsRef.current = null }
  }, [token, id])

  useEffect(() => {
    if (gameOver) {
      refreshSnapshot()
    }
  }, [gameOver])

  useEffect(() => {
    if (gameOver && uiSnapshot?.yelp_review) {
      setYelpReview(uiSnapshot.yelp_review)
    }
  }, [gameOver, uiSnapshot])

  function openMenu() {
    setMenuView('main')
    setShowMenu(true)
    if (token && id) {
      api.fetchSessionUi(token, id).then(setUiSnapshot).catch(() => {})
    }
  }

  function addOutcome(text: string) {
    setLines(prev => [...prev, { text, key: nextKey.current++ }])
  }

  async function execCommand(cmd: string) {
    if (!token || !id || busy || gameOver) return
    setActiveMenu(null)
    setMovie(null)
    setMovieFrame(0)
    setBusy(true)
    wsRef.current?.send('pause')
    flushTypewriter()
    const cmdLine: Line = { text: `> ${cmd}`, key: nextKey.current++ }
    setLines(prev => [...prev, cmdLine])
    try {
      const res = await api.runCommand(token, id, cmd)
      wsRef.current?.send('resume')
      if (res.text) {
        const outLine: Line = { text: res.text, key: nextKey.current++ }
        setLines(prev => [...prev, outLine])
      }
      refreshSnapshot()
      if (res.movie) {
        setMovie(res.movie)
        setMovieFrame(0)
      }
      if (res.game_over) setGameOver(true)
    } catch (err: unknown) {
      wsRef.current?.send('resume')
      addOutcome(`[error: ${err instanceof Error ? err.message : 'request failed'}]`)
    } finally {
      setBusy(false)
    }
  }

  function closeMovie() {
    if (movie && movie.narrative_lines.length > 0) {
      setLines(prev => [
        ...prev,
        ...movie.narrative_lines.map(t => ({ text: t, key: nextKey.current++ })),
      ])
    }
    setMovie(null)
    setMovieFrame(0)
    refreshSnapshot()
  }

  async function doSwitchRoom(roomId: string) {
    if (!token || !id) return
    setShowMenu(false)
    setBusy(true)
    try {
      const res = await api.switchRoom(token, id, roomId)
      addOutcome(res.text)
      refreshSnapshot()
      if (res.game_over) setGameOver(true)
    } catch (err: unknown) {
      addOutcome(`[error: ${err instanceof Error ? err.message : 'request failed'}]`)
    } finally {
      setBusy(false)
    }
  }

  async function doFollowActor(actorId: string | null) {
    if (!token || !id) return
    setShowMenu(false)
    setBusy(true)
    try {
      const res = await api.followActor(token, id, actorId)
      addOutcome(res.text)
      refreshSnapshot()
      if (res.game_over) setGameOver(true)
    } catch (err: unknown) {
      addOutcome(`[error: ${err instanceof Error ? err.message : 'request failed'}]`)
    } finally {
      setBusy(false)
    }
  }

  async function doChangeLocale(locale: string) {
    if (!token || !id) return
    setShowMenu(false)
    setBusy(true)
    try {
      const text = await api.setLocale(token, id, locale)
      addOutcome(text)
    } catch (err: unknown) {
      addOutcome(`[error: ${err instanceof Error ? err.message : 'request failed'}]`)
    } finally {
      setBusy(false)
    }
  }

  function doExit() {
    if (confirm('Exit game?')) {
      navigate('/games')
    }
  }

  async function send(e: FormEvent) {
    e.preventDefault()
    if (!token || !id || busy || gameOver) return
    const trimmed = input.trim()
    if (!trimmed) return
    setInput('')
    if (trimmed === '?') { openMenu(); return }
    if (trimmed.toLowerCase() === 'move' || trimmed.toLowerCase() === 'follow') {
      const snap = uiSnapshot || await api.fetchSessionUi(token, id).catch(() => null)
      if (snap?.channel_surfing_only) {
        setUiSnapshot(snap)
        setMenuView(trimmed.toLowerCase() === 'move' ? 'rooms' : 'follow')
        setShowMenu(true)
        return
      }
    }
    await execCommand(trimmed)
  }

  return (
    <ErrorBoundary>
    <div className="h-screen flex flex-col bg-surface">
      <header className="flex items-center justify-between px-4 py-2 border-b border-subtle shrink-0">
        <div className="flex items-center gap-2">
          <button onClick={() => navigate('/games')} className="text-sm text-muted hover:text-text cursor-pointer">&larr; Sessions</button>
          <button
            onClick={openMenu}
            disabled={busy}
            className="text-sm px-2 py-1 rounded bg-overlay border border-subtle text-text hover:brightness-110 disabled:opacity-50 cursor-pointer"
          >&#9776; Menu</button>
        </div>
        <button onClick={logout} className="text-sm text-muted hover:text-love cursor-pointer">Log out</button>
      </header>

      <div className="flex-1 flex overflow-hidden">
        <div className="flex-1 flex flex-col overflow-hidden">
          <div className="flex-1 overflow-y-auto px-4 py-4 space-y-3">
            {lines.map(line => (
              <div key={line.key} className="whitespace-pre-wrap text-sm leading-relaxed">
                {line.text.startsWith('> ') ? (
                  <span className="text-foam">{line.text}</span>
                ) : line.text.startsWith('== ') ? (
                  <span className="text-iris font-bold">{line.text}</span>
                ) : (
                  <span className="text-text">{line.text}</span>
                )}
              </div>
            ))}
            {typewriterDisplay && (
              <div key={typewriterDisplay.key} className="whitespace-pre-wrap text-sm leading-relaxed">
                <span className="text-text">
                  {typewriterDisplay.text}
                  <span className="animate-pulse text-muted">▌</span>
                </span>
              </div>
            )}
            {busy && <p className="text-muted text-sm italic">...</p>}
            {yelpReview && (
              <div className="fixed inset-0 bg-base/80 flex items-center justify-center z-50">
                <div className="bg-surface rounded-xl p-6 max-w-md w-full mx-4 shadow-2xl">
                  <h2 className="text-xl font-bold text-center mb-2">Session Complete</h2>
                  <div className="flex justify-center gap-1 text-2xl mb-4">
                    {[1,2,3,4,5].map(n => (
                      <span key={n} className={n <= yelpReview.rating ? 'text-yellow-400' : 'text-muted'}>
                        {n <= yelpReview.rating ? '\u2605' : '\u2606'}
                      </span>
                    ))}
                  </div>
                  <p className="text-center text-sm text-muted mb-1">— Noa</p>
                  <p className="text-center text-balance leading-relaxed">{yelpReview.review_text}</p>
                  <button
                    className="mt-6 w-full py-2 bg-love text-white rounded-lg font-semibold hover:opacity-90"
                    onClick={() => setYelpReview(null)}
                  >
                    OK
                  </button>
                </div>
              </div>
            )}
            {gameOver && !yelpReview && (
              <p className="text-love font-semibold text-center pt-4">Game Over</p>
            )}
            <div ref={bottomRef} />
          </div>

          <div className="flex gap-2 px-4 py-2 border-t border-subtle shrink-0">
            {(uiSnapshot?.action_bar_actions ?? [
              { id: 'look', label: 'Look' },
              { id: 'move', label: 'Move' },
              { id: 'follow', label: 'Follow' },
            ]).map(action => {
              const handleClick = () => {
                if (busy || gameOver) return
                if (action.id === 'look') { flushTypewriter(); setShowLookModal(true); return }
                if (action.id === 'move') { setMenuView('rooms'); setShowMenu(true); return }
                if (action.id === 'follow') { setMenuView('follow'); setShowMenu(true); return }
                const talkOpts = uiSnapshot?.talk_options ?? []
                if ((action.id === 'speak' || action.id === 'talk') && talkOpts.length > 0) {
                  if (talkOpts.length === 1) {
                    setInput(`talk to ${talkOpts[0].title} `)
                    setTimeout(() => inputRef.current?.focus(), 0)
                    return
                  }
                  if (talkOpts.length > 1) { setShowTalkModal(true); return }
                }
                execCommand(action.id)
              }
              return (
                <button
                  key={action.id}
                  onClick={handleClick}
                  disabled={busy || gameOver}
                  className="px-3 py-1.5 rounded bg-overlay border border-subtle text-text text-sm hover:brightness-110 disabled:opacity-50 cursor-pointer"
                >{action.label}</button>
              )
            })}
            {uiSnapshot && uiSnapshot.overflow_actions?.length > 0 && (
              <button
                onClick={() => setShowOverflowModal(true)}
                disabled={busy || gameOver}
                className="px-3 py-1.5 rounded bg-overlay border border-subtle text-text text-sm hover:brightness-110 disabled:opacity-50 cursor-pointer"
                title="More actions"
              >...</button>
            )}
          </div>

          {!channelSurfingOnly.current && (
            <form onSubmit={send} className="flex gap-2 border-t border-subtle px-4 py-3 shrink-0">
              <input
                ref={inputRef}
                className="flex-1 px-3 py-2 rounded bg-overlay border border-subtle text-text placeholder-faint focus:outline-none focus:border-pine text-sm"
                placeholder={gameOver ? 'Game over' : 'What do you do?'}
                value={input}
                onChange={e => setInput(e.target.value)}
                disabled={busy || gameOver}
                autoFocus
              />
              <button
                type="submit"
                disabled={busy || gameOver || !input.trim()}
                className="px-4 py-2 rounded bg-pine text-surface text-sm font-semibold hover:brightness-110 disabled:opacity-50 cursor-pointer"
              >Send</button>
            </form>
          )}
        </div>

        {uiSnapshot && (
          <aside className="w-60 shrink-0 border-l border-subtle p-4 flex flex-col gap-4 text-sm overflow-y-auto">
            <div>
              <p className="text-xs text-muted uppercase tracking-wider">Location</p>
              <p className="text-text font-medium">{uiSnapshot.current_room_name}</p>
            </div>
            <div>
              <p className="text-xs text-muted uppercase tracking-wider">Time</p>
              <p className="text-text">
                Day {uiSnapshot.day_number}
                {uiSnapshot.time_label ? <span className="text-muted ml-1">— {uiSnapshot.time_label}</span> : null}
              </p>
            </div>
            {uiSnapshot.followed_actor_name && (
              <div>
                <p className="text-xs text-muted uppercase tracking-wider">Following</p>
                <p className="text-pine font-medium">{uiSnapshot.followed_actor_name}</p>
              </div>
            )}
            {uiSnapshot.inventory.length > 0 && (
              <div>
                <p className="text-xs text-muted uppercase tracking-wider">Inventory</p>
                <ul className="mt-1 space-y-0.5">
                  {uiSnapshot.inventory.map((item, i) => (
                    <li key={i} className="text-text text-xs">• {item}</li>
                  ))}
                </ul>
              </div>
            )}
            <div>
              <p className="text-xs text-muted uppercase tracking-wider">What now?</p>
              <p className="text-text text-xs leading-relaxed">
                {uiSnapshot.objective_message || 'No current objective.'}
              </p>
            </div>
            {uiSnapshot.progress_total > 0 && (
              <div>
                <p className="text-xs text-muted uppercase tracking-wider">Progress</p>
                <div className="mt-1 h-1.5 w-full bg-overlay rounded-full overflow-hidden">
                  <div
                    className="h-full bg-pine rounded-full transition-all duration-500"
                    style={{ width: `${(uiSnapshot.progress_completed / uiSnapshot.progress_total) * 100}%` }}
                  />
                </div>
              </div>
            )}
            {uiSnapshot.secrets_total > 0 && (
              <div>
                <p className="text-xs text-muted uppercase tracking-wider">Secrets Found</p>
                <p className="text-text font-medium">{uiSnapshot.secrets_found} / {uiSnapshot.secrets_total}</p>
              </div>
            )}
          </aside>
        )}
      </div>

      {showMenu && uiSnapshot && (
        <ShellMenu
          ui={uiSnapshot}
          view={menuView}
          onViewChange={setMenuView}
          onClose={() => setShowMenu(false)}
          onSwitchRoom={doSwitchRoom}
          onFollowActor={doFollowActor}
          onChangeLocale={doChangeLocale}
          onExit={doExit}
          busy={busy}
        />
      )}

      {showLookModal && uiSnapshot && (
        <Modal title="Look" onClose={() => setShowLookModal(false)}>
          {(uiSnapshot.look_options ?? []).length === 0 ? (
            <p className="text-muted italic">Nothing of particular interest here.</p>
          ) : (
            uiSnapshot.look_options.map(opt => (
              <button
                key={opt.id}
                onClick={async () => {
                  setShowLookModal(false)
                  await execCommand(opt.command)
                }}
                disabled={busy}
                className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
              >
                {opt.title}
              </button>
            ))
          )}
        </Modal>
      )}

      {showTalkModal && uiSnapshot && (
        <Modal title="Talk" onClose={() => setShowTalkModal(false)}>
          <p className="text-sm text-muted mb-3">Who do you want to talk to?</p>
          {uiSnapshot.talk_options.map(opt => (
            <button
              key={opt.id}
              onClick={() => {
                setShowTalkModal(false)
                setInput(`talk to ${opt.title} `)
                setTimeout(() => inputRef.current?.focus(), 0)
              }}
              disabled={busy}
              className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
            >
              {opt.title}
            </button>
          ))}
        </Modal>
      )}

      {activeMenu && (
        <Modal title="Choose" onClose={() => setActiveMenu(null)}>
          {activeMenu.prompt && (
            <p className="text-sm text-text whitespace-pre-wrap mb-3">{activeMenu.prompt}</p>
          )}
          {activeMenu.options.length === 0 ? (
            <p className="text-muted italic">No options available.</p>
          ) : (
            activeMenu.options.map((opt, i) => (
              <button
                key={opt.id}
                onClick={async () => {
                  await execCommand((i + 1).toString())
                }}
                disabled={busy}
                className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
              >
                <span className="text-muted mr-2">{(i + 1).toString()}.</span>
                <span className="font-medium">{opt.title}</span>
                {opt.menu_text && <span className="text-muted ml-2">— {opt.menu_text}</span>}
              </button>
            ))
          )}
        </Modal>
      )}

      {showOverflowModal && uiSnapshot && (
        <Modal title="Commands" onClose={() => setShowOverflowModal(false)}>
          {groupOverflowActions(uiSnapshot.overflow_actions ?? []).map(([group, items]) => (
            <div key={group} className="mb-4 last:mb-0">
              <h3 className="text-xs font-semibold text-muted uppercase tracking-wider mb-2">{group}</h3>
              {items.map(action => (
                <button
                  key={action.id}
                  onClick={async () => {
                    setShowOverflowModal(false)
                    await execCommand(action.id)
                  }}
                  disabled={busy}
                  className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer mb-1 last:mb-0"
                  title={action.usage}
                >
                  <span className="font-medium">{action.label}</span>
                  {action.usage && <span className="text-muted text-xs ml-2">— {action.usage}</span>}
                </button>
              ))}
            </div>
          ))}
          {uiSnapshot.overflow_actions?.length === 0 && (
            <p className="text-muted italic">No additional commands available.</p>
          )}
        </Modal>
      )}

      {movie && (
        <MovieModal
          movie={movie}
          frame={movieFrame}
          onAdvance={() => {
            if (movieFrame < movie.frames.length - 1) {
              setMovieFrame(prev => prev + 1)
            } else {
              closeMovie()
            }
          }}
          onClose={closeMovie}
        />
      )}
    </div>
    </ErrorBoundary>
  )
}

function MovieModal({ movie, frame, onAdvance, onClose }: {
  movie: api.MovieData
  frame: number
  onAdvance: () => void
  onClose: () => void
}) {
  const f = movie.frames[frame]
  const isLast = frame >= movie.frames.length - 1

  useEffect(() => {
    if (!f || isLast) return
    const timer = setTimeout(onAdvance, Math.max(300, f.duration_ms))
    return () => clearTimeout(timer)
  }, [frame, f, isLast, onAdvance])

  if (!f) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black" onClick={onClose}>
      <div
        className="max-w-2xl w-full mx-4"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-base font-semibold text-text">{movie.title}</h2>
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted">{frame + 1} / {movie.frames.length}</span>
            <button onClick={onClose} className="text-muted hover:text-text text-lg leading-none cursor-pointer">&times;</button>
          </div>
        </div>
        <pre className="text-pine text-xs leading-none whitespace-pre-wrap font-mono bg-black/40 rounded-lg p-4 max-h-[60vh] overflow-y-auto border border-subtle select-none">
          {f.text}
        </pre>
        {isLast && (
          <p className="text-center text-muted text-sm mt-3 animate-pulse cursor-pointer" onClick={onClose}>Click or tap to continue...</p>
        )}
      </div>
    </div>
  )
}

function groupOverflowActions(actions: api.OverflowAction[]): [string, api.OverflowAction[]][] {
  const map = new Map<string, api.OverflowAction[]>()
  for (const a of actions) {
    const g = a.group || 'Other'
    if (!map.has(g)) map.set(g, [])
    map.get(g)!.push(a)
  }
  return Array.from(map.entries()).sort(([a], [b]) => a.localeCompare(b))
}
