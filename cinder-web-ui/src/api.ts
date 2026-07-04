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

export interface MovieFrameData {
  text: string
  duration_ms: number
}

export interface MovieData {
  title: string
  frames: MovieFrameData[]
  narrative_lines: string[]
}

export interface CommandResponse {
  text: string
  game_over: boolean
  movie: MovieData | null
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

export interface LocaleItem {
  code: string
  label: string
}

export interface ObjectiveItem {
  summary: string
  message: string
}

export interface MenuOptionItem {
  id: string
  title: string
  menu_text: string
}

export interface ActionBarAction {
  id: string
  label: string
}

export interface OverflowAction {
  id: string
  label: string
  group: string
  usage: string
}

export interface LookOptionData {
  id: string
  title: string
  command: string
}

export interface ActiveMenuData {
  prompt: string
  options: MenuOptionItem[]
}

export interface UiSnapshot {
  title: string
  time_label: string
  day_number: number
  current_room_name: string
  followed_actor_name: string | null
  help_text: string
  about_body: string
  current_locale: string
  locale_options: LocaleItem[]
  objectives: ObjectiveItem[]
  objective_message: string
  progress_completed: number
  progress_total: number
  secrets_found: number
  secrets_total: number
  rooms: MenuOptionItem[]
  follow_options: MenuOptionItem[]
  channel_surfing_only: boolean
  action_bar_actions: ActionBarAction[]
  overflow_actions: OverflowAction[]
  look_options: LookOptionData[]
  talk_options: MenuOptionItem[]
  active_menu: ActiveMenuData | null
  ui_text: {
    language_name: string
    menu_button_label: string
    shell_menu_title: string
    help_label: string
    resume_label: string
    things_to_do_label: string
    about_label: string
    exit_label: string
    language_menu_label: string
    room_switcher_label: string
    room_switcher_title: string
    follow_actor_title: string
    things_to_do_empty: string
    about_body: string
    language_modal_title: string
    modal_close_hint: string
    exit_confirm_title: string
    exit_confirm_body: string
    shell_menu: {
      items: { id: string; label: string; children?: { id: string; label: string }[] }[]
    }
    [key: string]: unknown
  }
}

export function fetchSessionUi(token: string, sessionId: string) {
  return req<UiSnapshot>(`/games/${sessionId}/ui`, {
    headers: authHeader(token),
  })
}

export function switchRoom(token: string, sessionId: string, roomId: string) {
  return req<CommandResponse>(`/games/${sessionId}/room`, {
    method: 'POST',
    headers: authHeader(token),
    body: JSON.stringify({ room_id: roomId }),
  })
}

export function followActor(token: string, sessionId: string, actorId: string | null) {
  return req<CommandResponse>(`/games/${sessionId}/follow`, {
    method: 'POST',
    headers: authHeader(token),
    body: JSON.stringify({ actor_id: actorId }),
  })
}

export function setLocale(token: string, sessionId: string, locale: string) {
  return req<string>(`/games/${sessionId}/locale`, {
    method: 'POST',
    headers: authHeader(token),
    body: JSON.stringify({ locale }),
  })
}

export function fetchTranscript(token: string, sessionId: string) {
  return req<string[]>(`/games/${sessionId}/transcript`, {
    headers: authHeader(token),
  })
}

export function deleteSession(token: string, sessionId: string) {
  return req<void>(`/games/${sessionId}`, {
    method: 'DELETE',
    headers: authHeader(token),
  })
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
