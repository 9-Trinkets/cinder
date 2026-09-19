import { memo } from 'react'
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
  if (!panel || !uiSnapshot) return null

  const genericOptions = uiSnapshot.panel_options?.[panel] ?? []

  const title = panelConfig?.title
    ?? (panel === 'look'
      ? uiSnapshot.ui_text.look_panel_title
      : panel === 'overflow'
        ? uiSnapshot.ui_text.commands_panel_title
        : panel)

  const prompt = panelConfig?.prompt

  return (
    <div className="absolute bottom-full inset-x-0 z-20 px-4 pb-2 flex justify-center pointer-events-none">
      <div className="max-w-2xl w-full pointer-events-auto rounded-2xl border border-subtle bg-surface/98 shadow-2xl backdrop-blur-sm">
        <div className="flex items-center justify-between px-4 py-3 border-b border-subtle">
          <div>
            <h3 className="text-sm font-semibold text-text">{title}</h3>
            {prompt && (
              <p className="text-xs text-muted mt-0.5">{prompt}</p>
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

          {panel !== 'look' && panel !== 'overflow' && genericOptions.length === 0 && (
            <p className="text-muted italic text-sm px-1">Nothing available here.</p>
          )}

          {panel !== 'look' && panel !== 'overflow' && genericOptions.length > 0 && (
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
