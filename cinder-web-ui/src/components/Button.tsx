import { type ButtonHTMLAttributes, type ReactNode } from 'react'

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger'
type Size = 'sm' | 'md'

const variantClasses: Record<Variant, string> = {
  primary:
    'bg-pine text-surface font-semibold hover:brightness-110',
  secondary:
    'bg-overlay border border-subtle text-text hover:brightness-110',
  ghost:
    'text-muted hover:text-text',
  danger:
    'text-muted transition duration-200 hover:text-love active:scale-[0.98]',
}

const sizeClasses: Record<Size, string> = {
  sm: 'px-3 py-1.5 text-sm rounded',
  md: 'px-4 py-2 text-sm rounded',
}

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant
  size?: Size
  children: ReactNode
}

export default function Button({
  variant = 'secondary',
  size = 'sm',
  className = '',
  children,
  ...props
}: ButtonProps) {
  return (
    <button
      className={`${sizeClasses[size]} ${variantClasses[variant]} transition duration-200 active:scale-[0.98] disabled:opacity-50 cursor-pointer ${className}`}
      {...props}
    >
      {children}
    </button>
  )
}
