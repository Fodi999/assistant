import * as React from 'react';
import { cn } from '../../lib/utils';

export const Input = React.forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn('h-10 w-full rounded-[2px] border border-[#08080b] bg-[#0d0d10] px-3 py-2 font-mono text-sm text-[#e7e5dc] outline-none transition-colors placeholder:text-[#777973] focus:border-[#607195]', className)}
      {...props}
    />
  )
);
Input.displayName = 'Input';
