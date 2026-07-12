import * as api from '../api'

export function groupOverflowActions(
  actions: api.OverflowAction[],
  uiText: api.UiSnapshot['ui_text'],
): [string, api.OverflowAction[]][] {
  const map = new Map<string, api.OverflowAction[]>()
  for (const a of actions) {
    const g = localizeCommandGroup(a.group, uiText)
    if (!map.has(g)) map.set(g, [])
    map.get(g)!.push(a)
  }
  return Array.from(map.entries()).sort(([a], [b]) => a.localeCompare(b))
}

function localizeCommandGroup(group: string, uiText: api.UiSnapshot['ui_text']): string {
  switch ((group || '').toLowerCase()) {
    case 'support':
      return uiText.commands_group_support
    case 'other':
    case '':
      return uiText.commands_group_other
    default:
      return group
  }
}

export function groupLookOptions(
  options: api.LookOptionData[],
  uiText: api.UiSnapshot['ui_text'],
): [string, api.LookOptionData[]][] {
  const grouped: [string, api.LookOptionData[]][] = []

  const room = options.filter(option => option.id === '__room__')
  if (room.length > 0) grouped.push([uiText.look_group_room, room])

  const things = options.filter(option => option.id.startsWith('feature:') || option.id.startsWith('item:'))
  if (things.length > 0) grouped.push([uiText.look_group_things, things])

  const people = options.filter(option => option.id.startsWith('actor:'))
  if (people.length > 0) grouped.push([uiText.look_group_people, people])

  const seen = new Set(options.flatMap(option => {
    if (option.id === '__room__') return [option.id]
    if (option.id.startsWith('feature:') || option.id.startsWith('item:') || option.id.startsWith('actor:')) {
      return [option.id]
    }
    return []
  }))
  const other = options.filter(option => !seen.has(option.id))
  if (other.length > 0) grouped.push([uiText.commands_group_other, other])

  return grouped
}
