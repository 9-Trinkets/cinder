import { useEffect, useRef } from 'react'
import * as api from '../api'

type TickCallback = (res: api.CommandResponse) => void
type TickStatusCallback = (generating: boolean, actorName?: string) => void

export function useNpcTicks(params: {
  token: string | null
  id: string | undefined
  gameOver: boolean
  documentVisible: boolean
  intervalMs: number
  blocked: boolean
  inputValue: string
  onTick: TickCallback
  onTickStatus?: TickStatusCallback
}) {
  const { token, id, gameOver, documentVisible, intervalMs, blocked, inputValue } = params
  const onTickRef = useRef<TickCallback>(() => {})
  onTickRef.current = params.onTick
  const onTickStatusRef = useRef<TickStatusCallback | undefined>(params.onTickStatus)
  onTickStatusRef.current = params.onTickStatus
  const inputValueRef = useRef(inputValue)
  inputValueRef.current = inputValue

  useEffect(() => {
    if (!token || !id || gameOver || !documentVisible) return
    if (intervalMs <= 0) return
    if (blocked) return

    const ws = new WebSocket(api.gameTicksWebSocketUrl(id, token, intervalMs))

    ws.onmessage = (event) => {
      if (inputValueRef.current.trim().length > 0) return
      try {
        const data = JSON.parse(event.data)
        if (data.type === 'tick_status') {
          onTickStatusRef.current?.(data.status === 'generating', data.actor_name)
          return
        }
        onTickStatusRef.current?.(false)
        const res: api.CommandResponse = data
        if (res.text || res.movie || res.game_over || res.act_closure || res.game_closure) {
          onTickRef.current(res)
        }
      } catch {
        console.error('ws: failed to parse tick message')
      }
    }

    ws.onerror = () => {
      onTickStatusRef.current?.(false)
      ws.close()
    }

    return () => {
      onTickStatusRef.current?.(false)
      ws.close()
    }
  }, [token, id, gameOver, documentVisible, intervalMs, blocked])
}
