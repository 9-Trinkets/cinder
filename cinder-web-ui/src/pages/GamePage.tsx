import { Component, useCallback, useEffect, useState, useRef, type FormEvent } from 'react'
import { useParams, useNavigate, useLocation } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import ShellMenu from '../components/ShellMenu'
import Modal from '../components/Modal'
import TranscriptPane from '../components/TranscriptPane'
import StatusPanel from '../components/StatusPanel'
import RelationshipChart from '../components/RelationshipChart'
import MovieModal from '../components/MovieModal'
import QuickActionPanel, { type QuickPanel } from '../components/QuickActionPanel'
import ConfirmDialog from '../components/ConfirmDialog'
import { useToast } from '../components/Toast'
import type { Line } from '../components/TranscriptLine'
import { themeVars } from '../utils/theme'

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

type MenuView = 'main' | 'rooms' | 'follow' | 'language'

export default function GamePage() {
  const { id } = useParams<{ id: string }>()
  const { token, logout } = useAuth()
  const { showToast } = useToast()
  const navigate = useNavigate()
  const location = useLocation()
  const sessionState = location.state as { title?: string; intro_text?: string } | null
  const [lines, setLines] = useState<Line[]>([])
  const [input, setInput] = useState('')
  const [gameOver, setGameOver] = useState(false)
  const [initializing, setInitializing] = useState(false)
  const [commandPending, setCommandPending] = useState(false)
  const [panelBusy, setPanelBusy] = useState(false)
  const [actClosure, setSessionClosure] = useState<api.ActClosureData | null>(null)
  const [gameClosure, setGameClosure] = useState<api.ActClosureData | null>(null)
  const [showMenu, setShowMenu] = useState(false)
  const [quickPanel, setQuickPanel] = useState<QuickPanel>(null)
  const [showStatusModal, setShowStatusModal] = useState(false)
  const [movie, setMovie] = useState<api.MovieData | null>(null)
  const [movieFrame, setMovieFrame] = useState(0)
  const [activeMenu, setActiveMenu] = useState<api.ActiveMenuData | null>(null)
  const [menuSelections, setMenuSelections] = useState<Set<string>>(new Set())
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
  const autoScrollRef = useRef(true)
  const scrollBehaviorRef = useRef<ScrollBehavior>('auto')
  const refreshInFlightRef = useRef(false)
  const refreshQueuedRef = useRef(false)
  const lastInteractionAtRef = useRef(0)
  const commandHistoryRef = useRef<string[]>([])
  const historyIndexRef = useRef<number | null>(null)
  const draftInputRef = useRef('')
  const busy = initializing || commandPending || panelBusy
  const busyLabel = commandPending ? 'Sending…' : panelBusy ? 'Updating…' : initializing ? 'Loading…' : null
  const activeMenuTitle = activeMenu?.prompt?.trim() || uiSnapshot?.ui_text.menu_option_list_title || 'Choose'

  function findPanelConfig(panelName: string): api.PanelConfigData | undefined {
    return (
      uiSnapshot?.action_bar_actions.find(a => a.panel === panelName)?.panel_config ??
      uiSnapshot?.overflow_actions.find(a => a.panel === panelName)?.panel_config
    )
  }

  function handleSelectPanelOption(panelName: string, option: api.PanelOptionData) {
    const config = findPanelConfig(panelName)
    setQuickPanel(null)
    if (!config) return
    switch (config.on_select) {
      case 'execute_command':
        if (option.command) void execCommand(option.command)
        break
      case 'prefill_input':
        setInput(`@${option.title} `)
        setAtSuggestions(null)
        focusInputToEnd()
        break
      case 'switch_room':
        void doSwitchRoom(option.id)
        break
      case 'follow_actor':
        void doFollowActor(option.id === 'none' ? null : option.id)
        break
    }
  }

  useEffect(() => {
    if (activeMenu?.max_selections && activeMenu.max_selections > 0) {
      setMenuSelections(new Set(activeMenu.selected_ids ?? []))
    } else {
      setMenuSelections(new Set())
    }
  }, [activeMenu])

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

  function appendLines(
    items: Array<{ text: string; kind?: api.LineKind }>,
    behavior: ScrollBehavior = 'auto',
  ) {
    if (items.length === 0) return
    if (behavior === 'smooth') {
      autoScrollRef.current = true
    }
    queueScroll(behavior)
    setLines(prev => [
      ...prev,
      ...items.map(item => ({ text: item.text, kind: item.kind, key: nextKey.current++ })),
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
      titleEntries.push({ text: `== ${sessionState.title} ==`, kind: 'heading', key: nextKey.current++ })
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
            ...transcript.map(t => ({ text: t.text, kind: t.kind, key: nextKey.current++ })),
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
            showToast(err instanceof Error ? err.message : 'failed to load', 'error')
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
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape' && !showMenu) {
        e.preventDefault()
        openMenu()
      }
    }
    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [showMenu])

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

  function applyCommandResponse(res: api.CommandResponse, behavior: ScrollBehavior = 'auto') {
    // Prefer the engine's typed narrative lines; fall back to splitting the
    // raw text into narration blocks for older responses without line kinds.
    const typed = (res.lines ?? []).filter(l => l.text.trim())
    if (typed.length > 0) {
      appendLines(typed.map(l => ({ text: l.text, kind: l.kind })), behavior)
    } else if (res.text) {
      const chunks = res.text
        .split(/\n\n+/)
        .map(chunk => chunk.trim())
        .filter(Boolean)
      appendLines(chunks.length ? chunks.map(text => ({ text })) : [{ text: res.text }], behavior)
    }
    if (res.ui_snapshot) {
      channelSurfingOnly.current = res.ui_snapshot.channel_surfing_only
      setUiSnapshot(res.ui_snapshot)
      setActiveMenu(res.ui_snapshot.active_menu ?? null)
    } else {
      refreshSnapshot()
    }
    if (res.act_closure) {
      setSessionClosure(prev => prev ?? res.act_closure)
    }
    if (res.game_closure) {
      setGameClosure(prev => prev ?? res.game_closure)
    }
    if (res.movie) {
      setMovie(res.movie)
      setMovieFrame(0)
    }
    setGameOver(res.game_over)
  }

  async function execCommand(cmd: string, displayCmd?: string) {
    if (!token || !id || commandPending || gameOver) return
    setActiveMenu(null)
    setMenuSelections(new Set())
    setMovie(null)
    setMovieFrame(0)
    setQuickPanel(null)
    setCommandPending(true)
    lastInteractionAtRef.current = Date.now()
    autoScrollRef.current = true
    const cmdLine: Line = { text: `> ${displayCmd ?? cmd}`, kind: 'player', key: nextKey.current++ }
    queueScroll('smooth')
    setLines(prev => [...prev, cmdLine])
    try {
      const res = await api.runCommand(token, id, cmd)
      applyCommandResponse(res, 'smooth')
    } catch (err: unknown) {
      showToast(err instanceof Error ? err.message : 'request failed', 'error')
    } finally {
      setCommandPending(false)
    }
  }

  async function toggleMenuOption(optionId: string) {
    if (!token || !id || commandPending || gameOver) return
    setMenuSelections(prev => {
      const next = new Set(prev)
      if (next.has(optionId)) {
        next.delete(optionId)
      } else {
        next.add(optionId)
      }
      return next
    })
    setCommandPending(true)
    try {
      const res = await api.runCommand(token, id, `toggle:${optionId}`)
      applyCommandResponse(res, 'smooth')
    } catch (err: unknown) {
      showToast(err instanceof Error ? err.message : 'request failed', 'error')
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
      showStatusModal
    ) {
      return
    }

    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const wsUrl = `${proto}//${window.location.host}/api/games/${id}/ws?token=${token}&tick_ms=${intervalMs}`
    const ws = new WebSocket(wsUrl)

    ws.onmessage = (event) => {
      if (input.trim().length > 0) return
      try {
        const res: api.CommandResponse = JSON.parse(event.data)
        if (res.text || res.movie || res.game_over || res.act_closure || res.game_closure) {
          applyCommandResponse(res, 'auto')
        }
      } catch {
        console.error('ws: failed to parse tick message')
      }
    }

    ws.onerror = () => {
      ws.close()
    }

    return () => {
      ws.close()
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
  ])

  function closeMovie() {
    if (movie && movie.narrative_lines.length > 0) {
      appendLines(movie.narrative_lines.map(text => ({ text })), 'auto')
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
      showToast(err instanceof Error ? err.message : 'request failed', 'error')
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
      showToast(err instanceof Error ? err.message : 'request failed', 'error')
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
      showToast(err instanceof Error ? err.message : 'request failed', 'error')
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
    if (!trimmed) {
      if (activeMenu && (activeMenu.max_selections ?? 0) > 0 && menuSelections.size >= (activeMenu.min_selections || 1)) {
        setInput('')
        await execCommand('done', 'Done')
      }
      return
    }
    const displayInput = trimmed
    setAtSuggestions(null)
    setInput('')
    if (commandHistoryRef.current[commandHistoryRef.current.length - 1] !== displayInput) {
      commandHistoryRef.current.push(displayInput)
    }
    historyIndexRef.current = null
    draftInputRef.current = ''
    if (trimmed.startsWith('@')) {
      trimmed = 'talk to ' + trimmed.slice(1).trimStart()
    }
    if (trimmed === '?') { openMenu(); return }
    const lowerInput = trimmed.toLowerCase()
    // Bare "look" opens the look panel; the panel's Room option still prints
    // the full description.
    if (lowerInput === 'look' || lowerInput === 'l') {
      setQuickPanel(current => current === 'look' ? null : 'look')
      return
    }
    const matchingBarAction = (uiSnapshot?.action_bar_actions ?? []).find(
      a => a.id.toLowerCase() === lowerInput || a.label.toLowerCase() === lowerInput
    )
    if (matchingBarAction?.panel) {
      const snap = uiSnapshot || await api.fetchSessionUi(token, id).catch(() => null)
      if (snap?.channel_surfing_only) {
        setUiSnapshot(snap)
        setQuickPanel(current => current === matchingBarAction.panel ? null : matchingBarAction.panel ?? null)
        return
      }
    }
    await execCommand(trimmed, displayInput)
  }

  return (
    <ErrorBoundary>
    <div className="h-dvh flex flex-col bg-surface overflow-hidden">
      <div style={uiSnapshot?.theme ? themeVars(uiSnapshot.theme) : undefined} className="contents">
      <header className="sticky top-0 z-10 bg-surface flex items-center justify-between gap-3 px-4 py-3 border-b border-subtle shrink-0">
        <div className="flex items-center gap-2">
          <button onClick={() => navigate(`/games/pack/${uiSnapshot?.pack_id}`)} className="text-sm text-muted hover:text-text cursor-pointer">&larr; Back</button>
          <button
            onClick={openMenu}
            disabled={busy}
            className="text-sm px-2 py-1 rounded bg-overlay border border-subtle text-text transition duration-200 hover:brightness-110 active:scale-[0.98] disabled:opacity-50 cursor-pointer"
          >&#9776; Menu</button>
        </div>
        <button onClick={logout} className="text-sm text-muted transition duration-200 hover:text-love active:scale-[0.98] cursor-pointer">Log out</button>
      </header>

      {uiSnapshot && (
        <button
          onClick={() => {
            setQuickPanel(null)
            setShowStatusModal(true)
          }}
          className="lg:hidden w-full text-left px-4 py-2 border-b border-subtle bg-base/40 cursor-pointer"
        >
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
        </button>
      )}

      <div className="flex-1 flex overflow-hidden">
        <div className="flex-1 flex flex-col overflow-hidden">
          <TranscriptPane
            lines={lines}
            busyLabel={busyLabel}
            actClosure={actClosure}
            gameClosure={gameClosure}
            gameOver={gameOver}
            craftedLabels={uiSnapshot?.crafted_consumable_labels ?? []}
            interactableLabels={uiSnapshot?.interactable_labels ?? []}
            transcriptRef={transcriptRef}
            bottomRef={bottomRef}
            onScroll={handleTranscriptScroll}
            onDismissClosure={() => {
              setSessionClosure(null)
              if (token && id) {
                api.continueSession(token, id).then(res => {
                  applyCommandResponse(res)
                }).catch(() => {})
              }
            }}
            onDismissGameClosure={() => setGameClosure(null)}
          />

          <div className="relative border-t border-subtle shrink-0">
            <QuickActionPanel
              panel={quickPanel}
              panelConfig={quickPanel ? findPanelConfig(quickPanel) : undefined}
              uiSnapshot={uiSnapshot}
              busy={busy}
              onClose={() => setQuickPanel(null)}
              onLook={async command => {
                setQuickPanel(null)
                await execCommand(command)
              }}
              onSelectOption={handleSelectPanelOption}
              onOverflow={action => {
                const panel = (action as unknown as Record<string, unknown>).panel as string | undefined
                if (panel) {
                  const options = uiSnapshot?.panel_options?.[panel] ?? []
                  if (options.length === 1) {
                    handleSelectPanelOption(panel, options[0])
                    return
                  }
                  if (options.length > 1) {
                    setQuickPanel(panel)
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
                // The Look button opens the look panel instead of dumping text.
                if (action.id === 'look') {
                  setQuickPanel(current => current === 'look' ? null : 'look')
                  return
                }
                const panel = action.panel as string | undefined
                if (panel) {
                  const options = uiSnapshot?.panel_options?.[panel] ?? []
                  if (options.length === 1) {
                    handleSelectPanelOption(panel, options[0])
                    return
                  }
                  if (options.length > 1) {
                    setQuickPanel(current => current === panel ? null : panel)
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
                    const history = commandHistoryRef.current
                    if (e.key === 'ArrowUp' && !atSuggestions?.length) {
                      if (history.length === 0) return
                      e.preventDefault()
                      if (historyIndexRef.current === null) {
                        draftInputRef.current = input
                        historyIndexRef.current = history.length - 1
                      } else if (historyIndexRef.current > 0) {
                        historyIndexRef.current -= 1
                      }
                      setInput(history[historyIndexRef.current])
                    } else if (e.key === 'ArrowDown' && !atSuggestions?.length) {
                      if (historyIndexRef.current === null) return
                      e.preventDefault()
                      if (historyIndexRef.current < history.length - 1) {
                        historyIndexRef.current += 1
                        setInput(history[historyIndexRef.current])
                      } else {
                        historyIndexRef.current = null
                        setInput(draftInputRef.current)
                      }
                    }
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
          <aside className="hidden lg:flex w-72 shrink-0 border-l border-subtle p-4 flex-col text-sm overflow-y-auto self-stretch">
            <StatusPanel uiSnapshot={uiSnapshot} />
            {uiSnapshot.show_relationship_sidebar && uiSnapshot.relationship_pairs.length > 0 && (
              <RelationshipChart pairs={uiSnapshot.relationship_pairs} />
            )}
          </aside>
        )}
      </div>
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
        <Modal title={activeMenuTitle} onClose={() => setActiveMenu(null)}>
          {activeMenu.max_selections && activeMenu.max_selections > 0 && (
            <p className="text-xs text-muted mb-2">
              Select up to {activeMenu.max_selections} option{activeMenu.max_selections === 1 ? '' : 's'}
              {activeMenu.min_selections && activeMenu.min_selections > 0
                ? ` (at least ${activeMenu.min_selections})`
                : ''}
              {menuSelections.size > 0 ? ` — ${menuSelections.size} selected` : ''}
            </p>
          )}
          {activeMenu.options.length === 0 ? (
            <p className="text-muted italic">No options available.</p>
          ) : (
            activeMenu.options.map((opt, i) => {
              const isMultiSelect = (activeMenu.max_selections ?? 0) > 0
              const isSelected = menuSelections.has(opt.id)
              return (
                <button
                  key={opt.id}
                  onClick={async () => {
                    if (isMultiSelect) {
                      await toggleMenuOption(opt.id)
                    } else {
                      await execCommand((i + 1).toString())
                    }
                  }}
                  disabled={busy}
                  className={`block w-full text-left px-3 py-2 rounded border disabled:opacity-50 cursor-pointer transition duration-150 ${
                    isSelected
                      ? 'bg-pine/15 border-pine text-text'
                      : 'hover:bg-overlay border-subtle text-text'
                  }`}
                >
                  {isMultiSelect && (
                    <span className={`inline-block w-4 h-4 mr-2 rounded border align-middle ${
                      isSelected
                        ? 'bg-pine border-pine'
                        : 'border-subtle'
                    }`}>
                      {isSelected && (
                        <span className="block text-surface text-xs text-center leading-4">✓</span>
                      )}
                    </span>
                  )}
                  {!isMultiSelect && (
                    <span className="text-muted mr-2">{(i + 1).toString()}.</span>
                  )}
                  <span className="font-medium">{opt.title}</span>
                  {opt.menu_text && <span className="text-muted ml-2">— {opt.menu_text}</span>}
                </button>
              )
            })
          )}
          {(activeMenu.max_selections ?? 0) > 0 && (
            <button
              onClick={async () => {
                await execCommand('done', 'Done')
              }}
              disabled={busy || menuSelections.size === 0}
              className="mt-3 w-full px-3 py-2 rounded bg-pine text-surface text-sm font-semibold transition duration-200 hover:brightness-110 active:scale-[0.98] disabled:opacity-50 cursor-pointer"
            >
              Done
            </button>
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
          title="Exit game?"
          message="Return to session list?"
          onConfirm={() => navigate('/games')}
          onCancel={() => setShowExitConfirm(false)}
        />
      )}
    </div>
    </ErrorBoundary>
  )
}
