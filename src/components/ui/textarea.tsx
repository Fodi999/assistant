import * as React from 'react';
import { cn } from '../../lib/utils';

export const Textarea = React.forwardRef<HTMLTextAreaElement, React.TextareaHTMLAttributes<HTMLTextAreaElement>>(
  ({ className, ...props }, ref) => (
    <textarea
      ref={ref}
      className={cn('min-h-24 w-full rounded-[2px] border border-[#08080b] bg-[#0d0d10] px-3 py-2 font-mono text-sm text-[#e7e5dc] outline-none transition-colors placeholder:text-[#777973] focus:border-[#607195]', className)}
      {...props}
    />
  )
);
Textarea.displayName = 'Textarea';
