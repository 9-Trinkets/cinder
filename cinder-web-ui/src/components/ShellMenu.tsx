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

  const menuItems = t.shell_menu.items.length > 0
    ? t.shell_menu.items
    : [
        { id: 'resume', label: t.resume_label, children: [] as never[] },
        { id: 'help', label: t.help_label, children: [] as never[] },
        { id: 'things_to_do', label: t.things_to_do_label, children: [] as never[] },
        { id: 'language', label: t.language_menu_label, children: [] as never[] },
        { id: 'about', label: t.about_label, children: [] as never[] },
        { id: 'exit', label: t.exit_label, children: [] as never[] },
      ]

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
        if (item.children && item.children.length > 0) {
          return (
            <div key={item.id}>
              <p className="text-xs text-muted uppercase tracking-wider mb-1">{item.label}</p>
              <div className="space-y-1 pl-2 border-l-2 border-subtle">
                {item.children.map((child) => {
                  const childId = child.id as View
                  const action = menuItemAction(childId)
                  if (action === 'exit') {
                    return (
                      <button
                        key={child.id}
                        onClick={onExit}
                        className="block w-full text-left px-3 py-1.5 rounded hover:bg-overlay cursor-pointer text-sm"
                      >
                        {child.label}
                      </button>
                    )
                  }
                  if (action === 'view') {
                    return (
                      <button
                        key={child.id}
                        onClick={() => onViewChange(childId)}
                        className="block w-full text-left px-3 py-1.5 rounded hover:bg-overlay cursor-pointer text-sm"
                      >
                        {child.label}
                      </button>
                    )
                  }
                  return null
                })}
              </div>
            </div>
          )
        }
        const itemId = item.id as View
        const action = menuItemAction(itemId)
        if (action === 'close') {
          return (
            <button
              key={item.id}
              onClick={onClose}
              className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle cursor-pointer"
            >
              {item.label}
            </button>
          )
        }
        if (action === 'exit') {
          return (
            <button
              key={item.id}
              onClick={onExit}
              className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle cursor-pointer"
            >
              {item.label}
            </button>
          )
        }
        if (action === 'view') {
          return (
            <button
              key={item.id}
              onClick={() => onViewChange(itemId)}
              className="block w-full text-left px-3 py-2 rounded hover:bg-overlay border border-subtle cursor-pointer"
            >
              {item.label}
            </button>
          )
        }
        return null
      })}
    </Modal>
  )
}

function menuItemAction(id: string): 'view' | 'exit' | 'close' | 'none' {
  const views = new Set(['help', 'things_to_do', 'objectives', 'about', 'rooms', 'follow', 'language'])
  const closes = new Set(['resume'])
  const exits = new Set(['exit'])
  if (views.has(id)) return 'view'
  if (exits.has(id)) return 'exit'
  if (closes.has(id)) return 'close'
  return 'none'
}
