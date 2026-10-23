import { useEffect, useState, useCallback } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth } from '../auth'
import * as api from '../api'
import ShellMenu from '../components/ShellMenu'
import Modal from '../components/Modal'
import TranscriptPane from '../components/TranscriptPane'
import FolioPanel from '../components/FolioPanel'
import MovieModal from '../components/MovieModal'
import QuickActionPanel from '../components/QuickActionPanel'
import ConfirmDialog from '../components/ConfirmDialog'
import { themeVars } from '../utils/theme'
import { usePlay } from '../hooks/usePlay'
import ActionBar from '../components/ActionBar'
import { formatClockLabel, useI18n } from '../i18n'

export default function GamePage() {
  const navigate = useNavigate()
  const { logout } = useAuth()
  const { t, locale } = useI18n()
  const play = usePlay()
  const {
    uiSnapshot,
    openMenu,
    busy,
    lines,
    busyLabel,
    actClosure,
    gameClosure,
    gameOver,
    applyCommandResponse,
    token,
    id,
    setActClosure,
    setGameClosure,
    transcriptRef,
    bottomRef,
    handleTranscriptScroll,
    quickPanel,
    setQuickPanel,
    findPanelConfig,
    handleSelectPanelOption,
    execCommand,
    atSuggestions,
    input,
    setInput,
    focusInputToEnd,
    send,
    inputRef,
    setAtSuggestions,
    commandHistoryRef,
    historyIndexRef,
    draftInputRef,
    showMenu,
    setShowMenu,
    menuView,
    setMenuView,
    doSwitchRoom,
    doFollowActor,
    doChangeLocale,
    doExit,
    movie,
    movieFrame,
    setMovieFrame,
    closeMovie,
    showExitConfirm,
    setShowExitConfirm,
    doCreateBookmark,
    doResumeBookmark,
    doDeleteBookmark,
  } = play
  const [showFolio, setShowFolio] = useState(false)
  const [showSidebar, setShowSidebar] = useState(true)
  const [selectedSuggestionIndex, setSelectedSuggestionIndex] = useState(0)

  const handleTriggerAction = useCallback((action: api.ActionBarAction) => {
    if (busy || gameOver) return
    if (action.id === 'look') {
      setQuickPanel(current => current === 'look' ? null : 'look')
      return
    }
    const panel = action.panel as string | undefined
    if (panel) {
      const options = uiSnapshot?.panel_options?.[panel] ?? []
      if (
        options.length === 1 &&
        action.panel_config?.layout !== 'list' &&
        !options[0].disabled &&
        action.id !== 'give' &&
        action.id !== 'trace' &&
        panel !== 'trace' &&
        action.id !== 'drop' &&
        panel !== 'drop'
      ) {
        handleSelectPanelOption(panel, options[0])
        return
      }
      if (options.length >= 1) {
        setQuickPanel(current => current === panel ? null : panel)
        return
      }
    }
    void execCommand(action.id, action.label)
  }, [busy, gameOver, uiSnapshot, handleSelectPanelOption, setQuickPanel, execCommand])

  const handleTakeItem = useCallback((item: api.InventoryItem) => {
    if (busy || gameOver) return
    const idOrName = item.id ?? item.label.toLowerCase()
    void execCommand(`take ${idOrName}`)
  }, [busy, gameOver, execCommand])

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (busy || gameOver) return

      const isInputActive = ['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName ?? '')

      // Focus input on '/' when not already typing
      if (e.key === '/' && !isInputActive && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault()
        inputRef.current?.focus()
        return
      }

      // Escape closes open overlays/panels/modals in priority order, or opens menu if none are open
      if (e.key === 'Escape') {
        if (atSuggestions && atSuggestions.length > 0) {
          e.preventDefault()
          setAtSuggestions(null)
          setSelectedSuggestionIndex(0)
          return
        }
        if (quickPanel) {
          e.preventDefault()
          setQuickPanel(null)
          return
        }
        if (showFolio) {
          e.preventDefault()
          setShowFolio(false)
          return
        }
        if (showExitConfirm) {
          e.preventDefault()
          setShowExitConfirm(false)
          return
        }
        if (movie) {
          e.preventDefault()
          closeMovie()
          return
        }
        if (showMenu) {
          e.preventDefault()
          setShowMenu(false)
          return
        }
        e.preventDefault()
        openMenu()
        return
      }

      // Check for 1-9 shortcuts
      const match = e.key.match(/^[1-9]$/)
      if (match) {
        if (e.metaKey) return
        const allowShortcut = e.ctrlKey || e.altKey || !isInputActive
        if (!allowShortcut) return

        // While a list panel (the pending choices) is open, number keys pick.
        if (quickPanel && findPanelConfig(quickPanel)?.layout === 'list') {
          const choice = uiSnapshot?.panel_options?.[quickPanel]?.[parseInt(e.key, 10) - 1]
          if (choice && !choice.disabled) {
            e.preventDefault()
            handleSelectPanelOption(quickPanel, choice)
          }
          return
        }

        const num = parseInt(e.key, 10)
        const fallbackActions = [
          { id: 'look', label: t('hud.actionLook'), shortcut: 1 },
          { id: 'move', label: t('hud.actionMove'), shortcut: 2 },
          { id: 'follow', label: t('hud.actionFollow'), shortcut: 3 },
        ]
        const actions: any[] = uiSnapshot?.action_bar_actions ?? fallbackActions
        const hasExplicitShortcuts = actions.some(a => a.shortcut !== undefined)
        const targetAction = hasExplicitShortcuts
          ? actions.find(a => a.shortcut === num)
          : actions[num - 1]

        if (targetAction) {
          e.preventDefault()
          handleTriggerAction(targetAction)
        } else if (!hasExplicitShortcuts) {
          const roomItems = (uiSnapshot?.current_room_items ?? []).filter(item => {
            const id = (item.id ?? item.label).toLowerCase()
            return !id.includes('sigil') && !item.label.toLowerCase().includes('sigil')
          })
          const idx = num - 1
          const itemIdx = idx - actions.length
          if (itemIdx >= 0 && itemIdx < roomItems.length) {
            e.preventDefault()
            handleTakeItem(roomItems[itemIdx])
          }
        }
      }

      // Check for '0' shortcut (More actions / overflow menu)
      if (e.key === '0') {
        if (e.metaKey) return
        const allowShortcut = e.ctrlKey || e.altKey || !isInputActive
        if (!allowShortcut) return
        const hasOverflow = Boolean(uiSnapshot && uiSnapshot.overflow_actions?.length > 0)
        if (hasOverflow) {
          e.preventDefault()
          setQuickPanel(current => current === 'overflow' ? null : 'overflow')
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [
    busy,
    gameOver,
    atSuggestions,
    quickPanel,
    findPanelConfig,
    handleSelectPanelOption,
    showFolio,
    showExitConfirm,
    movie,
    showMenu,
    uiSnapshot,
    handleTriggerAction,
    handleTakeItem,
    setAtSuggestions,
    setQuickPanel,
    setShowFolio,
    setShowExitConfirm,
    closeMovie,
    setShowMenu,
    openMenu,
  ])

  useEffect(() => {
    if (!busy && !gameOver && !quickPanel && !showMenu && !showExitConfirm && !showFolio && !movie) {
      focusInputToEnd()
    }
  }, [
    busy,
    gameOver,
    quickPanel,
    showMenu,
    showExitConfirm,
    showFolio,
    movie,
    uiSnapshot?.current_room_name,
    focusInputToEnd,
  ])

  return (
    <div
      style={uiSnapshot?.theme ? themeVars(uiSnapshot.theme) : undefined}
      className="h-dvh flex flex-col bg-surface overflow-hidden"
    >
      <header className="sticky top-0 z-10 bg-surface flex items-center justify-between gap-3 px-4 py-3 border-b border-subtle shrink-0">
        <div className="flex items-center gap-2">
          <button onClick={() => navigate(`/games/pack/${uiSnapshot?.pack_id}`)} className="text-sm text-muted hover:text-text cursor-pointer">&larr; {t('hud.back')}</button>
          <button
            type="button"
            onClick={openMenu}
            disabled={busy}
            className="text-sm px-2.5 py-1 rounded bg-overlay border border-subtle text-text transition duration-200 hover:brightness-110 active:scale-[0.98] disabled:opacity-50 cursor-pointer"
          >{t('hud.menu')}</button>
        </div>
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => setShowSidebar(s => !s)}
            className={`hidden lg:inline-flex text-xs font-mono uppercase tracking-wider px-2.5 py-1 rounded border transition-colors cursor-pointer ${
              showSidebar
                ? 'border-pine/40 text-foam bg-pine/10'
                : 'border-subtle text-muted hover:text-text'
            }`}
            title={showSidebar ? t('hud.hideFolioSidebar') : t('hud.showFolioSidebar')}
          >
            {t('hud.folio')}
          </button>
          <button onClick={logout} className="text-sm text-muted transition duration-200 hover:text-love active:scale-[0.98] cursor-pointer">{t('hud.logout')}</button>
        </div>
      </header>

      {uiSnapshot && (
        <button
          type="button"
          onClick={() => setShowFolio(true)}
          className="lg:hidden w-full text-left px-4 py-2.5 border-b border-subtle/60 bg-canvas/60 hover:bg-overlay/40 transition-colors cursor-pointer flex items-center justify-between"
          aria-label={t('hud.openStatusFolio')}
        >
          <div className="flex items-center gap-2 text-xs text-muted truncate">
            <span className="font-semibold text-text truncate">{uiSnapshot.current_room_name}</span>
            {uiSnapshot.show_clock !== false && (
              <>
                <span className="text-muted/40 font-mono">&bull;</span>
                <span className="font-mono text-muted shrink-0">
                  {t('hud.day', { day: uiSnapshot.day_number })}{uiSnapshot.time_label ? ` — ${formatClockLabel(uiSnapshot.time_label, locale)}` : ''}
                </span>
              </>
            )}
            {uiSnapshot.followed_actor_name && (
              <>
                <span className="text-muted/40 font-mono">&bull;</span>
                <span className="text-foam truncate">{t('hud.withActor', { name: uiSnapshot.followed_actor_name })}</span>
              </>
            )}
          </div>
          <span className="text-xs font-mono uppercase tracking-widest text-muted/70 pl-2 shrink-0">
            {t('hud.folio')} &rsaquo;
          </span>
        </button>
      )}

      <div className="flex-1 flex min-h-0 overflow-hidden">
        <div className="flex-1 flex flex-col overflow-hidden">
          <TranscriptPane
            lines={lines}
            busyLabel={busyLabel}
            actClosure={actClosure}
            gameClosure={gameClosure}
            gameOver={gameOver}
            craftedLabels={uiSnapshot?.crafted_consumable_labels ?? []}
            interactableLabels={uiSnapshot?.interactable_labels ?? []}
            channelHeaderLabel={uiSnapshot?.ui_text.channel_header_label}
            transcriptRef={transcriptRef}
            bottomRef={bottomRef}
            onScroll={handleTranscriptScroll}
            onDismissClosure={() => {
              setActClosure(null)
              if (token && id) {
                api.continuePlay(token, id).then(res => {
                  applyCommandResponse(res)
                }).catch(() => {})
              }
            }}
            onDismissGameClosure={() => setGameClosure(null)}
          />

          <div className="relative border-t border-subtle shrink-0">
            <QuickActionPanel
              panel={quickPanel}
              panelConfig={quickPanel ? findPanelConfig(quickPanel) : undefined}
              uiSnapshot={uiSnapshot}
              busy={busy}
              onClose={() => setQuickPanel(null)}
              onLook={async command => {
                setQuickPanel(null)
                await execCommand(command)
              }}
              onSelectOption={handleSelectPanelOption}
              onOverflow={action => {
                const panel = (action as unknown as Record<string, unknown>).panel as string | undefined
                if (panel) {
                  const options = uiSnapshot?.panel_options?.[panel] ?? []
                  if (
                    options.length === 1 &&
                    !options[0].disabled &&
                    action.id !== 'give' &&
                    action.id !== 'trace' &&
                    panel !== 'trace' &&
                    action.id !== 'drop' &&
                    panel !== 'drop'
                  ) {
                    handleSelectPanelOption(panel, options[0])
                    return
                  }
                  if (options.length >= 1) {
                    setQuickPanel(panel)
                    return
                  }
                }
                setQuickPanel(null)
                void execCommand(action.id, action.label)
              }}
            />
          <ActionBar
            actions={uiSnapshot?.action_bar_actions ?? [
              { id: 'look', label: t('hud.actionLook') },
              { id: 'move', label: t('hud.actionMove') },
              { id: 'follow', label: t('hud.actionFollow') },
            ]}
            roomItems={uiSnapshot?.current_room_items}
            hasOverflow={Boolean(uiSnapshot && uiSnapshot.overflow_actions?.length > 0)}
            busy={busy}
            gameOver={gameOver}
            onAction={handleTriggerAction}
            onTakeItem={handleTakeItem}
            onToggleOverflow={() => setQuickPanel(current => current === 'overflow' ? null : 'overflow')}
          />
          </div>

          {uiSnapshot?.show_player_input !== false && (
          <div className="border-t border-subtle shrink-0 relative">
              {atSuggestions && atSuggestions.length > 0 && (
                <div role="listbox" aria-label={t('hud.talkTo')} className="absolute bottom-full left-4 right-4 mb-1">
                  <div className="max-w-2xl mx-auto rounded border border-subtle bg-overlay shadow-lg overflow-hidden">
                    {atSuggestions.map((opt, idx) => {
                      const safeIdx = selectedSuggestionIndex < atSuggestions.length ? selectedSuggestionIndex : 0
                      const isSelected = idx === safeIdx
                      return (
                        <button
                          key={opt.id}
                          role="option"
                          aria-selected={isSelected}
                          onMouseDown={e => {
                            e.preventDefault()
                            setInput(`@${opt.title} `)
                            setAtSuggestions(null)
                            setSelectedSuggestionIndex(0)
                            focusInputToEnd()
                          }}
                          onMouseEnter={() => setSelectedSuggestionIndex(idx)}
                          className={`w-full text-left px-3 py-2 text-sm transition duration-150 cursor-pointer flex items-center justify-between ${
                            isSelected
                              ? 'bg-pine/20 text-foam font-medium'
                              : 'text-text hover:bg-canvas'
                          }`}
                        >
                          <span>@{opt.title}</span>
                          {opt.menu_text && (
                            <span className="text-xs text-muted font-normal">{opt.menu_text}</span>
                          )}
                        </button>
                      )
                    })}
                  </div>
                </div>
              )}
              <form onSubmit={send} className="max-w-2xl mx-auto flex gap-2 px-4 py-2.5">
                <input
                  ref={inputRef}
                  className="flex-1 px-3 py-2 rounded bg-overlay border border-subtle text-text placeholder-faint focus:outline-none focus:border-pine text-sm font-mono"
                  placeholder={gameOver ? t('hud.placeholderEnded') : t('hud.placeholder')}
                  value={input}
                  onChange={e => {
                    const val = e.target.value
                    setInput(val)
                    if (val.startsWith('@')) {
                      const query = val.slice(1).toLowerCase()
                      const opts = uiSnapshot?.talk_options ?? []
                      setAtSuggestions(opts.filter(o => o.title.toLowerCase().includes(query)))
                      setSelectedSuggestionIndex(0)
                    } else {
                      setAtSuggestions(null)
                      setSelectedSuggestionIndex(0)
                    }
                  }}
                  onKeyDown={e => {
                    if (atSuggestions && atSuggestions.length > 0) {
                      if (e.key === 'ArrowDown') {
                        e.preventDefault()
                        setSelectedSuggestionIndex(prev => (prev + 1) % atSuggestions.length)
                        return
                      }
                      if (e.key === 'ArrowUp') {
                        e.preventDefault()
                        setSelectedSuggestionIndex(prev => (prev - 1 + atSuggestions.length) % atSuggestions.length)
                        return
                      }
                      if (e.key === 'Enter' || e.key === 'Tab') {
                        e.preventDefault()
                        const safeIdx = selectedSuggestionIndex < atSuggestions.length ? selectedSuggestionIndex : 0
                        const opt = atSuggestions[safeIdx] ?? atSuggestions[0]
                        if (opt) {
                          setInput(`@${opt.title} `)
                          setAtSuggestions(null)
                          setSelectedSuggestionIndex(0)
                          focusInputToEnd()
                        }
                        return
                      }
                      if (e.key === 'Escape') {
                        e.preventDefault()
                        e.stopPropagation()
                        setAtSuggestions(null)
                        setSelectedSuggestionIndex(0)
                        return
                      }
                    }

                    const history = commandHistoryRef.current
                    if (e.key === 'ArrowUp' && !atSuggestions?.length) {
                      if (history.length === 0) return
                      e.preventDefault()
                      if (historyIndexRef.current === null) {
                        draftInputRef.current = input
                        historyIndexRef.current = history.length - 1
                      } else if (historyIndexRef.current > 0) {
                        historyIndexRef.current -= 1
                      }
                      setInput(history[historyIndexRef.current])
                    } else if (e.key === 'ArrowDown' && !atSuggestions?.length) {
                      if (historyIndexRef.current === null) return
                      e.preventDefault()
                      if (historyIndexRef.current < history.length - 1) {
                        historyIndexRef.current += 1
                        setInput(history[historyIndexRef.current])
                      } else {
                        historyIndexRef.current = null
                        setInput(draftInputRef.current)
                      }
                    }
                  }}
                  disabled={gameOver}
                  readOnly={busy}
                  autoFocus
                />
                <button
                  type="submit"
                  disabled={busy || gameOver || !input.trim()}
                  className="px-4 py-2 rounded bg-pine text-surface text-sm font-semibold transition duration-200 hover:brightness-110 active:scale-[0.98] disabled:opacity-50 cursor-pointer"
                >{t('hud.send')}</button>
              </form>
            </div>
          )}
        </div>

        {uiSnapshot && showSidebar && (
          <aside className="hidden lg:flex lg:w-64 xl:w-72 2xl:w-80 min-h-0 shrink-0 border-l border-subtle p-4 flex-col text-sm overflow-y-auto">
            <FolioPanel
              uiSnapshot={uiSnapshot}
              onTakeItem={itemId => void execCommand(`take ${itemId}`)}
              onOpenPanel={setQuickPanel}
            />
          </aside>
        )}
      </div>

      {showMenu && uiSnapshot && (
        <ShellMenu
          ui={uiSnapshot}
          view={menuView}
          onViewChange={setMenuView}
          onClose={() => setShowMenu(false)}
          onSwitchRoom={doSwitchRoom}
          onFollowActor={doFollowActor}
          onChangeLocale={doChangeLocale}
          onExit={doExit}
          busy={busy}
          playId={id ?? undefined}
          token={token ?? undefined}
          onCreateBookmark={doCreateBookmark}
          onResumeBookmark={doResumeBookmark}
          onDeleteBookmark={doDeleteBookmark}
        />
      )}

      {showFolio && uiSnapshot && (
        <Modal title={t('hud.travelersFolio')} onClose={() => setShowFolio(false)}>
          <FolioPanel
            uiSnapshot={uiSnapshot}
            onTakeItem={itemId => void execCommand(`take ${itemId}`)}
            onOpenPanel={panel => { setShowFolio(false); setQuickPanel(panel) }}
          />
        </Modal>
      )}

      {movie && (
        <MovieModal
          movie={movie}
          frame={movieFrame}
          onAdvance={() => {
            if (movieFrame < movie.frames.length - 1) {
              setMovieFrame(prev => prev + 1)
            } else {
              closeMovie()
            }
          }}
          onClose={closeMovie}
        />
      )}

      {showExitConfirm && (
        <ConfirmDialog
          title={uiSnapshot?.ui_text?.exit_confirm_title || t('hud.leaveChronicle')}
          message={uiSnapshot?.ui_text?.exit_confirm_body || t('hud.leaveBody')}
          onConfirm={() => navigate('/games')}
          onCancel={() => setShowExitConfirm(false)}
        />
      )}
    </div>
  )
}
