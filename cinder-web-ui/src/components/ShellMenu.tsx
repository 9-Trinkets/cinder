import { useEffect, useState } from 'react'
import Modal from './Modal'
import Button from './Button'
import ConfirmDialog from './ConfirmDialog'
import * as api from '../api'
import type { BookmarkInfo, UiSnapshot } from '../api'
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
  playId?: string
  token?: string
  onCreateBookmark?: (label?: string) => Promise<BookmarkInfo | null>
  onResumeBookmark?: (bookmarkId: string) => Promise<boolean>
  onDeleteBookmark?: (bookmarkId: string) => Promise<boolean>
}

interface FlatItem {
  id: string
  label: string
}

const CANONICAL_FALLBACK: { id: string; labelKey: string }[] = [
  { id: 'bookmarks', labelKey: 'bookmark_menu_label' },
  { id: 'language', labelKey: 'language_menu_label' },
  { id: 'exit', labelKey: 'exit_label' },
]

const KNOWN_IDS = new Set([
  'rooms', 'follow', 'quests', 'bookmarks', 'language',
])

function isKnownMenuItem(id: string): boolean {
  return id === 'exit' || KNOWN_IDS.has(id) || !!VIEW_ROUTE[id]
}

function flattenItems(t: UiSnapshot['ui_text']): FlatItem[] {
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
  playId,
  token,
  onCreateBookmark,
  onResumeBookmark,
  onDeleteBookmark,
}: ShellMenuProps) {
  const t = ui.ui_text
  const items = flattenItems(t).filter(item => item.id !== 'quests' || ui.quests_revealed !== false)

  if (view === 'rooms') {
    return (
      <Modal title={t.room_switcher_title || 'Fast Travel'} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40">
          {ui.rooms.map((r) => (
            <button
              key={r.id}
              type="button"
              className="w-full py-3 px-1 flex items-center justify-between group hover:text-foam text-left cursor-pointer transition-colors disabled:opacity-50"
              onClick={() => onSwitchRoom(r.id)}
              disabled={busy}
            >
              <div>
                <span className="text-sm font-medium text-text group-hover:text-foam transition-colors block">{r.title}</span>
                {r.menu_text && <span className="text-xs text-muted">{r.menu_text}</span>}
              </div>
              <span className="text-xs text-foam opacity-0 group-hover:opacity-100 transition-opacity">Travel &rsaquo;</span>
            </button>
          ))}
        </div>
      </Modal>
    )
  }

  if (view === 'follow') {
    return (
      <Modal title={t.follow_actor_title || 'Companions'} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40">
          {ui.follow_options.map((a) => {
            const isFollowing = (a.id === 'none' && !ui.followed_actor_name) ||
              (a.title === ui.followed_actor_name)
            return (
              <button
                key={a.id}
                type="button"
                className="w-full py-3 px-1 flex items-center justify-between group hover:text-foam text-left cursor-pointer transition-colors disabled:opacity-50"
                onClick={() => onFollowActor(a.id === 'none' ? null : a.id)}
                disabled={busy}
              >
                <div>
                  <span className="text-sm font-medium text-text group-hover:text-foam transition-colors block">{a.title}</span>
                  {a.menu_text && <span className="text-xs text-muted">{a.menu_text}</span>}
                </div>
                {isFollowing && (
                  <span className="text-xs font-mono uppercase tracking-wider text-foam bg-pine/15 px-2 py-0.5 rounded border border-pine/30">
                    Active
                  </span>
                )}
              </button>
            )
          })}
        </div>
      </Modal>
    )
  }

  if (view === 'language') {
    return (
      <Modal title={t.language_modal_title || 'Language'} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40">
          {ui.locale_options.map((l) => (
            <button
              key={l.code}
              type="button"
              className={`w-full py-3 px-1 flex items-center justify-between group text-left cursor-pointer transition-colors ${
                l.code === ui.current_locale ? 'text-foam font-semibold' : 'text-text hover:text-foam'
              }`}
              onClick={() => onChangeLocale(l.code)}
              disabled={busy || l.code === ui.current_locale}
            >
              <span className="text-sm">{l.label}</span>
              {l.code === ui.current_locale && (
                <span className="text-xs font-mono uppercase tracking-wider text-foam bg-pine/15 px-2 py-0.5 rounded border border-pine/30">
                  Active
                </span>
              )}
            </button>
          ))}
        </div>
      </Modal>
    )
  }

  if (view === 'quests') {
    const groups = [
      { status: 'active', label: (t.quests_active_label as string) || 'Active' },
      { status: 'completed', label: (t.quests_completed_label as string) || 'Completed' },
      { status: 'failed', label: (t.quests_failed_label as string) || 'Failed' },
    ] as const
    return (
      <Modal title={(t.quests_menu_title as string) || 'Quest Log'} onClose={onClose}>
        <MenuBackButton onClick={() => onViewChange('main')} />
        {ui.quests.length > 0 ? (
          <div className="space-y-5">
            {groups.map(group => {
              const quests = ui.quests.filter(quest => quest.status === group.status)
              if (quests.length === 0) return null
              return (
                <section key={group.status} aria-labelledby={`quests-${group.status}`}>
                  <div className="mb-2 flex items-center gap-2">
                    <h3
                      id={`quests-${group.status}`}
                      className={`text-[10px] font-mono font-semibold uppercase tracking-[0.2em] ${
                        group.status === 'active'
                          ? 'text-foam'
                          : group.status === 'completed'
                          ? 'text-pine'
                          : 'text-love'
                      }`}
                    >
                      {group.label}
                    </h3>
                    <span className="h-px flex-1 bg-subtle/60" />
                    <span className="text-[10px] tabular-nums text-muted">{quests.length}</span>
                  </div>
                  <div className="space-y-2">
                    {quests.map(quest => (
                      <article
                        key={quest.quest_id}
                        className={`rounded-lg border p-3 ${
                          quest.status === 'active'
                            ? 'border-foam/25 bg-foam/5'
                            : quest.status === 'completed'
                            ? 'border-pine/20 bg-pine/5'
                            : 'border-love/25 bg-love/5'
                        }`}
                      >
                        <div className="flex items-start justify-between gap-3">
                          <h4 className="text-sm font-semibold leading-tight text-text">{quest.title}</h4>
                          <span className="shrink-0 text-[9px] font-mono uppercase tracking-[0.14em] text-muted">
                            {quest.kind}
                          </span>
                        </div>
                        {quest.summary && (
                          <p className="mt-1.5 text-xs leading-relaxed text-text/90">{quest.summary}</p>
                        )}
                        {quest.message && quest.message !== quest.summary && (
                          <p className="mt-1 text-[11px] leading-relaxed text-muted">{quest.message}</p>
                        )}
                      </article>
                    ))}
                  </div>
                </section>
              )
            })}
          </div>
        ) : (
          <p className="border-y border-subtle/40 py-5 text-center text-sm text-muted">
            {(t.quests_empty as string) || 'No quests have been recorded yet.'}
          </p>
        )}
      </Modal>
    )
  }

  if (view === 'bookmarks') {
    return (
      <BookmarksView
        playId={playId}
        token={token}
        t={t}
        onViewChange={onViewChange}
        onClose={onClose}
        onCreateBookmark={onCreateBookmark}
        onResumeBookmark={onResumeBookmark}
        onDeleteBookmark={onDeleteBookmark}
        busy={busy}
      />
    )
  }

  return (
    <MainMenu
      items={items}
      t={t}
      ui={ui}
      onViewChange={onViewChange}
      onClose={onClose}
      onExit={onExit}
      busy={busy}
    />
  )
}

interface MainMenuProps {
  items: FlatItem[]
  t: UiSnapshot['ui_text']
  ui: UiSnapshot
  onViewChange: (v: MenuView) => void
  onClose: () => void
  onExit: () => void
  busy: boolean
}

function MainMenu({
  items,
  t,
  ui,
  onViewChange,
  onClose,
  onExit,
  busy,
}: MainMenuProps) {
  const [submenu, setSubmenu] = useState<{ id: string; label: string }[] | null>(null)
  const [submenuTitle, setSubmenuTitle] = useState('')

  if (submenu !== null) {
    return (
      <Modal title={submenuTitle} onClose={onClose}>
        <MenuBackButton onClick={() => { setSubmenu(null); setSubmenuTitle('') }} />
        <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40">
          {submenu.map((child) => (
            <button
              key={child.id}
              type="button"
              className="w-full py-3 px-1 flex items-center justify-between group hover:text-foam text-left cursor-pointer transition-colors"
              onClick={() => handleItemClick(child.id, onViewChange, onExit)}
            >
              <span className="text-sm font-medium text-text group-hover:text-foam transition-colors">{child.label}</span>
              <span className="text-muted group-hover:text-foam group-hover:translate-x-1 transition-transform">&rsaquo;</span>
            </button>
          ))}
        </div>
      </Modal>
    )
  }

  return (
    <Modal title={t.shell_menu_title || 'System Menu'} onClose={onClose}>
      <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40 my-1">
        {items.map((item) => {
          const packItem = t.shell_menu.items.find(i => i.id === item.id)
          const hasChildren = packItem?.children && packItem.children.length > 0

          if (item.id === 'exit') {
            return (
              <button
                key={item.id}
                type="button"
                onClick={onExit}
                className="w-full py-3.5 px-1 flex items-center justify-between group text-left cursor-pointer transition-colors"
              >
                <div>
                  <span className="text-sm font-medium text-text group-hover:text-love transition-colors block">
                    {item.label}
                  </span>
                  <span className="text-xs text-muted">
                    Leave session and return to library
                  </span>
                </div>
                <span className="text-muted group-hover:text-love group-hover:translate-x-1 transition-transform">
                  &rsaquo;
                </span>
              </button>
            )
          }

          let subtitle = ''
          if (item.id === 'rooms') subtitle = 'Fast travel to discovered chambers'
          else if (item.id === 'follow') subtitle = ui.followed_actor_name ? `Accompanying ${ui.followed_actor_name}` : 'Travel unaccompanied'
          else if (item.id === 'quests') {
            const activeCount = ui.quests.filter(quest => quest.status === 'active').length
            subtitle = activeCount === 1 ? '1 active quest' : `${activeCount} active quests`
          }
          else if (item.id === 'bookmarks') subtitle = 'Save and restore game state'
          else if (item.id === 'language') subtitle = ui.locale_options.find(l => l.code === ui.current_locale)?.label || ui.current_locale

          if (hasChildren) {
            const children = packItem!.children!
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => {
                  setSubmenu(children)
                  setSubmenuTitle(item.label)
                }}
                className="w-full py-3.5 px-1 flex items-center justify-between group hover:text-foam text-left cursor-pointer transition-colors"
              >
                <div>
                  <span className="text-sm font-medium text-text group-hover:text-foam transition-colors block">
                    {item.label}
                  </span>
                  {subtitle && <span className="text-xs text-muted">{subtitle}</span>}
                </div>
                <span className="text-muted group-hover:text-foam group-hover:translate-x-1 transition-transform">
                  &rsaquo;
                </span>
              </button>
            )
          }

          return (
            <button
              key={item.id}
              type="button"
              onClick={() => handleItemClick(item.id, onViewChange, onExit)}
              disabled={busy}
              className="w-full py-3.5 px-1 flex items-center justify-between group hover:text-foam text-left cursor-pointer transition-colors disabled:opacity-50"
            >
              <div>
                <span className="text-sm font-medium text-text group-hover:text-foam transition-colors block">
                  {item.label}
                </span>
                {subtitle && <span className="text-xs text-muted">{subtitle}</span>}
              </div>
              <span className="text-muted group-hover:text-foam group-hover:translate-x-1 transition-transform">
                &rsaquo;
              </span>
            </button>
          )
        })}
      </div>
    </Modal>
  )
}

function MenuBackButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="text-xs font-mono uppercase tracking-widest text-muted hover:text-text cursor-pointer flex items-center gap-1.5 mb-3 transition-colors"
    >
      &lsaquo; Back
    </button>
  )
}

const VIEW_ROUTE: Record<string, MenuView> = {
  rooms: 'rooms',
  follow: 'follow',
  quests: 'quests',
  bookmarks: 'bookmarks',
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

function fmtBookmarkDate(s: string): string {
  const d = new Date(s)
  if (!isNaN(d.getTime())) return d.toLocaleString()
  return s
}

interface BookmarksViewProps {
  playId?: string
  token?: string
  t: UiSnapshot['ui_text']
  onViewChange: (v: MenuView) => void
  onClose: () => void
  onCreateBookmark?: (label?: string) => Promise<BookmarkInfo | null>
  onResumeBookmark?: (bookmarkId: string) => Promise<boolean>
  onDeleteBookmark?: (bookmarkId: string) => Promise<boolean>
  busy: boolean
}

function BookmarksView({
  playId,
  token,
  t,
  onViewChange,
  onClose,
  onCreateBookmark,
  onResumeBookmark,
  onDeleteBookmark,
  busy,
}: BookmarksViewProps) {
  const [bookmarks, setBookmarks] = useState<BookmarkInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [labelInput, setLabelInput] = useState('')
  const [creating, setCreating] = useState(false)
  const [actionBusy, setActionBusy] = useState(false)
  const [confirmResumeId, setConfirmResumeId] = useState<string | null>(null)
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null)

  useEffect(() => {
    if (!token || !playId) {
      setLoading(false)
      return
    }
    let active = true
    setLoading(true)
    api.listGameBookmarks(token, playId)
      .then(items => {
        if (active) setBookmarks(items)
      })
      .catch(() => {})
      .finally(() => {
        if (active) setLoading(false)
      })
    return () => { active = false }
  }, [token, playId])

  async function handleCreate(e?: React.FormEvent) {
    if (e) e.preventDefault()
    if (!onCreateBookmark || creating || busy || actionBusy) return
    setCreating(true)
    try {
      const created = await onCreateBookmark(labelInput.trim() || undefined)
      if (created) {
        setBookmarks(prev => [created, ...prev])
        setLabelInput('')
      }
    } finally {
      setCreating(false)
    }
  }

  async function handleResume(bookmarkId: string) {
    if (!onResumeBookmark || actionBusy) return
    setActionBusy(true)
    try {
      const ok = await onResumeBookmark(bookmarkId)
      if (ok) {
        setConfirmResumeId(null)
        onClose()
      }
    } finally {
      setActionBusy(false)
    }
  }

  async function handleDelete(bookmarkId: string) {
    if (!onDeleteBookmark || actionBusy) return
    setActionBusy(true)
    try {
      const ok = await onDeleteBookmark(bookmarkId)
      if (ok) {
        setBookmarks(prev => prev.filter(b => b.id !== bookmarkId))
        setConfirmDeleteId(null)
      }
    } finally {
      setActionBusy(false)
    }
  }

  return (
    <Modal title={(t.bookmark_modal_title as string) || 'Bookmarks'} onClose={onClose}>
      <MenuBackButton onClick={() => onViewChange('main')} />

      <form onSubmit={handleCreate} className="mb-4 flex gap-2">
        <input
          type="text"
          value={labelInput}
          onChange={e => setLabelInput(e.target.value)}
          placeholder="Bookmark note / label (optional)..."
          disabled={creating || busy || actionBusy}
          className="flex-1 px-3 py-1.5 text-xs rounded bg-surface border border-subtle text-text placeholder:text-muted focus:outline-none focus:border-foam"
        />
        <Button
          variant="primary"
          size="sm"
          type="submit"
          disabled={creating || busy || actionBusy || !onCreateBookmark}
          className="text-xs px-3 py-1.5 whitespace-nowrap cursor-pointer"
        >
          {creating ? 'Saving…' : ((t.create_bookmark_label as string) || '+ Save Bookmark')}
        </Button>
      </form>

      {loading ? (
        <div className="py-6 text-center text-xs text-muted">Loading bookmarks…</div>
      ) : bookmarks.length === 0 ? (
        <p className="border-y border-subtle/40 py-6 text-center text-xs text-muted">
          {(t.bookmark_empty as string) || 'No bookmarks saved yet. Save a bookmark to create a restore point.'}
        </p>
      ) : (
        <div className="divide-y divide-subtle/40 border-t border-b border-subtle/40 max-h-72 overflow-y-auto">
          {bookmarks.map(b => (
            <div key={b.id} className="py-3 px-1 flex items-center justify-between gap-3 group">
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium text-text truncate">
                    {b.label || b.current_room_name || 'Save Point'}
                  </span>
                  {b.day_number !== null && b.day_number !== undefined && (
                    <span className="text-[10px] font-mono uppercase tracking-wider text-foam bg-pine/15 px-1.5 py-0.5 rounded border border-pine/30">
                      Day {b.day_number}
                    </span>
                  )}
                </div>
                <p className="text-xs text-muted mt-0.5 truncate">
                  {[
                    b.time_label,
                    b.turn_number !== null && b.turn_number !== undefined ? `Turn ${b.turn_number}` : null,
                    b.current_room_name && b.label ? b.current_room_name : null,
                    fmtBookmarkDate(b.created_at),
                  ].filter(Boolean).join(' • ')}
                </p>
              </div>
              <div className="flex items-center gap-1.5 shrink-0">
                <button
                  type="button"
                  onClick={() => setConfirmResumeId(b.id)}
                  disabled={busy || actionBusy}
                  className="px-2.5 py-1 text-xs font-semibold rounded text-foam hover:bg-pine/20 border border-pine/30 transition-colors cursor-pointer"
                >
                  Resume
                </button>
                <button
                  type="button"
                  onClick={() => setConfirmDeleteId(b.id)}
                  disabled={busy || actionBusy}
                  className="px-2 py-1 text-xs rounded text-muted hover:text-love transition-colors cursor-pointer"
                  title="Delete bookmark"
                >
                  ✕
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {confirmResumeId && (
        <ConfirmDialog
          title="Resume Bookmark"
          message="Resuming this bookmark will replace your current running session and progress. Continue?"
          confirmLabel="Resume"
          onConfirm={() => handleResume(confirmResumeId)}
          onCancel={() => setConfirmResumeId(null)}
        />
      )}

      {confirmDeleteId && (
        <ConfirmDialog
          title="Delete Bookmark"
          message="Are you sure you want to delete this bookmark? This cannot be undone."
          confirmLabel="Delete"
          onConfirm={() => handleDelete(confirmDeleteId)}
          onCancel={() => setConfirmDeleteId(null)}
        />
      )}
    </Modal>
  )
}

