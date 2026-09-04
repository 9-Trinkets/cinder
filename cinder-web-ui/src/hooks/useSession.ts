import { useCallback, useEffect, useState, useRef, type FormEvent } from 'react'
import { useParams, useLocation } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import { useToast } from '../components/Toast'
import { useNpcTicks } from './useNpcTicks'
import type { Line } from '../components/TranscriptLine'
import type { QuickPanel } from '../components/QuickActionPanel'

export type MenuView = 'main' | 'rooms' | 'follow' | 'language'

export function useSession() {
  const { id } = useParams<{ id: string }>()
  const { token, logout } = useAuth()
  const { showToast } = useToast()
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

  function applyCommandResponse(res: api.CommandResponse, behavior: ScrollBehavior = 'auto') {
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

  function closeMovie() {
    if (movie && movie.narrative_lines.length > 0) {
      appendLines(movie.narrative_lines.map(text => ({ text })), 'auto')
    }
    setMovie(null)
    setMovieFrame(0)
    refreshSnapshot()
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

  const handleTranscriptScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    const el = e.currentTarget
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight
    autoScrollRef.current = distanceFromBottom < 80
  }, [])

  useEffect(() => {
    if (activeMenu?.max_selections && activeMenu.max_selections > 0) {
      setMenuSelections(new Set(activeMenu.selected_ids ?? []))
    } else {
      setMenuSelections(new Set())
    }
  }, [activeMenu])

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

  useNpcTicks({
    token,
    id,
    gameOver,
    documentVisible,
    intervalMs: uiSnapshot?.npc_tick_interval_ms ?? 0,
    blocked: busy || movie !== null || activeMenu !== null || showMenu || quickPanel !== null || showStatusModal,
    inputValue: input,
    onTick: applyCommandResponse,
  })

  return {
    id,
    token,
    logout,
    lines,
    input,
    setInput,
    gameOver,
    busy,
    busyLabel,
    initializing,
    actClosure,
    setSessionClosure,
    gameClosure,
    setGameClosure,
    uiSnapshot,
    activeMenu,
    activeMenuTitle,
    setActiveMenu,
    menuSelections,
    menuView,
    setMenuView,
    quickPanel,
    setQuickPanel,
    showMenu,
    setShowMenu,
    showStatusModal,
    setShowStatusModal,
    showExitConfirm,
    setShowExitConfirm,
    movie,
    movieFrame,
    setMovieFrame,
    atSuggestions,
    setAtSuggestions,
    channelSurfingOnly,
    bottomRef,
    transcriptRef,
    inputRef,
    commandHistoryRef,
    historyIndexRef,
    draftInputRef,
    openMenu,
    closeMovie,
    execCommand,
    toggleMenuOption,
    applyCommandResponse,
    appendLines,
    refreshSnapshot,
    doSwitchRoom,
    doFollowActor,
    doChangeLocale,
    doExit,
    send,
    findPanelConfig,
    handleSelectPanelOption,
    handleTranscriptScroll,
    focusInputToEnd,
  }
}
