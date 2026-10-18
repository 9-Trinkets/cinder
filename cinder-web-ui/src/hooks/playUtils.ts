import * as api from '../api'
import type { MessageKey } from '../i18n/strings'

type Translate = (key: MessageKey, vars?: Record<string, string | number>) => string

export type MenuView = 'main' | 'rooms' | 'follow' | 'quests' | 'bookmarks' | 'language'

export type QuickPanel = string | null

export function findPanelConfig(
  uiSnapshot: api.UiSnapshot | null,
  panelName: string,
  t?: Translate,
): api.PanelConfigData | undefined {
  const configured =
    uiSnapshot?.action_bar_actions.find(a => a.panel === panelName)?.panel_config ??
    uiSnapshot?.overflow_actions.find(a => a.panel === panelName)?.panel_config
  if (configured) return configured
  if (panelName.startsWith('party-order-deploy:')) {
    const actorId = panelName.slice('party-order-deploy:'.length)
    const deployMember = uiSnapshot?.party.find(item => item.id === actorId)
    const label = deployMember ? deployMember.label : (t ? t('hud.companion') : 'Companion')
    return {
      title: t ? t('hud.deployDestination', { label }) : `Deploy Destination — ${label}`,
      prompt: t ? t('hud.deployPrompt') : 'Choose a destination room for this companion to hold.',
      data_source: 'actors_in_room',
      on_select: 'execute_command',
    }
  }
  const member = uiSnapshot?.party.find(item => item.order_panel === panelName)
  if (!member) return undefined
  return {
    title: t ? t('hud.orders', { label: member.label }) : `Orders — ${member.label}`,
    prompt: t ? t('hud.ordersPrompt') : 'Choose how this party member should respond in combat.',
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
