import { useState } from 'react'

export default function Section({
  title,
  defaultOpen = false,
  children,
}: {
  title: string
  defaultOpen?: boolean
  children: React.ReactNode
}) {
  const [open, setOpen] = useState(defaultOpen)
  return (
    <div className="border-b border-subtle pb-1">
      <button
        type="button"
        onClick={() => setOpen(open => !open)}
        className="flex w-full items-center justify-between py-2 text-left text-xs uppercase tracking-wider text-muted hover:text-text transition-colors cursor-pointer"
        aria-expanded={open}
      >
        <span>{title}</span>
        <span className={`transition-transform duration-200 ${open ? 'rotate-90' : ''}`}>›</span>
      </button>
      {open && <div className="pb-2 space-y-1.5">{children}</div>}
    </div>
  )
}
