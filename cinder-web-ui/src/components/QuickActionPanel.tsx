import { memo } from 'react'
import * as api from '../api'
import { groupLookOptions, groupOverflowActions } from '../utils/grouping'

export type QuickPanel = 'look' | 'talk' | 'overflow' | 'rooms' | 'follow' | null

const QuickActionPanel = memo(function QuickActionPanel({
  panel,
  uiSnapshot,
  busy,
  onClose,
  onLook,
  onSwitchRoom,
  onFollowActor,
  onTalk,
  onOverflow,
}: {
  panel: QuickPanel
  uiSnapshot: api.UiSnapshot | null
  busy: boolean
  onClose: () => void
  onLook: (command: string) => Promise<void>
  onSwitchRoom: (roomId: string) => void
  onFollowActor: (actorId: string | null) => void
  onTalk: (title: string) => void
  onOverflow: (action: api.OverflowAction) => void
}) {
  if (!panel || !uiSnapshot) return null

  return (
    <div className="absolute bottom-full inset-x-0 z-20 px-4 pb-2">
      <div className="rounded-2xl border border-subtle bg-surface/98 shadow-2xl backdrop-blur-sm">
        <div className="flex items-center justify-between px-4 py-3 border-b border-subtle">
          <div>
            <h3 className="text-sm font-semibold text-text">
              {panel === 'look'
                ? uiSnapshot.ui_text.look_panel_title
                : panel === 'talk'
                  ? uiSnapshot.ui_text.talk_panel_title
                  : panel === 'rooms'
                    ? uiSnapshot.ui_text.room_switcher_title
                    : panel === 'follow'
                      ? uiSnapshot.ui_text.follow_actor_title
                  : uiSnapshot.ui_text.commands_panel_title}
            </h3>
            {panel === 'talk' && (
              <p className="text-xs text-muted mt-0.5">{uiSnapshot.ui_text.talk_panel_prompt}</p>
            )}
          </div>
          <button
            onClick={onClose}
            aria-label="Close"
            className="text-muted hover:text-text text-lg leading-none transition duration-200 active:scale-95 cursor-pointer"
          >
            &times;
          </button>
        </div>

        <div className="max-h-[40dvh] overflow-y-auto p-3 space-y-3">
          {panel === 'look' && (
            (uiSnapshot.look_options ?? []).length === 0 ? (
              <p className="text-muted italic text-sm px-1">Nothing of particular interest here.</p>
            ) : (
              groupLookOptions(uiSnapshot.look_options, uiSnapshot.ui_text).map(([group, options]) => (
                <div key={group} className="space-y-2">
                  <p className="text-[11px] text-muted uppercase tracking-wider px-1">{group}</p>
                  <div className="grid gap-2 sm:grid-cols-2">
                    {options.map(opt => (
                      <button
                        key={opt.id}
                        onClick={() => { void onLook(opt.command) }}
                        disabled={busy}
                        className="block w-full text-left px-3 py-2 rounded-xl hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
                      >
                        {opt.title}
                      </button>
                    ))}
                  </div>
                </div>
              ))
            )
          )}

          {panel === 'talk' && (
            <div className="grid gap-2 sm:grid-cols-2">
              {uiSnapshot.talk_options.map(opt => (
                <button
                  key={opt.id}
                  onClick={() => onTalk(opt.title)}
                  disabled={busy}
                  className="block w-full text-left px-3 py-2 rounded-xl hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
                >
                  {opt.title}
                </button>
              ))}
            </div>
          )}

          {panel === 'rooms' && (
            <div className="grid gap-2 sm:grid-cols-2">
              {uiSnapshot.rooms.map(room => (
                <button
                  key={room.id}
                  onClick={() => onSwitchRoom(room.id)}
                  disabled={busy}
                  className="block w-full text-left px-3 py-2 rounded-xl hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
                >
                  <span className="font-medium">{room.title}</span>
                  {room.menu_text && <span className="text-muted text-xs ml-2">{room.menu_text}</span>}
                </button>
              ))}
            </div>
          )}

          {panel === 'follow' && (
            <div className="grid gap-2 sm:grid-cols-2">
              {uiSnapshot.follow_options.map(actor => (
                <button
                  key={actor.id}
                  onClick={() => onFollowActor(actor.id === 'none' ? null : actor.id)}
                  disabled={busy}
                  className="block w-full text-left px-3 py-2 rounded-xl hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
                >
                  <span className="font-medium">{actor.title}</span>
                  {actor.menu_text && <span className="text-muted text-xs ml-2">{actor.menu_text}</span>}
                </button>
              ))}
            </div>
          )}

          {panel === 'overflow' && (
            (uiSnapshot.overflow_actions ?? []).length === 0 ? (
              <p className="text-muted italic text-sm px-1">{uiSnapshot.ui_text.commands_panel_empty}</p>
            ) : (
              groupOverflowActions(uiSnapshot.overflow_actions ?? [], uiSnapshot.ui_text).map(([group, items]) => (
                <div key={group} className="space-y-2">
                  <p className="text-[11px] font-semibold text-muted uppercase tracking-wider px-1">{group}</p>
                  {items.map(action => (
                    <button
                      key={action.id}
                      onClick={() => onOverflow(action)}
                      disabled={busy}
                      className="block w-full text-left px-3 py-2 rounded-xl hover:bg-overlay border border-subtle disabled:opacity-50 cursor-pointer"
                      title={action.usage}
                    >
                      <span className="font-medium">{action.label}</span>
                      {action.usage && <span className="text-muted text-xs ml-2">— {action.usage}</span>}
                    </button>
                  ))}
                </div>
              ))
            )
          )}
        </div>
      </div>
    </div>
  )
})

export default QuickActionPanel
