import { forwardRef, type InputHTMLAttributes } from 'react'

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {}

const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className = '', ...props }, ref) => {
    return (
      <input
        ref={ref}
        className={`px-3 py-2 rounded bg-base border border-subtle text-text placeholder-faint focus:outline-none focus:border-pine text-sm ${className}`}
        {...props}
      />
    )
  }
)

Input.displayName = 'Input'

export default Input
