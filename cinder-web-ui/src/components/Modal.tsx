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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={onClose}>
      <div
        className="bg-surface border border-subtle rounded-lg shadow-xl max-w-lg w-full mx-4 max-h-[80vh] flex flex-col"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between px-5 py-3 border-b border-subtle shrink-0">
          <h2 className="text-base font-semibold text-text">{title}</h2>
          <button onClick={onClose} className="text-muted hover:text-text text-lg leading-none cursor-pointer">&times;</button>
        </div>
        <div className="overflow-y-auto px-5 py-4 text-sm text-text space-y-3">
          {children}
        </div>
      </div>
    </div>
  )
}
