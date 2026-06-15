import * as React from 'react';
import { Slot } from '@radix-ui/react-slot';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../../lib/utils';

const buttonVariants = cva(
  'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-[2px] border font-mono text-sm font-black uppercase tracking-[.04em] transition-colors disabled:pointer-events-none disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#607195]/70',
  {
    variants: {
      variant: {
        default: 'border-[#7b8caf] bg-[#607195] text-[#e7e5dc] hover:bg-[#7182a6]',
        secondary: 'border-[#08080b] bg-[#1f1f22] text-[#aaa9a1] hover:bg-[#242428] hover:text-[#e7e5dc]',
        ghost: 'border-transparent bg-transparent text-[#aaa9a1] hover:bg-[#242428] hover:text-[#e7e5dc]',
        destructive: 'border border-red-900/60 bg-red-950/40 text-red-200 hover:bg-red-900/40',
        outline: 'border-[#08080b] bg-transparent text-[#e7e5dc] hover:bg-[#242428]'
      },
      size: {
        default: 'h-10 px-4 py-2',
        sm: 'h-8 px-3 text-xs',
        lg: 'h-12 px-6',
        icon: 'h-10 w-10'
      }
    },
    defaultVariants: {
      variant: 'default',
      size: 'default'
    }
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : 'button';
    return <Comp className={cn(buttonVariants({ variant, size, className }))} ref={ref} {...props} />;
  }
);
Button.displayName = 'Button';

export { buttonVariants };
