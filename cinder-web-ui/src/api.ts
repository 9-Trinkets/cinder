const BASE = '/api'

async function req<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...init?.headers,
    },
  })
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText)
    throw new Error(text || `HTTP ${res.status}`)
  }
  return res.json()
}

function authHeader(token: string): HeadersInit {
  return { Authorization: `Bearer ${token}` }
}

export interface AuthResponse {
  token: string
  player_id: string
}

export function signup(username: string, password: string) {
  return req<AuthResponse>('/auth/signup', {
    method: 'POST',
    body: JSON.stringify({ username, password }),
  })
}

export function login(username: string, password: string) {
  return req<AuthResponse>('/auth/login', {
    method: 'POST',
    body: JSON.stringify({ username, password }),
  })
}

export interface SessionInfo {
  session_id: string
  pack_id: string
  created_at: string
  updated_at: string
  title: string
  intro_text: string
}

export function createSession(token: string, packId: string) {
  return req<SessionInfo>('/games', {
    method: 'POST',
    headers: authHeader(token),
    body: JSON.stringify({ pack_id: packId }),
  })
}

export function listSessions(token: string) {
  return req<SessionInfo[]>('/games', {
    headers: authHeader(token),
  })
}

export interface CommandResponse {
  text: string
  game_over: boolean
}

export function runCommand(token: string, sessionId: string, input: string) {
  return req<CommandResponse>(`/games/${sessionId}/command`, {
    method: 'POST',
    headers: authHeader(token),
    body: JSON.stringify({ input }),
  })
}

export function saveGame(token: string, sessionId: string) {
  return req<{ session_id: string; created_at: string }>(
    `/games/${sessionId}/save`,
    { method: 'POST', headers: authHeader(token) },
  )
}

export function listSaves(token: string, sessionId: string) {
  return req<{ session_id: string; created_at: string }[]>(
    `/games/${sessionId}/saves`,
    { headers: authHeader(token) },
  )
}

export interface LoadGameResponse {
  session_id: string
  pack_id: string
}

export function loadGame(token: string, sessionId: string) {
  return req<LoadGameResponse>(`/games/${sessionId}/load`, {
    method: 'POST',
    headers: authHeader(token),
    body: JSON.stringify({ session_id: sessionId }),
  })
}
