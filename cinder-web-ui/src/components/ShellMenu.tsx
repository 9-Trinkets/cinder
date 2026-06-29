import { useState } from 'react'
import Modal from './Modal'
import type { UiSnapshot } from '../api'

type View = 'main' | 'help' | 'objectives' | 'about' | 'rooms' | 'follow' | 'language'

interface ShellMenuProps {
  ui: UiSnapshot
  view: View
  onViewChange: (v: View) => void
  onClose: () => void
  onSwitchRoom: (roomId: string) => void
  onFollowActor: (actorId: string | null) => void
  onChangeLocale: (locale: string) => void
  onExit: () => void
  busy: boolean
}

interface FlatItem {
  id: string
  label: string
}

const CANONICAL_ORDER: string[] = [
  'resume', 'help', 'goals', 'things_to_do', 'objectives',
  'rooms', 'follow', 'language', 'about', 'exit',
]

const CANONICAL_FALLBACK: { id: string; labelKey: string }[] = [
  { id: 'resume', labelKey: 'resume_label' },
  { id: 'help', labelKey: 'help_label' },
  { id: 'objectives', labelKey: 'things_to_do_label' },
  { id: 'language', labelKey: 'language_menu_label' },
  { id: 'about', labelKey: 'about_label' },
  { id: 'exit', labelKey: 'exit_label' },
]

const KNOWN_IDS = new Set([
  'help', 'things_to_do', 'objectives', 'goals', 'about',
  'rooms', 'follow', 'language',
])

function flattenItems(t: UiSnapshot['ui_text']): FlatItem[] {
  if (t.shell_menu.items.length > 0) {
    return t.shell_menu.items.map(item => ({ id: item.id, label: item.label }))
  }

  const out: FlatItem[] = []
  for (const entry of CANONICAL_FALLBACK) {
    out.push({ id: entry.id, label: t[entry.labelKey as keyof typeof t] as string || entry.id })
  }
  return out.filter(i => i.id === 'resume' || i.id === 'exit' || KNOWN_IDS.has(i.id))
}

export default function ShellMenu({
  ui,
  view,
  onViewChange,
  onClose,
  onSwitchRoom,
  onFollowActor,
  onChangeLocale,
  onExit,
  busy,
}: ShellMenuProps) {
  const t = ui.ui_text
  const items = flattenItems(t)

  if (view === 'help') {
    return (
      <Modal title={t.help_label} onClose={() => onViewChange('main')}>
        <pre className="whitespace-pre-wrap font-sans text-sm">{ui.help_text}</pre>
      </Modal>
    )
  }

  if (view === 'objectives') {
    return (
      <Modal title={t.things_to_do_label} onClose={() => onViewChange('main')}>
        {ui.objectives.length === 0 ? (
          <p className="text-muted italic">{t.things_to_do_empty}</p>
        ) : (
          <ul className="space-y-2">
            {ui.objectives.map((o, i) => (
              <li key={i}>
                <p className="font-medium">{o.summary}</p>
                <p className="text-muted text-xs">{o.message}</p>
              </li>
            ))}
          </ul>
        )}
      </Modal>
    )
  }

  if (view === 'about') {
    return (
      <Modal title={t.about_label} onClose={() => onViewChange('main')}>
        <p className="whitespace-pre-wrap">{ui.about_body}</p>
      </Modal>
    )
  }

  if (view === 'rooms') {
    return (
      <Modal title={t.room_switcher_title} onClose={() => onViewChange('main')}>
        {ui.rooms.map((r) => (
          <button
            key={r.id}
            onClick={() => onSwitchRoom(r.id)}
            disabled={busy}
            className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
          >
            <span className="font-medium">{r.title}</span>
            {r.menu_text && <span className="text-muted text-xs ml-2">{r.menu_text}</span>}
          </button>
        ))}
      </Modal>
    )
  }

  if (view === 'follow') {
    return (
      <Modal title={t.follow_actor_title} onClose={() => onViewChange('main')}>
        {ui.follow_options.map((a) => (
          <button
            key={a.id}
            onClick={() => onFollowActor(a.id === 'none' ? null : a.id)}
            disabled={busy}
            className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
          >
            <span className="font-medium">{a.title}</span>
            {a.menu_text && <span className="text-muted text-xs ml-2">{a.menu_text}</span>}
          </button>
        ))}
      </Modal>
    )
  }

  if (view === 'language') {
    return (
      <Modal title={t.language_modal_title} onClose={() => onViewChange('main')}>
        {ui.locale_options.map((l) => (
          <button
            key={l.code}
            onClick={() => onChangeLocale(l.code)}
            disabled={busy || l.code === ui.current_locale}
            className={`block w-full text-left px-3 py-2 rounded border border-subtle disabled:opacity-50 cursor-pointer ${
              l.code === ui.current_locale ? 'bg-pine/20 border-pine' : 'hover:bg-overlay'
            }`}
          >
            <span className="font-medium">{l.label}</span>
            {l.code === ui.current_locale && <span className="text-pine text-xs ml-2">(current)</span>}
          </button>
        ))}
      </Modal>
    )
  }

  return <MainMenu
    items={items}
    t={t}
    ui={ui}
    onViewChange={onViewChange}
    onClose={onClose}
    onExit={onExit}
    busy={busy}
  />
}

interface MainMenuProps {
  items: FlatItem[]
  t: UiSnapshot['ui_text']
  ui: UiSnapshot
  onViewChange: (v: View) => void
  onClose: () => void
  onExit: () => void
  busy: boolean
}

function MainMenu({ items, t, ui, onViewChange, onClose, onExit, busy }: MainMenuProps) {
  const [submenu, setSubmenu] = useState<{ id: string; label: string }[] | null>(null)
  const [submenuTitle, setSubmenuTitle] = useState('')

  if (submenu !== null) {
    return (
      <Modal title={submenuTitle} onClose={onClose}>
        <button
          onClick={() => { setSubmenu(null); setSubmenuTitle('') }}
          className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle cursor-pointer text-sm mb-2"
        >
          &larr; Back
        </button>
        {submenu.map((child) => (
          <button
            key={child.id}
            onClick={() => handleItemClick(child.id, onViewChange, onClose, onExit)}
            className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle cursor-pointer text-sm"
          >
            {child.label}
          </button>
        ))}
      </Modal>
    )
  }

  return (
    <Modal title={t.shell_menu_title} onClose={onClose}>
      {items.map((item) => {
        const packItem = t.shell_menu.items.find(i => i.id === item.id)
        const hasChildren = packItem?.children && packItem.children.length > 0

        if (item.id === 'exit') {
          return (
            <div key={item.id}>
              <hr className="border-subtle my-2" />
              <button
                onClick={onExit}
                className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle cursor-pointer text-sm"
              >
                {item.label}
              </button>
            </div>
          )
        }

        if (item.id === 'resume') {
          return (
            <button
              key={item.id}
              onClick={onClose}
              className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle cursor-pointer text-sm"
            >
              {item.label}
            </button>
          )
        }

        if (hasChildren) {
          const children = packItem!.children!
          return (
            <button
              key={item.id}
              onClick={() => {
                setSubmenu(children)
                setSubmenuTitle(item.label)
              }}
              className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle cursor-pointer text-sm"
            >
              {item.label} &rarr;
            </button>
          )
        }

        return (
          <button
            key={item.id}
            onClick={() => handleItemClick(item.id, onViewChange, onClose, onExit)}
            disabled={busy}
            className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer text-sm"
          >
            {item.label}
          </button>
        )
      })}
    </Modal>
  )
}

const VIEW_ROUTE: Record<string, View> = {
  help: 'help',
  goals: 'objectives',
  things_to_do: 'objectives',
  objectives: 'objectives',
  about: 'about',
  rooms: 'rooms',
  follow: 'follow',
  language: 'language',
}

function handleItemClick(
  id: string,
  onViewChange: (v: View) => void,
  onClose: () => void,
  onExit: () => void,
) {
  if (id === 'exit') { onExit(); return }
  if (id === 'resume') { onClose(); return }
  const view = VIEW_ROUTE[id]
  if (view) onViewChange(view)
}
