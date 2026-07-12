import { type ReactNode } from 'react'

type BadgeColor = 'default' | 'success' | 'accent'

const colorClasses: Record<BadgeColor, string> = {
  default: 'bg-overlay text-text',
  success: 'bg-pine/20 text-foam',
  accent: 'bg-iris/20 text-iris',
}

interface BadgeProps {
  children: ReactNode
  color?: BadgeColor
  className?: string
}

export default function Badge({ children, color = 'default', className = '' }: BadgeProps) {
  return (
    <span className={`shrink-0 rounded-full px-2 py-1 text-xs ${colorClasses[color]} ${className}`}>
      {children}
    </span>
  )
}
