const configuredBase = import.meta.env.VITE_API_BASE_URL?.trim()
const BASE = configuredBase
  ? configuredBase.replace(/\/+$/, '') + '/api'
  : '/api'

export function gameTicksWebSocketUrl(
  playId: string,
  token: string,
  intervalMs: number,
) {
  const url = new URL(
    `${BASE}/games/${encodeURIComponent(playId)}/ws`,
    window.location.origin,
  )
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
  url.searchParams.set('token', token)
  url.searchParams.set('tick_ms', intervalMs.toString())
  return url.toString()
}

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
    if (res.status === 401) {
      localStorage.removeItem('token')
      localStorage.removeItem('playerId')
      if (window.location.pathname !== '/login') {
        window.location.assign('/login')
      }
    }
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

export interface PlayInfo {
  play_id: string
  session_id?: string
  pack_id: string
  created_at: string
  updated_at: string
  title: string
  intro_text: string
  day_number: number
  current_room_name: string
}

export type SessionInfo = PlayInfo

export function createPlay(token: string, packId: string) {
  return req<PlayInfo>('/games', {
    method: 'POST',
    headers: authHeader(token),
    body: JSON.stringify({ pack_id: packId }),
  })
}
export const createSession = createPlay

export function listPlays(token: string) {
  return req<PlayInfo[]>('/games', {
    headers: authHeader(token),
  })
}
export const listSessions = listPlays

export interface MovieFrameData {
  text: string
  duration_ms: number
}

export interface MovieData {
  title: string
  frames: MovieFrameData[]
  narrative_lines: string[]
}

export type LineKind = 'narration' | 'heading' | 'player' | 'error' | 'system' | 'channel'

export interface NarrativeLine {
  kind: LineKind
  text: string
}

export interface CommandResponse {
  text: string
  lines: NarrativeLine[]
  game_over: boolean
  movie: MovieData | null
  act_closure: ActClosureData | null
  game_closure: ActClosureData | null
  ui_snapshot: UiSnapshot | null
}

export function runCommand(token: string, playId: string, input: string) {
  return req<CommandResponse>(`/games/${playId}/command`, {
    method: 'POST',
    headers: authHeader(token),
    body: JSON.stringify({ input }),
  })
}

export function runRealtimeTick(token: string, playId: string) {
  return req<CommandResponse>(`/games/${playId}/tick`, {
    method: 'POST',
    headers: authHeader(token),
  })
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
  panel?: string
  panel_config?: PanelConfigData
}

export interface PanelConfigData {
  title: string
  prompt: string
  data_source: 'actors_in_room' | 'exits' | 'features' | 'craftable_items'
  on_select: 'execute_command' | 'prefill_input' | 'switch_room' | 'follow_actor'
}

export interface PanelOptionData {
  id: string
  title: string
  subtitle?: string
  command?: string
  disabled?: boolean
  selected?: boolean
}

export interface OverflowAction {
  id: string
  label: string
  group: string
  usage: string
  panel?: string
  panel_config?: PanelConfigData
}

export interface LookOptionData {
  id: string
  title: string
  command: string
}

export interface ActiveMenuData {
  prompt: string
  options: MenuOptionItem[]
  max_selections?: number
  min_selections?: number
  selected_ids?: string[]
}

export interface ActClosureData {
  title: string
  subtitle: string | null
  sections: ActClosureSection[]
}

export type ActClosureSection =
  | {
      kind: 'text'
      title: string
      body: string
    }
  | {
      kind: 'rating'
      title: string
      value: number
      max: number
    }

export interface InventoryItem {
  label: string
  count: number
  /** Set for loose room items so the UI can dispatch `take <id>`. */
  id?: string
}

export interface EquippedItem {
  slot: string
  label: string
}

export interface PartyMember {
  id: string
  label: string
  level: number
  hp: number
  hp_max: number
  order: string
  order_panel: string
}

export interface StatValue {
  id: string
  value: number
}

export interface PlayerStatus {
  hp: number
  hp_max: number
  stats: StatValue[]
  level: number
  xp: number
  xp_max: number
}

export interface MinimapRoom {
  id: string
  label: string
  x: number
  y: number
  current: boolean
  visited: boolean
}

export interface MinimapConnection {
  from: string
  to: string
}

export interface MinimapData {
  id: string
  label: string
  fully_revealed: boolean
  visited_count: number
  total_count: number | null
  rooms: MinimapRoom[]
  connections: MinimapConnection[]
}

export interface ConsumableInfo {
  id: string
  label: string
  kind: string
  stock: number
  is_crafted: boolean
}

export interface RoomConsumableGroup {
  feature_label: string
  items: ConsumableInfo[]
}

export interface RelationshipPair {
  actor_a: string
  actor_b: string
  connection: number
  attraction: number
  safety: number
}

export interface ThemeDefinition {
  base: string
  surface: string
  overlay: string
  muted: string
  text: string
  love: string
  gold: string
  rose: string
  pine: string
  foam: string
  iris: string
  highlight_high: string
  crafted_highlight: string
  crt_glow: string
  crt_dim: string
  crt_bez: string
}

export interface UiSnapshot {
  pack_id: string
  title: string
  time_label: string
  npc_tick_interval_ms: number
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
  interactable_labels: string[]
  talk_options: MenuOptionItem[]
  panel_options: Record<string, PanelOptionData[]>
  active_menu: ActiveMenuData | null
  act_closure: ActClosureData | null
  game_closure: ActClosureData | null
  game_over?: boolean
  inventory: InventoryItem[]
  equipped_items: EquippedItem[]
  party: PartyMember[]
  player: PlayerStatus
  minimap: MinimapData | null
  levels_revealed: boolean
  current_room_items: InventoryItem[]
  room_consumables: RoomConsumableGroup[]
  crafted_consumable_labels: string[]
  show_relationship_sidebar: boolean
  relationship_pairs: RelationshipPair[]
  show_vitals_sidebar: boolean
  theme: ThemeDefinition
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
    commands_panel_title: string
    commands_panel_empty: string
    commands_group_other: string
    commands_group_support: string
    commands_group_act: string
    look_panel_title: string
    look_group_room: string
    look_group_things: string
    look_group_people: string
    room_items_sidebar_label: string
    minimap_sidebar_label: string
    minimap_revealed_label: string
    minimap_charted_label: string
    trace_mark_present_label: string
    talk_panel_title: string
    talk_panel_prompt: string
    menu_option_list_title: string
    exit_confirm_title: string
    exit_confirm_body: string
    shell_menu: {
      items: { id: string; label: string; children?: { id: string; label: string }[] }[]
    }
    [key: string]: unknown
  }
}

export function fetchPlayUi(token: string, playId: string) {
  return req<UiSnapshot>(`/games/${playId}/ui`, {
    headers: authHeader(token),
  })
}
export const fetchSessionUi = fetchPlayUi

export function switchRoom(token: string, playId: string, roomId: string) {
  return req<CommandResponse>(`/games/${playId}/room`, {
    method: 'POST',
    headers: authHeader(token),
    body: JSON.stringify({ room_id: roomId }),
  })
}

export function followActor(token: string, playId: string, actorId: string | null) {
  return req<CommandResponse>(`/games/${playId}/follow`, {
    method: 'POST',
    headers: authHeader(token),
    body: JSON.stringify({ actor_id: actorId }),
  })
}

export function setLocale(token: string, playId: string, locale: string) {
  return req<CommandResponse>(`/games/${playId}/locale`, {
    method: 'POST',
    headers: authHeader(token),
    body: JSON.stringify({ locale }),
  })
}

export function continuePlay(token: string, playId: string) {
  return req<CommandResponse>(`/games/${playId}/continue`, {
    method: 'POST',
    headers: authHeader(token),
  })
}
export const continueSession = continuePlay

export function fetchTranscript(token: string, playId: string) {
  return req<NarrativeLine[]>(`/games/${playId}/transcript`, {
    headers: authHeader(token),
  })
}

export function deletePlay(token: string, playId: string) {
  return req<void>(`/games/${playId}`, {
    method: 'DELETE',
    headers: authHeader(token),
  })
}
export const deleteSession = deletePlay

export interface PackInfo {
  id: string
  title: string
  tagline: string
  description: string
  theme: ThemeDefinition
}

export function listPacks(token: string) {
  return req<PackInfo[]>('/packs', {
    headers: authHeader(token),
  })
}
