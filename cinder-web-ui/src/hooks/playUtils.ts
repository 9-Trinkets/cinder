import * as api from '../api'

export type MenuView = 'main' | 'rooms' | 'follow' | 'quests' | 'bookmarks' | 'language'

export type QuickPanel = string | null

export function findPanelConfig(
  uiSnapshot: api.UiSnapshot | null,
  panelName: string,
): api.PanelConfigData | undefined {
  const configured =
    uiSnapshot?.action_bar_actions.find(a => a.panel === panelName)?.panel_config ??
    uiSnapshot?.overflow_actions.find(a => a.panel === panelName)?.panel_config
  if (configured) return configured
  if (panelName.startsWith('party-order-deploy:')) {
    const actorId = panelName.slice('party-order-deploy:'.length)
    const deployMember = uiSnapshot?.party.find(item => item.id === actorId)
    const label = deployMember ? deployMember.label : 'Companion'
    return {
      title: `Deploy Destination — ${label}`,
      prompt: 'Choose a destination room for this companion to hold.',
      data_source: 'actors_in_room',
      on_select: 'execute_command',
    }
  }
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
): Array<{ text: string; kind: api.LineKind }> {
  return (res.lines ?? [])
    .filter(l => l.text.trim())
    .map(l => ({ text: l.text, kind: l.kind }))
}
