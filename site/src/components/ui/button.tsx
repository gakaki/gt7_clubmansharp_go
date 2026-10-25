import * as React from 'react'
import { cn } from '@/lib/utils'

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'default' | 'ghost' | 'outline'
  size?: 'default' | 'sm' | 'icon'
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'default', size = 'default', ...props }, ref) => (
    <button
      ref={ref}
      className={cn(
        'inline-flex items-center justify-center gap-2 rounded-xl font-semibold transition-colors',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-neutral-900/25',
        'disabled:pointer-events-none disabled:opacity-45',
        variant === 'default' && 'bg-neutral-950 text-white hover:bg-neutral-800',
        variant === 'outline' && 'border border-neutral-200 bg-white text-neutral-800 hover:bg-neutral-50',
        variant === 'ghost' && 'text-neutral-700 hover:bg-neutral-100',
        size === 'default' && 'h-10 px-4 text-sm',
        size === 'sm' && 'h-8 px-3 text-xs',
        size === 'icon' && 'size-10 p-0',
        className,
      )}
      {...props}
    />
  ),
)
Button.displayName = 'Button'
