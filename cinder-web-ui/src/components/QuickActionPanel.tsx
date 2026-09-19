import { memo, useState, useRef } from 'react'
import * as api from '../api'
import { groupLookOptions, groupOverflowActions } from '../utils/grouping'
import { titleize } from '../utils/text'
import type { QuickPanel } from '../hooks/playUtils'

const QuickActionPanel = memo(function QuickActionPanel({
  panel,
  panelConfig,
  uiSnapshot,
  busy,
  onClose,
  onLook,
  onSelectOption,
  onOverflow,
}: {
  panel: QuickPanel
  panelConfig?: api.PanelConfigData
  uiSnapshot: api.UiSnapshot | null
  busy: boolean
  onClose: () => void
  onLook: (command: string) => Promise<void>
  onSelectOption: (panel: string, option: api.PanelOptionData) => void
  onOverflow: (action: api.OverflowAction) => void
}) {
  const [selectedGiveItem, setSelectedGiveItem] = useState<{ id: string; title: string } | null>(null)

  // Reset give selection whenever panel switches or closes
  const prevPanelRef = useRef(panel)
  if (prevPanelRef.current !== panel) {
    prevPanelRef.current = panel
    if (selectedGiveItem !== null) {
      setSelectedGiveItem(null)
    }
  }

  if (!panel || !uiSnapshot) return null

  const genericOptions = uiSnapshot.panel_options?.[panel] ?? []

  const isGiveStep2 = panel === 'give' && selectedGiveItem !== null

  const title = isGiveStep2
    ? `Give ${titleize(selectedGiveItem.title)}`
    : (panelConfig?.title
      ?? (panel === 'look'
        ? uiSnapshot.ui_text.look_panel_title
        : panel === 'overflow'
          ? uiSnapshot.ui_text.commands_panel_title
          : panel))

  const prompt = isGiveStep2
    ? 'Choose a companion to receive this'
    : panelConfig?.prompt

  const handleClose = () => {
    setSelectedGiveItem(null)
    onClose()
  }

  return (
    <div className="absolute bottom-full inset-x-0 z-20 px-4 pb-2 flex justify-center pointer-events-none">
      <div className="max-w-2xl w-full pointer-events-auto rounded-2xl border border-subtle bg-surface/98 shadow-2xl backdrop-blur-sm">
        <div className="flex items-center justify-between px-4 py-3 border-b border-subtle">
          <div className="flex items-center gap-2.5">
            {isGiveStep2 && (
              <button
                type="button"
                onClick={() => setSelectedGiveItem(null)}
                className="text-xs text-muted hover:text-text px-2 py-0.5 rounded-md border border-subtle hover:border-muted cursor-pointer transition-colors"
                title="Choose a different item"
              >
                &larr; Back
              </button>
            )}
            <div>
              <h3 className="text-sm font-semibold text-text">{title}</h3>
              {prompt && (
                <p className="text-xs text-muted mt-0.5">{prompt}</p>
              )}
            </div>
          </div>
          <button
            onClick={handleClose}
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
                        {titleize(opt.title)}
                      </button>
                    ))}
                  </div>
                </div>
              ))
            )
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
                      <span className="font-medium">{titleize(action.label)}</span>
                      {action.usage && <span className="text-muted text-xs ml-2">— {action.usage}</span>}
                    </button>
                  ))}
                </div>
              ))
            )
          )}

          {panel === 'give' && isGiveStep2 && (
            <div className="grid gap-2 sm:grid-cols-2">
              {uiSnapshot.party.map(member => (
                <button
                  key={member.id}
                  onClick={() => {
                    const item = selectedGiveItem!
                    setSelectedGiveItem(null)
                    onSelectOption('give', {
                      id: `${item.id}:${member.id}`,
                      title: member.label,
                      command: `give ${item.id} to ${member.id}`,
                      disabled: false,
                      selected: false,
                    })
                  }}
                  disabled={busy}
                  className="group block w-full text-left px-3 py-2.5 rounded-xl border border-subtle hover:bg-overlay hover:border-pine cursor-pointer transition duration-200"
                >
                  <span className="flex items-center justify-between gap-3">
                    <span>
                      <span className="block font-medium text-text">{member.label}</span>
                      <span className="block text-muted text-xs mt-0.5">
                        {member.order ? `${member.order.toUpperCase()} · ` : ''}HP {member.hp}/{member.hp_max}
                      </span>
                    </span>
                    <span className="text-pine text-sm opacity-0 group-hover:opacity-100 transition-opacity">&rarr;</span>
                  </span>
                </button>
              ))}
            </div>
          )}

          {panel === 'give' && !isGiveStep2 && (
            genericOptions.length === 0 ? (
              <p className="text-muted italic text-sm px-1">You have no items to give.</p>
            ) : (
              <div className="grid gap-2 sm:grid-cols-2">
                {genericOptions.map(opt => (
                  <button
                    key={opt.id}
                    onClick={() => {
                      if (uiSnapshot.party.length === 1) {
                        const cmd = opt.command || `give ${opt.id} to ${uiSnapshot.party[0].id}`
                        onSelectOption('give', { ...opt, command: cmd })
                      } else {
                        setSelectedGiveItem({ id: opt.id, title: opt.title })
                      }
                    }}
                    disabled={busy || opt.disabled}
                    className="group block w-full text-left px-3 py-2.5 rounded-xl border border-subtle hover:bg-overlay hover:border-muted cursor-pointer transition duration-200"
                  >
                    <span className="flex items-center justify-between gap-3">
                      <span>
                        <span className="block font-medium text-text">{titleize(opt.title)}</span>
                        {opt.subtitle && <span className="block text-muted text-xs mt-0.5">{opt.subtitle}</span>}
                      </span>
                      {uiSnapshot.party.length > 1 && (
                        <span className="text-muted text-xs group-hover:text-text transition-colors">&rsaquo;</span>
                      )}
                    </span>
                  </button>
                ))}
              </div>
            )
          )}

          {panel !== 'look' && panel !== 'overflow' && panel !== 'give' && genericOptions.length === 0 && (
            <p className="text-muted italic text-sm px-1">Nothing available here.</p>
          )}

          {panel !== 'look' && panel !== 'overflow' && panel !== 'give' && genericOptions.length > 0 && (
            <div className="grid gap-2 sm:grid-cols-2">
              {genericOptions.map(opt => (
                <button
                  key={opt.id}
                  onClick={() => onSelectOption(panel, opt)}
                  disabled={busy || opt.disabled}
                  aria-pressed={opt.selected}
                  className={`group block w-full text-left px-3 py-2.5 rounded-xl border transition duration-200 ${
                    opt.selected
                      ? 'border-gold/50 bg-gold/10 text-text cursor-default'
                      : 'border-subtle hover:bg-overlay hover:border-muted cursor-pointer'
                  } disabled:cursor-not-allowed`}
                >
                  <span className="flex items-center justify-between gap-3">
                    <span>
                      <span className="block font-medium">{titleize(opt.title)}</span>
                      {opt.subtitle && <span className="block text-muted text-xs mt-0.5">{opt.subtitle}</span>}
                    </span>
                    {opt.selected && <span className="text-gold text-sm" aria-label="Current order">✓</span>}
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
})

export default QuickActionPanel
