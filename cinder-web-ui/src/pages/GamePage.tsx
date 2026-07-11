import { Component, memo, useCallback, useEffect, useState, useRef, type FormEvent, type MutableRefObject, type ReactNode, type UIEvent } from 'react'
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
        <div className="min-h-dvh flex items-center justify-center bg-surface text-text p-8">
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

type MenuView = 'main' | 'about' | 'rooms' | 'follow' | 'language'
type QuickPanel = 'look' | 'talk' | 'overflow' | 'rooms' | 'follow' | null

export default function GamePage() {
  const { id } = useParams<{ id: string }>()
  const { token, logout } = useAuth()
  const navigate = useNavigate()
  const location = useLocation()
  const sessionState = location.state as { title?: string; intro_text?: string } | null
  const [lines, setLines] = useState<Line[]>([])
  const [input, setInput] = useState('')
  const [gameOver, setGameOver] = useState(false)
  const [initializing, setInitializing] = useState(false)
  const [commandPending, setCommandPending] = useState(false)
  const [panelBusy, setPanelBusy] = useState(false)
  const [sessionClosure, setSessionClosure] = useState<api.SessionClosureData | null>(null)
  const [showMenu, setShowMenu] = useState(false)
  const [quickPanel, setQuickPanel] = useState<QuickPanel>(null)
  const [showStatusModal, setShowStatusModal] = useState(false)
  const [movie, setMovie] = useState<api.MovieData | null>(null)
  const [movieFrame, setMovieFrame] = useState(0)
  const [activeMenu, setActiveMenu] = useState<api.ActiveMenuData | null>(null)
  const [menuView, setMenuView] = useState<MenuView>('main')
  const [uiSnapshot, setUiSnapshot] = useState<api.UiSnapshot | null>(null)
  const [atSuggestions, setAtSuggestions] = useState<api.MenuOptionItem[] | null>(null)
  const [documentVisible, setDocumentVisible] = useState(document.visibilityState === 'visible')
  const channelSurfingOnly = useRef(false)
  const bottomRef = useRef<HTMLDivElement>(null)
  const transcriptRef = useRef<HTMLDivElement>(null)
  const nextKey = useRef(1)
  const initialized = useRef(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const tickInFlight = useRef(false)
  const autoScrollRef = useRef(true)
  const scrollBehaviorRef = useRef<ScrollBehavior>('auto')
  const refreshInFlightRef = useRef(false)
  const refreshQueuedRef = useRef(false)
  const lastInteractionAtRef = useRef(0)
  const busy = initializing || commandPending || panelBusy
  const busyLabel = commandPending ? 'Sending…' : panelBusy ? 'Updating…' : initializing ? 'Loading…' : null

  function focusInputToEnd() {
    requestAnimationFrame(() => {
      inputRef.current?.focus()
      inputRef.current?.setSelectionRange(inputRef.current.value.length, inputRef.current.value.length)
    })
  }

  function refreshSnapshot() {
    if (!token || !id) return
    if (refreshInFlightRef.current) {
      refreshQueuedRef.current = true
      return
    }
    refreshInFlightRef.current = true
    api.fetchSessionUi(token, id).then(snap => {
      channelSurfingOnly.current = snap.channel_surfing_only
      setUiSnapshot(snap)
      setActiveMenu(snap.active_menu ?? null)
    }).catch(() => {}).finally(() => {
      refreshInFlightRef.current = false
      if (refreshQueuedRef.current) {
        refreshQueuedRef.current = false
        refreshSnapshot()
      }
    })
  }

  function queueScroll(behavior: ScrollBehavior) {
    scrollBehaviorRef.current = behavior
  }

  function appendLines(texts: string[], behavior: ScrollBehavior = 'auto') {
    if (texts.length === 0) return
    if (behavior === 'smooth') {
      autoScrollRef.current = true
    }
    queueScroll(behavior)
    setLines(prev => [
      ...prev,
      ...texts.map(text => ({ text, key: nextKey.current++ })),
    ])
  }

  const handleTranscriptScroll = useCallback((e: UIEvent<HTMLDivElement>) => {
    const el = e.currentTarget
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight
    autoScrollRef.current = distanceFromBottom < 80
  }, [])

  useEffect(() => {
    if (!autoScrollRef.current) return
    bottomRef.current?.scrollIntoView({ behavior: scrollBehaviorRef.current })
    scrollBehaviorRef.current = 'auto'
  }, [lines])

  useEffect(() => {
    const onVisibilityChange = () => setDocumentVisible(document.visibilityState === 'visible')
    document.addEventListener('visibilitychange', onVisibilityChange)
    return () => document.removeEventListener('visibilitychange', onVisibilityChange)
  }, [])

  useEffect(() => {
    if (initialized.current || !token || !id) return
    initialized.current = true
    setInitializing(true)

    const titleEntries: Line[] = []
    if (sessionState?.title) {
      titleEntries.push({ text: `== ${sessionState.title} ==`, key: nextKey.current++ })
    }
    setLines(titleEntries)

    api.fetchSessionUi(token, id)
      .then(snap => {
        channelSurfingOnly.current = snap.channel_surfing_only
        setUiSnapshot(snap)
        setActiveMenu(snap.active_menu ?? null)
      })
      .catch(() => {})

    api.fetchTranscript(token, id)
      .then(transcript => {
        if (transcript.length > 0) {
          setLines([
            ...titleEntries,
            ...transcript.map(t => ({ text: t, key: nextKey.current++ })),
          ])
          setInitializing(false)
          return false
        }
        if (sessionState?.intro_text) {
          setLines([
            ...titleEntries,
            { text: sessionState.intro_text, key: nextKey.current++ },
          ])
        }
        return true
      })
      .catch(() => true)
      .then(shouldLook => {
        if (!shouldLook) return
        api.runCommand(token, id, 'look')
          .then(res => {
            applyCommandResponse(res, 'auto')
          })
          .catch(err => {
            appendLines([`[error: ${err instanceof Error ? err.message : 'failed to load'}]`], 'auto')
          })
          .finally(() => setInitializing(false))
      })
  }, [token, id])

  useEffect(() => {
    if (gameOver) {
      refreshSnapshot()
    }
  }, [gameOver])

  useEffect(() => {
    if (gameOver && uiSnapshot?.session_closure) {
      setSessionClosure(uiSnapshot.session_closure)
    }
  }, [gameOver, uiSnapshot])

  function openMenu() {
    setQuickPanel(null)
    setMenuView('main')
    setShowMenu(true)
    if (token && id) {
      api.fetchSessionUi(token, id).then(snap => {
        channelSurfingOnly.current = snap.channel_surfing_only
        setUiSnapshot(snap)
        setActiveMenu(snap.active_menu ?? null)
      }).catch(() => {})
    }
  }

  function addOutcome(text: string) {
    appendLines([text], 'auto')
  }

  function applyCommandResponse(res: api.CommandResponse, behavior: ScrollBehavior = 'auto') {
    if (res.text) {
      appendLines([res.text], behavior)
    }
    if (res.ui_snapshot) {
      channelSurfingOnly.current = res.ui_snapshot.channel_surfing_only
      setUiSnapshot(res.ui_snapshot)
      setActiveMenu(res.ui_snapshot.active_menu ?? null)
    } else {
      refreshSnapshot()
    }
    if (res.session_closure) {
      setSessionClosure(res.session_closure)
    }
    if (res.movie) {
      setMovie(res.movie)
      setMovieFrame(0)
    }
    if (res.game_over) setGameOver(true)
  }

  async function execCommand(cmd: string, displayCmd?: string) {
    if (!token || !id || commandPending || gameOver) return
    setActiveMenu(null)
    setMovie(null)
    setMovieFrame(0)
    setQuickPanel(null)
    setCommandPending(true)
    lastInteractionAtRef.current = Date.now()
    autoScrollRef.current = true
    const cmdLine: Line = { text: `> ${displayCmd ?? cmd}`, key: nextKey.current++ }
    queueScroll('smooth')
    setLines(prev => [...prev, cmdLine])
    try {
      const res = await api.runCommand(token, id, cmd)
      applyCommandResponse(res, 'smooth')
    } catch (err: unknown) {
      addOutcome(`[error: ${err instanceof Error ? err.message : 'request failed'}]`)
    } finally {
      setCommandPending(false)
    }
  }

  useEffect(() => {
    if (!token || !id || gameOver || !documentVisible) return
    const intervalMs = uiSnapshot?.npc_tick_interval_ms ?? 0
    if (intervalMs <= 0) return
    if (
      busy ||
      movie ||
      activeMenu ||
      showMenu ||
      quickPanel !== null ||
      showStatusModal ||
      input.trim().length > 0
    ) {
      return
    }

    let cancelled = false
    let timer: number | undefined

    const schedule = (delay: number) => {
      timer = window.setTimeout(async () => {
        if (cancelled) return
        if (Date.now() - lastInteractionAtRef.current < intervalMs) {
          schedule(intervalMs)
          return
        }
        if (tickInFlight.current) {
          schedule(intervalMs)
          return
        }
        tickInFlight.current = true
        try {
          const res = await api.runRealtimeTick(token, id)
          if (res.text || res.movie || res.game_over || res.session_closure) {
            applyCommandResponse(res, 'auto')
          }
        } catch (error) {
          console.error('background tick failed', error)
        } finally {
          tickInFlight.current = false
          if (!cancelled) schedule(intervalMs)
        }
      }, delay)
    }

    schedule(intervalMs)

    return () => {
      cancelled = true
      if (timer !== undefined) window.clearTimeout(timer)
    }
  }, [
    token,
    id,
    gameOver,
    documentVisible,
    uiSnapshot?.npc_tick_interval_ms,
    busy,
    movie,
    activeMenu,
    showMenu,
    quickPanel,
    showStatusModal,
    input,
  ])

  function closeMovie() {
    if (movie && movie.narrative_lines.length > 0) {
      appendLines(movie.narrative_lines, 'auto')
    }
    setMovie(null)
    setMovieFrame(0)
    refreshSnapshot()
  }

  async function doSwitchRoom(roomId: string) {
    if (!token || !id) return
    setShowMenu(false)
    setShowStatusModal(false)
    setPanelBusy(true)
    lastInteractionAtRef.current = Date.now()
    try {
      const res = await api.switchRoom(token, id, roomId)
      applyCommandResponse(res, 'smooth')
    } catch (err: unknown) {
      addOutcome(`[error: ${err instanceof Error ? err.message : 'request failed'}]`)
    } finally {
      setPanelBusy(false)
    }
  }

  async function doFollowActor(actorId: string | null) {
    if (!token || !id) return
    setShowMenu(false)
    setShowStatusModal(false)
    setPanelBusy(true)
    lastInteractionAtRef.current = Date.now()
    try {
      const res = await api.followActor(token, id, actorId)
      applyCommandResponse(res, 'smooth')
    } catch (err: unknown) {
      addOutcome(`[error: ${err instanceof Error ? err.message : 'request failed'}]`)
    } finally {
      setPanelBusy(false)
    }
  }

  async function doChangeLocale(locale: string) {
    if (!token || !id) return
    setShowMenu(false)
    setShowStatusModal(false)
    setPanelBusy(true)
    lastInteractionAtRef.current = Date.now()
    try {
      const res = await api.setLocale(token, id, locale)
      applyCommandResponse(res, 'smooth')
    } catch (err: unknown) {
      addOutcome(`[error: ${err instanceof Error ? err.message : 'request failed'}]`)
    } finally {
      setPanelBusy(false)
    }
  }

  function doExit() {
    if (confirm('Exit game?')) {
      navigate('/games')
    }
  }

  async function send(e: FormEvent) {
    e.preventDefault()
    if (!token || !id || commandPending || gameOver) return
    let trimmed = input.trim()
    if (!trimmed) return
    const displayInput = trimmed
    setAtSuggestions(null)
    setInput('')
    if (trimmed.startsWith('@')) {
      trimmed = 'talk to ' + trimmed.slice(1).trimStart()
    }
    if (trimmed === '?') { openMenu(); return }
    if (trimmed.toLowerCase() === 'move' || trimmed.toLowerCase() === 'follow') {
      const snap = uiSnapshot || await api.fetchSessionUi(token, id).catch(() => null)
      if (snap?.channel_surfing_only) {
        setUiSnapshot(snap)
        setQuickPanel(trimmed.toLowerCase() === 'move' ? 'rooms' : 'follow')
        return
      }
    }
    await execCommand(trimmed, displayInput)
  }

  return (
    <ErrorBoundary>
    <div className="min-h-dvh flex flex-col bg-surface">
      <header className="flex items-center justify-between gap-3 px-4 py-3 border-b border-subtle shrink-0">
        <div className="flex items-center gap-2">
          <button onClick={() => navigate('/games')} className="text-sm text-muted hover:text-text cursor-pointer">&larr; Sessions</button>
          <button
            onClick={openMenu}
            disabled={busy}
            className="text-sm px-2 py-1 rounded bg-overlay border border-subtle text-text transition duration-200 hover:brightness-110 active:scale-[0.98] disabled:opacity-50 cursor-pointer"
          >&#9776; Menu</button>
          {uiSnapshot && (
            <button
              onClick={() => {
                setQuickPanel(null)
                setShowStatusModal(true)
              }}
              className="lg:hidden text-sm px-2 py-1 rounded bg-overlay border border-subtle text-text transition duration-200 hover:brightness-110 active:scale-[0.98] cursor-pointer"
            >
              Status
            </button>
          )}
        </div>
        <button onClick={logout} className="text-sm text-muted transition duration-200 hover:text-love active:scale-[0.98] cursor-pointer">Log out</button>
      </header>

      {uiSnapshot && (
        <div className="lg:hidden px-4 py-2 border-b border-subtle bg-base/40">
          <div className="flex items-center gap-2 text-xs text-muted overflow-x-auto">
            <span className="shrink-0 rounded-full bg-overlay px-2 py-1 text-text">{uiSnapshot.current_room_name}</span>
            <span className="shrink-0 rounded-full bg-overlay px-2 py-1 text-text">
              Day {uiSnapshot.day_number}{uiSnapshot.time_label ? ` — ${uiSnapshot.time_label}` : ''}
            </span>
            {uiSnapshot.followed_actor_name && (
              <span className="shrink-0 rounded-full bg-pine/20 px-2 py-1 text-foam">
                Following {uiSnapshot.followed_actor_name}
              </span>
            )}
          </div>
        </div>
      )}

      <div className="flex-1 flex overflow-hidden">
        <div className="flex-1 flex flex-col overflow-hidden">
          <TranscriptPane
            lines={lines}
            busyLabel={busyLabel}
            sessionClosure={sessionClosure}
            gameOver={gameOver}
            transcriptRef={transcriptRef}
            bottomRef={bottomRef}
            onScroll={handleTranscriptScroll}
            onDismissClosure={() => setSessionClosure(null)}
          />

          <div className="relative border-t border-subtle shrink-0">
            <QuickActionPanel
              panel={quickPanel}
              uiSnapshot={uiSnapshot}
              busy={busy}
              onClose={() => setQuickPanel(null)}
              onLook={async command => {
                setQuickPanel(null)
                await execCommand(command)
              }}
              onSwitchRoom={roomId => {
                setQuickPanel(null)
                void doSwitchRoom(roomId)
              }}
              onFollowActor={actorId => {
                setQuickPanel(null)
                void doFollowActor(actorId)
              }}
              onTalk={title => {
                setQuickPanel(null)
                setInput(`@${title} `)
                setAtSuggestions(null)
                focusInputToEnd()
              }}
              onOverflow={action => {
                const talkOpts = uiSnapshot?.talk_options ?? []
                if ((action.id === 'speak' || action.id === 'talk') && talkOpts.length > 0) {
                  if (talkOpts.length === 1) {
                    setQuickPanel(null)
                    setInput(`@${talkOpts[0].title} `)
                    setAtSuggestions(null)
                    focusInputToEnd()
                    return
                  }
                  if (talkOpts.length > 1) {
                    setQuickPanel('talk')
                    return
                  }
                }
                setQuickPanel(null)
                void execCommand(action.id)
              }}
            />
          <div className="flex flex-wrap gap-2 px-4 py-2">
            {(uiSnapshot?.action_bar_actions ?? [
              { id: 'look', label: 'Look' },
              { id: 'move', label: 'Move' },
              { id: 'follow', label: 'Follow' },
            ]).map(action => {
              const handleClick = () => {
                if (busy || gameOver) return
                if (action.id === 'look') {
                  setQuickPanel(current => current === 'look' ? null : 'look')
                  return
                }
                if (action.id === 'move') {
                  setQuickPanel(current => current === 'rooms' ? null : 'rooms')
                  return
                }
                if (action.id === 'follow') {
                  setQuickPanel(current => current === 'follow' ? null : 'follow')
                  return
                }
                const talkOpts = uiSnapshot?.talk_options ?? []
                if ((action.id === 'speak' || action.id === 'talk') && talkOpts.length > 0) {
                  if (talkOpts.length === 1) {
                    setInput(`@${talkOpts[0].title} `)
                    setAtSuggestions(null)
                    focusInputToEnd()
                    return
                  }
                  if (talkOpts.length > 1) {
                    setQuickPanel(current => current === 'talk' ? null : 'talk')
                    return
                  }
                }
                execCommand(action.id)
              }
              return (
                <button
                  key={action.id}
                  onClick={handleClick}
                  disabled={busy || gameOver}
                  className="px-3 py-1.5 rounded bg-overlay border border-subtle text-text text-sm transition duration-200 hover:brightness-110 active:scale-[0.98] disabled:opacity-50 cursor-pointer"
                >{action.label}</button>
              )
            })}
            {uiSnapshot && uiSnapshot.overflow_actions?.length > 0 && (
              <button
                onClick={() => setQuickPanel(current => current === 'overflow' ? null : 'overflow')}
                disabled={busy || gameOver}
                className="px-3 py-1.5 rounded bg-overlay border border-subtle text-text text-sm transition duration-200 hover:brightness-110 active:scale-[0.98] disabled:opacity-50 cursor-pointer"
                title="More actions"
              >...</button>
            )}
          </div>
          </div>

          {!channelSurfingOnly.current && (
            <div className="border-t border-subtle shrink-0 relative">
              {atSuggestions && atSuggestions.length > 0 && (
                <div className="absolute bottom-full left-4 right-4 mb-1 rounded border border-subtle bg-overlay shadow-lg overflow-hidden">
                  {atSuggestions.map(opt => (
                    <button
                      key={opt.id}
                      onMouseDown={e => {
                        e.preventDefault()
                        setInput(`@${opt.title} `)
                        setAtSuggestions(null)
                        focusInputToEnd()
                      }}
                      className="block w-full text-left px-3 py-2 text-sm text-text transition duration-200 hover:bg-base cursor-pointer"
                    >@{opt.title}</button>
                  ))}
                </div>
              )}
              <form onSubmit={send} className="flex gap-2 px-4 py-3">
                <input
                  ref={inputRef}
                  className="flex-1 px-3 py-2 rounded bg-overlay border border-subtle text-text placeholder-faint focus:outline-none focus:border-pine text-sm"
                  placeholder={gameOver ? 'Game over' : 'What do you do?'}
                  value={input}
                  onChange={e => {
                    const val = e.target.value
                    setInput(val)
                    if (val.startsWith('@')) {
                      const query = val.slice(1).toLowerCase()
                      const opts = uiSnapshot?.talk_options ?? []
                      setAtSuggestions(opts.filter(o => o.title.toLowerCase().includes(query)))
                    } else {
                      setAtSuggestions(null)
                    }
                  }}
                  onKeyDown={e => {
                    if (e.key === 'Escape') setAtSuggestions(null)
                  }}
                  disabled={busy || gameOver}
                  autoFocus
                />
                <button
                  type="submit"
                  disabled={busy || gameOver || !input.trim()}
                  className="px-4 py-2 rounded bg-pine text-surface text-sm font-semibold transition duration-200 hover:brightness-110 active:scale-[0.98] disabled:opacity-50 cursor-pointer"
                >Send</button>
              </form>
            </div>
          )}
        </div>

        {uiSnapshot && (
          <aside className="hidden lg:flex w-72 shrink-0 border-l border-subtle p-4 flex-col gap-4 text-sm overflow-y-auto">
            <StatusPanel uiSnapshot={uiSnapshot} />
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

      {activeMenu && (
        <Modal title={uiSnapshot?.ui_text.menu_option_list_title ?? 'Choose'} onClose={() => setActiveMenu(null)}>
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

      {showStatusModal && uiSnapshot && (
        <Modal title="Status" onClose={() => setShowStatusModal(false)}>
          <StatusPanel uiSnapshot={uiSnapshot} />
        </Modal>
      )}
    </div>
    </ErrorBoundary>
  )
}

const TranscriptPane = memo(function TranscriptPane({
  lines,
  busyLabel,
  sessionClosure,
  gameOver,
  transcriptRef,
  bottomRef,
  onScroll,
  onDismissClosure,
}: {
  lines: Line[]
  busyLabel: string | null
  sessionClosure: api.SessionClosureData | null
  gameOver: boolean
  transcriptRef: MutableRefObject<HTMLDivElement | null>
  bottomRef: MutableRefObject<HTMLDivElement | null>
  onScroll: (event: UIEvent<HTMLDivElement>) => void
  onDismissClosure: () => void
}) {
  return (
    <div
      ref={transcriptRef}
      onScroll={onScroll}
      className="flex-1 overflow-y-auto px-4 py-4 space-y-3"
    >
      {lines.map(line => (
        <TranscriptLine key={line.key} line={line} />
      ))}
      {busyLabel && <p className="text-muted text-sm italic">{busyLabel}</p>}
      {sessionClosure && (
        <SessionClosureModal sessionClosure={sessionClosure} onDismiss={onDismissClosure} />
      )}
      {gameOver && !sessionClosure && (
        <p className="text-love font-semibold text-center pt-4">Game Over</p>
      )}
      <div ref={bottomRef} />
    </div>
  )
})

const QuickActionPanel = memo(function QuickActionPanel({
  panel,
  uiSnapshot,
  busy,
  onClose,
  onLook,
  onSwitchRoom,
  onFollowActor,
  onTalk,
  onOverflow,
}: {
  panel: QuickPanel
  uiSnapshot: api.UiSnapshot | null
  busy: boolean
  onClose: () => void
  onLook: (command: string) => Promise<void>
  onSwitchRoom: (roomId: string) => void
  onFollowActor: (actorId: string | null) => void
  onTalk: (title: string) => void
  onOverflow: (action: api.OverflowAction) => void
}) {
  if (!panel || !uiSnapshot) return null

  return (
    <div className="absolute bottom-full inset-x-0 z-20 px-4 pb-2">
      <div className="rounded-2xl border border-subtle bg-surface/98 shadow-2xl backdrop-blur-sm">
        <div className="flex items-center justify-between px-4 py-3 border-b border-subtle">
          <div>
            <h3 className="text-sm font-semibold text-text">
              {panel === 'look'
                ? uiSnapshot.ui_text.look_modal_title
                : panel === 'talk'
                  ? uiSnapshot.ui_text.talk_modal_title
                  : panel === 'rooms'
                    ? uiSnapshot.ui_text.room_switcher_title
                    : panel === 'follow'
                      ? uiSnapshot.ui_text.follow_actor_title
                  : uiSnapshot.ui_text.commands_modal_title}
            </h3>
            {panel === 'talk' && (
              <p className="text-xs text-muted mt-0.5">{uiSnapshot.ui_text.talk_modal_prompt}</p>
            )}
          </div>
          <button
            onClick={onClose}
            className="text-muted hover:text-text text-lg leading-none transition duration-200 active:scale-95 cursor-pointer"
          >
            &times;
          </button>
        </div>

        <div className="max-h-[40dvh] overflow-y-auto p-3 space-y-3">
          {panel === 'look' && (
            (uiSnapshot.look_options ?? []).length === 0 ? (
              <p className="text-muted italic text-sm px-1">Nothing of particular interest here.</p>
            ) : (
              groupLookOptions(uiSnapshot.look_options, uiSnapshot.ui_text).map(([group, options]) => (
                <div key={group} className="space-y-2">
                  <p className="text-[11px] text-muted uppercase tracking-wider px-1">{group}</p>
                  <div className="grid gap-2 sm:grid-cols-2">
                    {options.map(opt => (
                      <button
                        key={opt.id}
                        onClick={() => { void onLook(opt.command) }}
                        disabled={busy}
                        className="block w-full text-left px-3 py-2 rounded-xl hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
                      >
                        {opt.title}
                      </button>
                    ))}
                  </div>
                </div>
              ))
            )
          )}

          {panel === 'talk' && (
            <div className="grid gap-2 sm:grid-cols-2">
              {uiSnapshot.talk_options.map(opt => (
                <button
                  key={opt.id}
                  onClick={() => onTalk(opt.title)}
                  disabled={busy}
                  className="block w-full text-left px-3 py-2 rounded-xl hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
                >
                  {opt.title}
                </button>
              ))}
            </div>
          )}

          {panel === 'rooms' && (
            <div className="grid gap-2 sm:grid-cols-2">
              {uiSnapshot.rooms.map(room => (
                <button
                  key={room.id}
                  onClick={() => onSwitchRoom(room.id)}
                  disabled={busy}
                  className="block w-full text-left px-3 py-2 rounded-xl hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
                >
                  <span className="font-medium">{room.title}</span>
                  {room.menu_text && <span className="text-muted text-xs ml-2">{room.menu_text}</span>}
                </button>
              ))}
            </div>
          )}

          {panel === 'follow' && (
            <div className="grid gap-2 sm:grid-cols-2">
              {uiSnapshot.follow_options.map(actor => (
                <button
                  key={actor.id}
                  onClick={() => onFollowActor(actor.id === 'none' ? null : actor.id)}
                  disabled={busy}
                  className="block w-full text-left px-3 py-2 rounded-xl hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
                >
                  <span className="font-medium">{actor.title}</span>
                  {actor.menu_text && <span className="text-muted text-xs ml-2">{actor.menu_text}</span>}
                </button>
              ))}
            </div>
          )}

          {panel === 'overflow' && (
            (uiSnapshot.overflow_actions ?? []).length === 0 ? (
              <p className="text-muted italic text-sm px-1">{uiSnapshot.ui_text.commands_modal_empty}</p>
            ) : (
              groupOverflowActions(uiSnapshot.overflow_actions ?? [], uiSnapshot.ui_text).map(([group, items]) => (
                <div key={group} className="space-y-2">
                  <p className="text-[11px] font-semibold text-muted uppercase tracking-wider px-1">{group}</p>
                  {items.map(action => (
                    <button
                      key={action.id}
                      onClick={() => onOverflow(action)}
                      disabled={busy}
                      className="block w-full text-left px-3 py-2 rounded-xl hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
                      title={action.usage}
                    >
                      <span className="font-medium">{action.label}</span>
                      {action.usage && <span className="text-muted text-xs ml-2">— {action.usage}</span>}
                    </button>
                  ))}
                </div>
              ))
            )
          )}
        </div>
      </div>
    </div>
  )
})

const TranscriptLine = memo(function TranscriptLine({ line }: { line: Line }) {
  return (
    <div className="whitespace-pre-wrap text-sm leading-relaxed">
      {line.text.startsWith('> ') ? (
        <span className="text-foam">{line.text}</span>
      ) : line.text.startsWith('== ') ? (
        <span className="text-iris font-bold">{line.text}</span>
      ) : (
        <span className="text-text">{line.text}</span>
      )}
    </div>
  )
})

const SessionClosureModal = memo(function SessionClosureModal({
  sessionClosure,
  onDismiss,
}: {
  sessionClosure: api.SessionClosureData
  onDismiss: () => void
}) {
  return (
    <div className="fixed inset-0 bg-base/80 flex items-center justify-center z-50">
      <div className="bg-surface rounded-xl p-6 max-w-md w-full mx-4 shadow-2xl">
        <h2 className="text-xl font-bold text-center mb-2">{sessionClosure.title}</h2>
        {sessionClosure.subtitle && (
          <p className="text-center text-sm text-muted mb-4">— {sessionClosure.subtitle}</p>
        )}
        <div className="space-y-4">
          {sessionClosure.sections.map((section, index) => (
            <div key={`${section.kind}-${index}`}>
              {section.title && (
                <p className="text-xs text-muted uppercase tracking-wider mb-2">{section.title}</p>
              )}
              {section.kind === 'rating' ? (
                <div className="flex justify-center gap-1 text-2xl">
                  {Array.from({ length: section.max }, (_, i) => i + 1).map(n => (
                    <span key={n} className={n <= section.value ? 'text-yellow-400' : 'text-muted'}>
                      {n <= section.value ? '\u2605' : '\u2606'}
                    </span>
                  ))}
                </div>
              ) : (
                <p className="text-center text-balance leading-relaxed whitespace-pre-wrap">{section.body}</p>
              )}
            </div>
          ))}
        </div>
        <button
          className="mt-6 w-full py-2 bg-love text-white rounded-lg font-semibold hover:opacity-90"
          onClick={onDismiss}
        >
          OK
        </button>
      </div>
    </div>
  )
})

function StatusPanel({ uiSnapshot }: { uiSnapshot: api.UiSnapshot }) {
  return (
    <div className="space-y-4">
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
              <li key={i} className="text-text text-xs">
                • {item.label}{item.count > 1 ? <span className="text-muted ml-1">×{item.count}</span> : null}
              </li>
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
    </div>
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

function groupOverflowActions(
  actions: api.OverflowAction[],
  uiText: api.UiSnapshot['ui_text'],
): [string, api.OverflowAction[]][] {
  const map = new Map<string, api.OverflowAction[]>()
  for (const a of actions) {
    const g = localizeCommandGroup(a.group, uiText)
    if (!map.has(g)) map.set(g, [])
    map.get(g)!.push(a)
  }
  return Array.from(map.entries()).sort(([a], [b]) => a.localeCompare(b))
}

function localizeCommandGroup(group: string, uiText: api.UiSnapshot['ui_text']): string {
  switch ((group || '').toLowerCase()) {
    case 'support':
      return uiText.commands_group_support
    case 'other':
    case '':
      return uiText.commands_group_other
    default:
      return group
  }
}

function groupLookOptions(
  options: api.LookOptionData[],
  uiText: api.UiSnapshot['ui_text'],
): [string, api.LookOptionData[]][] {
  const grouped: [string, api.LookOptionData[]][] = []

  const room = options.filter(option => option.id === '__room__')
  if (room.length > 0) grouped.push([uiText.look_group_room, room])

  const things = options.filter(option => option.id.startsWith('feature:') || option.id.startsWith('item:'))
  if (things.length > 0) grouped.push([uiText.look_group_things, things])

  const people = options.filter(option => option.id.startsWith('actor:'))
  if (people.length > 0) grouped.push([uiText.look_group_people, people])

  const seen = new Set(options.flatMap(option => {
    if (option.id === '__room__') return [option.id]
    if (option.id.startsWith('feature:') || option.id.startsWith('item:') || option.id.startsWith('actor:')) {
      return [option.id]
    }
    return []
  }))
  const other = options.filter(option => !seen.has(option.id))
  if (other.length > 0) grouped.push([uiText.commands_group_other, other])

  return grouped
}
