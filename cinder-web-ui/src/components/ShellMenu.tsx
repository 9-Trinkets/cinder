import { useState } from 'react'
import Modal from './Modal'
import Button from './Button'
import Badge from './Badge'
import StatusPanel from './StatusPanel'
import type { UiSnapshot } from '../api'
import type { MenuView } from '../hooks/playUtils'

interface ShellMenuProps {
  ui: UiSnapshot
  view: MenuView
  onViewChange: (v: MenuView) => void
  onClose: () => void
  onSwitchRoom: (roomId: string) => void
  onFollowActor: (actorId: string | null) => void
  onChangeLocale: (locale: string) => void
  onExit: () => void
  busy: boolean
  onTakeItem?: (itemId: string) => void
  onOpenPanel?: (panel: string) => void
}

interface FlatItem {
  id: string
  label: string
}

const CANONICAL_FALLBACK: { id: string; labelKey: string }[] = [
  { id: 'language', labelKey: 'language_menu_label' },
  { id: 'exit', labelKey: 'exit_label' },
]

const KNOWN_IDS = new Set([
  'rooms', 'follow', 'language',
])

function isKnownMenuItem(id: string): boolean {
  return id === 'exit' || KNOWN_IDS.has(id) || !!VIEW_ROUTE[id]
}

function flattenItems(t: UiSnapshot['ui_text']): FlatItem[] {
  // The menu is pack-authored: only what the pack declares (that maps to a
  // known platform view) is shown, so no capability is forced in for packs
  // that route it elsewhere (e.g. room switching via the Move panel).
  if (t.shell_menu.items.length > 0) {
    return t.shell_menu.items
      .filter(item => isKnownMenuItem(item.id))
      .map(item => ({ id: item.id, label: item.label }))
  }

  return CANONICAL_FALLBACK
    .map(entry => ({ id: entry.id, label: t[entry.labelKey as keyof typeof t] as string || entry.id }))
    .filter(i => isKnownMenuItem(i.id))
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
  onTakeItem,
  onOpenPanel,
}: ShellMenuProps) {
  const t = ui.ui_text
  const items = flattenItems(t)

  if (view === 'rooms') {
    return (
      <Modal title={t.room_switcher_title} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        {ui.rooms.map((r) => (
          <Button
            key={r.id}
            variant="secondary"
            className="block w-full text-left"
            onClick={() => onSwitchRoom(r.id)}
            disabled={busy}
          >
            <span className="font-medium">{r.title}</span>
            {r.menu_text && <span className="text-muted text-xs ml-2">{r.menu_text}</span>}
          </Button>
        ))}
      </Modal>
    )
  }

  if (view === 'follow') {
    return (
      <Modal title={t.follow_actor_title} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        {ui.follow_options.map((a) => (
          <Button
            key={a.id}
            variant="secondary"
            className="block w-full text-left"
            onClick={() => onFollowActor(a.id === 'none' ? null : a.id)}
            disabled={busy}
          >
            <span className="font-medium">{a.title}</span>
            {a.menu_text && <span className="text-muted text-xs ml-2">{a.menu_text}</span>}
          </Button>
        ))}
      </Modal>
    )
  }

  if (view === 'language') {
    return (
      <Modal title={t.language_modal_title} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        {ui.locale_options.map((l) => (
          <Button
            key={l.code}
            variant="secondary"
            className={`block w-full text-left ${l.code === ui.current_locale ? '!bg-pine/20 !border-pine' : ''}`}
            onClick={() => onChangeLocale(l.code)}
            disabled={busy || l.code === ui.current_locale}
          >
            <span className="font-medium">{l.label}</span>
            {l.code === ui.current_locale && <span className="text-pine text-xs ml-2">(current)</span>}
          </Button>
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
    onTakeItem={onTakeItem}
    onOpenPanel={onOpenPanel}
  />
}

interface MainMenuProps {
  items: FlatItem[]
  t: UiSnapshot['ui_text']
  ui: UiSnapshot
  onViewChange: (v: MenuView) => void
  onClose: () => void
  onExit: () => void
  busy: boolean
  onTakeItem?: (itemId: string) => void
  onOpenPanel?: (panel: string) => void
}

function MainMenu({
  items,
  t,
  ui,
  onViewChange,
  onClose,
  onExit,
  busy,
  onTakeItem,
  onOpenPanel,
}: MainMenuProps) {
  const [submenu, setSubmenu] = useState<{ id: string; label: string }[] | null>(null)
  const [submenuTitle, setSubmenuTitle] = useState('')

  if (submenu !== null) {
    return (
      <Modal title={submenuTitle} onClose={onClose}>
        <MenuBackButton onClick={() => { setSubmenu(null); setSubmenuTitle('') }} />
        {submenu.map((child) => (
          <Button
            key={child.id}
            variant="secondary"
            className="block w-full text-left"
            onClick={() => handleItemClick(child.id, onViewChange, onExit)}
          >
            {child.label}
          </Button>
        ))}
      </Modal>
    )
  }

  return (
    <Modal title={t.shell_menu_title} onClose={onClose}>
      <div className="rounded-lg border border-subtle bg-canvas/30 px-3 py-3 text-xs text-muted">
        <div className="flex flex-wrap gap-2">
          <Badge>{ui.current_room_name}</Badge>
          <Badge>
            Day {ui.day_number}{ui.time_label ? ` — ${ui.time_label}` : ''}
          </Badge>
          {ui.followed_actor_name && (
            <Badge color="success">
              Following {ui.followed_actor_name}
            </Badge>
          )}
        </div>
      </div>
      {/* On viewports without the sidebar the menu is the only status surface,
          so render the packed status sections (map, vitals, level, party, …)
          here; the sidebar already shows them on larger screens. */}
      <div className="lg:hidden mb-1">
        <StatusPanel uiSnapshot={ui} onTakeItem={onTakeItem} onOpenPanel={onOpenPanel} />
      </div>
      {items.map((item) => {
        const packItem = t.shell_menu.items.find(i => i.id === item.id)
        const hasChildren = packItem?.children && packItem.children.length > 0

        if (item.id === 'exit') {
          return (
            <div key={item.id}>
              <hr className="border-subtle my-2" />
              <Button
                variant="secondary"
                className="block w-full text-left"
                onClick={onExit}
              >
                {item.label}
              </Button>
            </div>
          )
        }

        if (hasChildren) {
          const children = packItem!.children!
          return (
            <Button
              key={item.id}
              variant="secondary"
              className="block w-full text-left"
              onClick={() => {
                setSubmenu(children)
                setSubmenuTitle(item.label)
              }}
            >
              {item.label} &rarr;
            </Button>
          )
        }

        return (
          <Button
            key={item.id}
            variant="secondary"
            className="block w-full text-left"
            onClick={() => handleItemClick(item.id, onViewChange, onExit)}
            disabled={busy}
          >
            {item.label}
          </Button>
        )
      })}
    </Modal>
  )
}

function MenuBackButton({ onClick }: { onClick: () => void }) {
  return (
    <Button
      variant="secondary"
      className="block w-full text-left mb-2"
      onClick={onClick}
    >
      &larr; Back
    </Button>
  )
}

const VIEW_ROUTE: Record<string, MenuView> = {
  rooms: 'rooms',
  follow: 'follow',
  language: 'language',
}

function handleItemClick(
  id: string,
  onViewChange: (v: MenuView) => void,
  onExit: () => void,
) {
  if (id === 'exit') { onExit(); return }
  const view = VIEW_ROUTE[id]
  if (view) onViewChange(view)
}
