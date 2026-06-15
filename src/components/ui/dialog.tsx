import * as DialogPrimitive from '@radix-ui/react-dialog';
import type * as React from 'react';
import { X } from 'lucide-react';
import { cn } from '../../lib/utils';

export const Dialog = DialogPrimitive.Root;
export const DialogTrigger = DialogPrimitive.Trigger;
export const DialogPortal = DialogPrimitive.Portal;
export const DialogClose = DialogPrimitive.Close;

export const DialogOverlay = ({ className, ...props }: React.ComponentPropsWithoutRef<typeof DialogPrimitive.Overlay>) => (
  <DialogPrimitive.Overlay className={cn('fixed inset-0 z-50 bg-black/85', className)} {...props} />
);

export const DialogContent = ({ className, children, ...props }: React.ComponentPropsWithoutRef<typeof DialogPrimitive.Content>) => (
  <DialogPortal>
    <DialogOverlay />
    <DialogPrimitive.Content className={cn('fixed left-1/2 top-1/2 z-50 grid max-h-[92vh] w-[min(94vw,1400px)] -translate-x-1/2 -translate-y-1/2 gap-4 rounded-md border border-zinc-800 bg-zinc-950 p-6 text-zinc-50 shadow-2xl outline-none', className)} {...props}>
      {children}
      <DialogPrimitive.Close className="absolute right-4 top-4 rounded-md border border-zinc-800 bg-zinc-900 p-2 text-zinc-300 hover:text-white">
        <X className="h-5 w-5" />
      </DialogPrimitive.Close>
    </DialogPrimitive.Content>
  </DialogPortal>
);

export const DialogTitle = DialogPrimitive.Title;
export const DialogDescription = DialogPrimitive.Description;
