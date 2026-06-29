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

interface MenuEntry {
  id: string
  label: string
  kind: 'view' | 'close' | 'exit'
}

const CANONICAL_ORDER = ['resume', 'help', 'goals', 'things_to_do', 'objectives', 'rooms', 'follow', 'language', 'about', 'exit']

const CANONICAL_FALLBACK: { id: string; labelKey: string }[] = [
  { id: 'resume', labelKey: 'resume_label' },
  { id: 'help', labelKey: 'help_label' },
  { id: 'objectives', labelKey: 'things_to_do_label' },
  { id: 'language', labelKey: 'language_menu_label' },
  { id: 'about', labelKey: 'about_label' },
  { id: 'exit', labelKey: 'exit_label' },
]

function buildMenuItems(
  t: UiSnapshot['ui_text'],
): MenuEntry[] {
  const items: { id: string; label: string }[] = []

  if (t.shell_menu.items.length > 0) {
    for (const item of t.shell_menu.items) {
      if (item.children && item.children.length > 0) {
        for (const child of item.children) {
          items.push({ id: child.id, label: child.label })
        }
      } else {
        items.push({ id: item.id, label: item.label })
      }
    }
  } else {
    for (const entry of CANONICAL_FALLBACK) {
      items.push({ id: entry.id, label: t[entry.labelKey as keyof typeof t] as string })
    }
  }

  items.sort((a, b) => {
    const ai = CANONICAL_ORDER.indexOf(a.id)
    const bi = CANONICAL_ORDER.indexOf(b.id)
    return (ai === -1 ? 999 : ai) - (bi === -1 ? 999 : bi)
  })

  return items
      .filter(i => {
        if (i.id === 'resume' || i.id === 'exit') return true
        return new Set(['help', 'things_to_do', 'objectives', 'goals', 'about', 'rooms', 'follow', 'language']).has(i.id)
      })
      .map(i => ({
        id: i.id,
        label: i.label,
        kind: i.id === 'exit' ? 'exit' as const : i.id === 'resume' ? 'close' as const : 'view' as const,
      }))
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

  const menuItems: MenuEntry[] = buildMenuItems(t)

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
        <button
          onClick={() => onFollowActor(null)}
          disabled={busy}
          className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
        >
          Stop following
        </button>
        {ui.follow_options.map((a) => (
          <button
            key={a.id}
            onClick={() => onFollowActor(a.id)}
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

  return (
    <Modal title={t.shell_menu_title} onClose={onClose}>
      {menuItems.map((item) => {
        if (item.kind === 'exit') {
          return (
            <div key={item.id}>
              <hr className="border-subtle my-2" />
              <button onClick={onExit} className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle cursor-pointer text-sm">{item.label}</button>
            </div>
          )
        }
        if (item.kind === 'close') {
          return (
            <button key={item.id} onClick={onClose} className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle cursor-pointer text-sm">{item.label}</button>
          )
        }
        return (
          <button key={item.id} onClick={() => onViewChange(item.id as View)} disabled={busy} className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer text-sm">{item.label}</button>
        )
      })}
    </Modal>
  )
}
