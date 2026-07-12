import { Component, useCallback, useEffect, useState, useRef, type FormEvent } from 'react'
import { useParams, useNavigate, useLocation } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import ShellMenu from '../components/ShellMenu'
import Modal from '../components/Modal'
import TranscriptPane from '../components/TranscriptPane'
import StatusPanel from '../components/StatusPanel'
import MovieModal from '../components/MovieModal'
import SessionClosureModal from '../components/SessionClosureModal'
import QuickActionPanel, { type QuickPanel } from '../components/QuickActionPanel'
import ConfirmDialog from '../components/ConfirmDialog'
import type { Line } from '../components/TranscriptLine'

class ErrorBoundary extends Component<{ children: React.ReactNode }, { error: Error | null }> {
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
  const [showExitConfirm, setShowExitConfirm] = useState(false)
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

  const handleTranscriptScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
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
    setShowExitConfirm(true)
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
    <div className="h-dvh flex flex-col bg-surface overflow-hidden">
      <header className="sticky top-0 z-10 bg-surface flex items-center justify-between gap-3 px-4 py-3 border-b border-subtle shrink-0">
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
                aria-label="More actions"
                className="px-3 py-1.5 rounded bg-overlay border border-subtle text-text text-sm transition duration-200 hover:brightness-110 active:scale-[0.98] disabled:opacity-50 cursor-pointer"
              >...</button>
            )}
          </div>
          </div>

          {!channelSurfingOnly.current && (
            <div className="border-t border-subtle shrink-0 relative">
              {atSuggestions && atSuggestions.length > 0 && (
                <div role="listbox" aria-label="Talk to" className="absolute bottom-full left-4 right-4 mb-1 rounded border border-subtle bg-overlay shadow-lg overflow-hidden">
                  {atSuggestions.map(opt => (
                    <button
                      key={opt.id}
                      role="option"
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
          <aside className="hidden lg:flex w-72 shrink-0 border-l border-subtle p-4 flex-col gap-4 text-sm overflow-y-auto self-start max-h-full">
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

      {showExitConfirm && (
        <ConfirmDialog
          message="Exit game?"
          onConfirm={() => navigate('/games')}
          onCancel={() => setShowExitConfirm(false)}
        />
      )}
    </div>
    </ErrorBoundary>
  )
}
