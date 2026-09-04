import { useEffect, useRef } from 'react'
import * as api from '../api'

type TickCallback = (res: api.CommandResponse) => void

export function useNpcTicks(params: {
  token: string | null
  id: string | undefined
  gameOver: boolean
  documentVisible: boolean
  intervalMs: number
  blocked: boolean
  inputValue: string
  onTick: TickCallback
}) {
  const { token, id, gameOver, documentVisible, intervalMs, blocked, inputValue } = params
  const onTickRef = useRef<TickCallback>(() => {})
  onTickRef.current = params.onTick
  const inputValueRef = useRef(inputValue)
  inputValueRef.current = inputValue

  useEffect(() => {
    if (!token || !id || gameOver || !documentVisible) return
    if (intervalMs <= 0) return
    if (blocked) return

    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const wsUrl = `${proto}//${window.location.host}/api/games/${id}/ws?token=${token}&tick_ms=${intervalMs}`
    const ws = new WebSocket(wsUrl)

    ws.onmessage = (event) => {
      if (inputValueRef.current.trim().length > 0) return
      try {
        const res: api.CommandResponse = JSON.parse(event.data)
        if (res.text || res.movie || res.game_over || res.act_closure || res.game_closure) {
          onTickRef.current(res)
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
  }, [token, id, gameOver, documentVisible, intervalMs, blocked])
}
