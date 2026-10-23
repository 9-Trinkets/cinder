import { useCallback, useEffect, useState, useRef, type FormEvent } from 'react'
import { useParams, useLocation } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import { useToast } from '../components/Toast'
import { useI18n } from '../i18n'
import { useNpcTicks } from './useNpcTicks'
import { toErrorMessage } from '../utils/error'
import type { Line } from '../components/TranscriptLine'
import {
  type MenuView,
  type QuickPanel,
  findPanelConfig,
  extractResponseLines,
} from './playUtils'

export type { MenuView }

export function usePlay() {
  const { id } = useParams<{ id: string }>()
  const { token, logout } = useAuth()
  const { showToast } = useToast()
  const { t } = useI18n()
  const location = useLocation()
  const playState = location.state as { title?: string; intro_text?: string } | null

  const [lines, setLines] = useState<Line[]>([])
  const [input, setInput] = useState('')
  const [gameOver, setGameOver] = useState(false)
  const [initializing, setInitializing] = useState(false)
  const [commandPending, setCommandPending] = useState(false)
  const [panelBusy, setPanelBusy] = useState(false)
  const [tickGenerating, setTickGenerating] = useState(false)
  const [tickSpeaker, setTickSpeaker] = useState<string | null>(null)
  const [actClosure, setActClosure] = useState<api.ActClosureData | null>(null)
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
  const bottomRef = useRef<HTMLDivElement>(null)
  const transcriptRef = useRef<HTMLDivElement>(null)
  const nextKey = useRef(1)
  const initialized = useRef(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const autoScrollRef = useRef(true)
  const scrollBehaviorRef = useRef<ScrollBehavior>('auto')
  const lastProgrammaticScrollRef = useRef(0)
  const refreshInFlightRef = useRef(false)
  const refreshQueuedRef = useRef(false)
  const lastInteractionAtRef = useRef(0)
  const commandHistoryRef = useRef<string[]>([])
  const historyIndexRef = useRef<number | null>(null)
  const draftInputRef = useRef('')

  const busy = initializing || commandPending || panelBusy
  const tickLabel = tickSpeaker
    ? t('hud.isThinking', { speaker: tickSpeaker })
    : t('hud.thinking')
  const busyLabel = commandPending
    ? t('hud.sending')
    : panelBusy
      ? t('hud.updating')
      : initializing
        ? t('hud.loading')
        : tickGenerating
          ? tickLabel
          : null
  const activeMenuTitle = activeMenu?.prompt?.trim() || uiSnapshot?.ui_text.menu_option_list_title || t('hud.choose')

  const getPanelConfig = (panelName: string) => findPanelConfig(uiSnapshot, panelName, t)

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
    api.fetchPlayUi(token, id).then(snap => {
      setUiSnapshot(snap)
      setActiveMenu(snap.active_menu ?? null)
      setGameOver(snap.game_over ?? snap.game_closure !== null)
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
    items: Array<{ text: string; kind: api.LineKind }>,
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
    appendLines(extractResponseLines(res), behavior)
    if (res.ui_snapshot) {
      setUiSnapshot(res.ui_snapshot)
      setActiveMenu(res.ui_snapshot.active_menu ?? null)
    } else {
      refreshSnapshot()
    }
    if (res.act_closure) {
      setActClosure(prev => prev ?? res.act_closure)
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
    // An empty display means the content already shows what the player said.
    const cmdLine: Line | null = displayCmd === ''
      ? null
      : { text: `> ${displayCmd ?? cmd}`, kind: 'player', key: nextKey.current++ }
    queueScroll('smooth')
    if (cmdLine) setLines(prev => [...prev, cmdLine])
    try {
      const res = await api.runCommand(token, id, cmd, displayCmd)
      applyCommandResponse(res, 'smooth')
    } catch (err: unknown) {
      showToast(toErrorMessage(err, t('hud.requestFailed')), 'error')
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
      showToast(toErrorMessage(err, t('hud.requestFailed')), 'error')
    } finally {
      setCommandPending(false)
    }
  }

  function openMenu() {
    setQuickPanel(null)
    setMenuView('main')
    setShowMenu(true)
    if (token && id) {
      api.fetchPlayUi(token, id).then(snap => {
        setUiSnapshot(snap)
        setActiveMenu(snap.active_menu ?? null)
      }).catch(() => {})
    }
  }

  async function runPanelCommand(
    command: (token: string, id: string) => Promise<api.CommandResponse>,
  ) {
    if (!token || !id) return
    setShowMenu(false)
    setShowStatusModal(false)
    setPanelBusy(true)
    lastInteractionAtRef.current = Date.now()
    try {
      const res = await command(token, id)
      applyCommandResponse(res, 'smooth')
    } catch (err: unknown) {
      showToast(toErrorMessage(err, t('hud.requestFailed')), 'error')
    } finally {
      setPanelBusy(false)
    }
  }

  function doSwitchRoom(roomId: string) {
    if (!token || !id) return Promise.resolve()
    setShowMenu(false)
    return execCommand(`go ${roomId}`)
  }

  function doFollowActor(actorId: string | null) {
    if (!token || !id) return Promise.resolve()
    setShowMenu(false)
    return execCommand(actorId ? `follow ${actorId}` : 'unfollow')
  }

  function doChangeLocale(locale: string) {
    if (!token || !id) return Promise.resolve()
    return runPanelCommand((t, i) => api.setLocale(t, i, locale))
  }

  function doExit() {
    setShowExitConfirm(true)
  }

  async function doCreateBookmark(label?: string): Promise<api.BookmarkInfo | null> {
    if (!token || !id) return null
    try {
      const b = await api.createBookmark(token, id, label)
      showToast(t('hud.bookmarkPlaced'), 'info')
      return b
    } catch (err: unknown) {
      showToast(toErrorMessage(err, t('hud.failedPlaceBookmark')), 'error')
      return null
    }
  }

  async function doResumeBookmark(bookmarkId: string): Promise<boolean> {
    if (!token || !id) return false
    setPanelBusy(true)
    try {
      const res = await api.resumeBookmark(token, bookmarkId)
      setLines(res.lines.map(l => ({ text: l.text, kind: l.kind as api.LineKind, key: nextKey.current++ })))
      setUiSnapshot(res.snapshot)
      setActiveMenu(res.snapshot.active_menu ?? null)
      setGameOver(res.snapshot.game_closure !== null)
      setShowMenu(false)
      showToast(t('hud.bookmarkReturned'), 'info')
      return true
    } catch (err: unknown) {
      showToast(toErrorMessage(err, t('hud.failedReturnBookmark')), 'error')
      return false
    } finally {
      setPanelBusy(false)
    }
  }

  async function doDeleteBookmark(bookmarkId: string): Promise<boolean> {
    if (!token) return false
    try {
      await api.deleteBookmark(token, bookmarkId)
      showToast(t('hud.bookmarkDeleted'), 'info')
      return true
    } catch (err: unknown) {
      showToast(toErrorMessage(err, t('hud.failedDeleteBookmark')), 'error')
      return false
    }
  }

  function closeMovie() {
    if (movie && movie.narrative_lines.length > 0) {
      appendLines(movie.narrative_lines.map(text => ({ text, kind: 'narration' })), 'auto')
    }
    setMovie(null)
    setMovieFrame(0)
    refreshSnapshot()
  }

  /** A readable echo for a panel pick (e.g. 攻擊 白石魔像) instead of the raw command id. */
  function describePanelPick(panelName: string, option: api.PanelOptionData): string | undefined {
    const title = option.title?.trim()
    if (!title) return undefined
    // List options carry their own line in the content, so nothing is echoed.
    if (getPanelConfig(panelName)?.layout === 'list') return ''
    const action =
      uiSnapshot?.action_bar_actions.find(a => a.panel === panelName || a.id === panelName) ??
      uiSnapshot?.overflow_actions.find(a => a.panel === panelName || a.id === panelName)
    return action?.label ? `${action.label} ${title}` : title
  }

  function handleSelectPanelOption(panelName: string, option: api.PanelOptionData) {
    if (option.panel) {
      setQuickPanel(option.panel)
      return
    }
    // Multi-select options toggle in place, so the panel stays open.
    if (option.command?.startsWith('toggle:')) {
      void toggleMenuOption(option.command.slice('toggle:'.length))
      return
    }
    const config = getPanelConfig(panelName)
    setQuickPanel(null)
    if (!config) {
      if (option.command) void execCommand(option.command, describePanelPick(panelName, option))
      return
    }
    switch (config.on_select) {
      case 'execute_command':
        if (option.command) void execCommand(option.command, describePanelPick(panelName, option))
        break
      case 'prefill_input':
        setInput(`@${option.title} `)
        setAtSuggestions(null)
        focusInputToEnd()
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
        await execCommand('done', t('hud.done'))
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
    await execCommand(trimmed, displayInput)
    focusInputToEnd()
  }

  const handleTranscriptScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    if (Date.now() - lastProgrammaticScrollRef.current < 600) return
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
    const behavior = scrollBehaviorRef.current
    lastProgrammaticScrollRef.current = Date.now()

    const doScroll = () => {
      if (behavior === 'smooth') {
        bottomRef.current?.scrollIntoView({ behavior: 'smooth' })
      } else if (transcriptRef.current) {
        transcriptRef.current.scrollTop = transcriptRef.current.scrollHeight
      } else {
        bottomRef.current?.scrollIntoView()
      }
    }

    doScroll()
    scrollBehaviorRef.current = 'auto'

    const rafId = requestAnimationFrame(() => {
      if (autoScrollRef.current) {
        doScroll()
      }
    })
    const timer = setTimeout(() => {
      if (autoScrollRef.current) {
        doScroll()
      }
    }, 120)

    return () => {
      cancelAnimationFrame(rafId)
      clearTimeout(timer)
    }
  }, [lines, busyLabel])

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
    if (playState?.title) {
      titleEntries.push({ text: `== ${playState.title} ==`, kind: 'heading', key: nextKey.current++ })
    }
    setLines(titleEntries)

    api.fetchPlayUi(token, id)
      .then(snap => {
        setUiSnapshot(snap)
        setActiveMenu(snap.active_menu ?? null)
        setGameOver(snap.game_closure !== null)
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
        if (playState?.intro_text) {
          setLines([
            ...titleEntries,
            ...playState.intro_text
              .split(/\n\s*\n/)
              .map(paragraph => paragraph.trim())
              .filter(Boolean)
              .map(paragraph => ({ text: paragraph, kind: 'narration' as const, key: nextKey.current++ })),
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
            showToast(toErrorMessage(err, t('hud.failedToLoad')), 'error')
          })
          .finally(() => setInitializing(false))
      })
  }, [token, id])

  useEffect(() => {
    if (gameOver) {
      refreshSnapshot()
    }
  }, [gameOver])


  const handleTickStatus = useCallback((generating: boolean, actorName?: string) => {
    setTickGenerating(generating)
    setTickSpeaker(generating && actorName ? actorName : null)
  }, [])

  useNpcTicks({
    token,
    id,
    gameOver,
    documentVisible,
    intervalMs: uiSnapshot?.npc_tick_interval_ms ?? 0,
    blocked: busy || movie !== null || activeMenu !== null || showMenu || quickPanel !== null || (!uiSnapshot?.autonomous_actor_dialogue && !lines.some(l => l.kind === 'player')),
    inputValue: input,
    onTick: applyCommandResponse,
    onTickStatus: handleTickStatus,
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
    setActClosure,
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
    doCreateBookmark,
    doResumeBookmark,
    doDeleteBookmark,
    send,
    findPanelConfig: getPanelConfig,
    handleSelectPanelOption,
    handleTranscriptScroll,
    focusInputToEnd,
  }
}

