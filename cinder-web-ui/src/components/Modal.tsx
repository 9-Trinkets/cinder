import { type ReactNode } from 'react'

export default function Modal({
  title,
  onClose,
  children,
}: {
  title: string
  onClose: () => void
  children: ReactNode
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-end justify-center bg-black/60 sm:items-center"
      onClick={onClose}
    >
      <div
        className="bg-surface border border-subtle shadow-xl w-full max-w-2xl max-h-[85dvh] min-h-0 flex flex-col rounded-t-2xl sm:rounded-2xl sm:mx-4"
        onClick={e => e.stopPropagation()}
      >
        <div className="mx-auto mt-2 h-1.5 w-12 rounded-full bg-highlight-med sm:hidden" />
        <div className="flex items-center justify-between px-4 py-3 border-b border-subtle shrink-0 sm:px-5">
          <h2 className="text-base font-semibold text-text">{title}</h2>
          <button
            onClick={onClose}
            className="text-muted hover:text-text text-xl leading-none transition duration-200 active:scale-95 cursor-pointer"
          >
            &times;
          </button>
        </div>
        <div className="overflow-y-auto px-4 py-4 pb-[max(1rem,env(safe-area-inset-bottom))] text-sm text-text space-y-3 sm:px-5">
          {children}
        </div>
      </div>
    </div>
  )
}
