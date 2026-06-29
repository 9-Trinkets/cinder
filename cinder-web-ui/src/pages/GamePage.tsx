import { useEffect, useState, useRef, type FormEvent } from 'react'
import { useParams, useNavigate, useLocation } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import ShellMenu from '../components/ShellMenu'

interface Line {
  text: string
  key: number
}

interface PendingLine {
  text: string
  key: number
}

type MenuView = 'main' | 'help' | 'objectives' | 'about' | 'rooms' | 'follow' | 'language'

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
  const [showMenu, setShowMenu] = useState(false)
  const [menuView, setMenuView] = useState<MenuView>('main')
  const [uiSnapshot, setUiSnapshot] = useState<api.UiSnapshot | null>(null)
  const channelSurfingOnly = useRef(false)
  const bottomRef = useRef<HTMLDivElement>(null)
  const nextKey = useRef(1)
  const initialized = useRef(false)
  const wsRef = useRef<WebSocket | null>(null)
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

  useEffect(() => {
    return () => {
      if (typewriterTimer.current) {
        clearInterval(typewriterTimer.current)
      }
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

    api.runCommand(token, id, 'look')
      .then(res => {
        setLines(prev => [...prev, { text: res.text, key: nextKey.current++ }])
        if (res.game_over) setGameOver(true)
      })
      .catch(err => {
        setLines(prev => [...prev, { text: `[error: ${err instanceof Error ? err.message : 'failed to load'}]`, key: nextKey.current++ }])
      })
      .finally(() => setBusy(false))
  }, [token, id])

  useEffect(() => {
    if (!token || !id) return
    if (wsRef.current) return

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const wsUrl = `${protocol}//${window.location.host}/api/games/${id}/stream`
    const ws = new WebSocket(wsUrl)
    wsRef.current = ws

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data)
        if (data.type === 'settings') {
          setTypewriterCharMs(data.typewriter_char_ms ?? 40)
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

    ws.onclose = () => {
      wsRef.current = null
    }

    return () => {
      ws.close()
      wsRef.current = null
    }
  }, [token, id])

  function openMenu() {
    setMenuView('main')
    setShowMenu(true)
    if (token && id) {
      api.fetchSessionUi(token, id).then(setUiSnapshot).catch((err) => console.error('fetchSessionUi failed', err))
    }
  }

  function addOutcome(text: string) {
    setLines(prev => [...prev, { text, key: nextKey.current++ }])
  }

  async function doSwitchRoom(roomId: string) {
    if (!token || !id) return
    setShowMenu(false)
    setBusy(true)
    try {
      const res = await api.switchRoom(token, id, roomId)
      addOutcome(res.text)
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

    if (trimmed === '?') {
      openMenu()
      return
    }

    if (trimmed.toLowerCase() === 'move' || trimmed.toLowerCase() === 'follow') {
      const snap = uiSnapshot || await api.fetchSessionUi(token, id).catch(() => null)
      if (snap?.channel_surfing_only) {
        setUiSnapshot(snap)
        setMenuView(trimmed.toLowerCase() === 'move' ? 'rooms' : 'follow')
        setShowMenu(true)
        return
      }
    }

    setBusy(true)
    wsRef.current?.send('pause')
    flushTypewriter()

    const cmdLine: Line = { text: `> ${trimmed}`, key: nextKey.current++ }
    setLines(prev => [...prev, cmdLine])

    try {
      const res = await api.runCommand(token, id, trimmed)
      wsRef.current?.send('resume')
      const outLine: Line = { text: res.text, key: nextKey.current++ }
      setLines(prev => [...prev, outLine])
      if (res.game_over) setGameOver(true)
    } catch (err: unknown) {
      wsRef.current?.send('resume')
      const errLine: Line = { text: `[error: ${err instanceof Error ? err.message : 'request failed'}]`, key: nextKey.current++ }
      setLines(prev => [...prev, errLine])
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="h-screen flex flex-col bg-surface">
      <header className="flex items-center justify-between px-4 py-2 border-b border-subtle shrink-0">
        <div className="flex items-center gap-2">
          <button onClick={() => navigate('/games')} className="text-sm text-muted hover:text-text cursor-pointer">&larr; Sessions</button>
          <button
            onClick={openMenu}
            disabled={busy}
            className="text-sm px-2 py-1 rounded bg-overlay border border-subtle text-text hover:brightness-110 disabled:opacity-50 cursor-pointer"
          >
            &#9776; Menu
          </button>
        </div>
        <button onClick={logout} className="text-sm text-muted hover:text-love cursor-pointer">Log out</button>
      </header>

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
        {gameOver && (
          <p className="text-love font-semibold text-center pt-4">Game Over</p>
        )}
        <div ref={bottomRef} />
      </div>

      <form onSubmit={send} className="flex gap-2 border-t border-subtle px-4 py-3 shrink-0">
        <input
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
        >
          Send
        </button>
      </form>

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
    </div>
  )
}
