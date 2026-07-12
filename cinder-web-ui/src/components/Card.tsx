import { type ReactNode } from 'react'

interface CardProps {
  children: ReactNode
  className?: string
}

export default function Card({ children, className = '' }: CardProps) {
  return (
    <div className={`rounded-lg border border-subtle bg-overlay ${className}`}>
      {children}
    </div>
  )
}
