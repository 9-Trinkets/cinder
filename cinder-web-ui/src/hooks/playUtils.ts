import * as api from '../api'

export type MenuView = 'main' | 'rooms' | 'follow' | 'language'

export function findPanelConfig(
  uiSnapshot: api.UiSnapshot | null,
  panelName: string,
): api.PanelConfigData | undefined {
  const configured =
    uiSnapshot?.action_bar_actions.find(a => a.panel === panelName)?.panel_config ??
    uiSnapshot?.overflow_actions.find(a => a.panel === panelName)?.panel_config
  if (configured) return configured
  const member = uiSnapshot?.party.find(item => item.order_panel === panelName)
  if (!member) return undefined
  return {
    title: `Orders — ${member.label}`,
    prompt: 'Choose how this party member should respond in combat.',
    data_source: 'actors_in_room',
    on_select: 'execute_command',
  }
}

export function extractResponseLines(
  res: api.CommandResponse,
): Array<{ text: string; kind?: api.LineKind }> {
  const typed = (res.lines ?? []).filter(l => l.text.trim())
  if (typed.length > 0) {
    return typed.map(l => ({ text: l.text, kind: l.kind }))
  }
  if (res.text) {
    const chunks = res.text
      .split(/\n\n+/)
      .map(chunk => chunk.trim())
      .filter(Boolean)
    return chunks.length ? chunks.map(text => ({ text })) : [{ text: res.text }]
  }
  return []
}
