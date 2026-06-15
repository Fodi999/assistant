import * as DialogPrimitive from '@radix-ui/react-dialog';
import * as React from 'react';
import { X } from 'lucide-react';
import { cn } from '../../lib/utils';

export const Dialog = DialogPrimitive.Root;
export const DialogTrigger = DialogPrimitive.Trigger;
export const DialogPortal = DialogPrimitive.Portal;
export const DialogClose = DialogPrimitive.Close;

export const DialogOverlay = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Overlay>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Overlay>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Overlay ref={ref} className={cn('fixed inset-0 z-50 bg-black/85', className)} {...props} />
));
DialogOverlay.displayName = DialogPrimitive.Overlay.displayName;

export const DialogContent = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Content>
>(({ className, children, ...props }, ref) => (
  <DialogPortal>
    <DialogOverlay />
    <DialogPrimitive.Content ref={ref} className={cn('fixed left-1/2 top-1/2 z-50 grid max-h-[92vh] w-[min(94vw,1400px)] -translate-x-1/2 -translate-y-1/2 gap-4 rounded-md border border-zinc-800 bg-zinc-950 p-6 text-zinc-50 shadow-2xl outline-none', className)} {...props}>
      {children}
      <DialogPrimitive.Close className="absolute right-4 top-4 rounded-md border border-zinc-800 bg-zinc-900 p-2 text-zinc-300 hover:text-white">
        <X className="h-5 w-5" />
      </DialogPrimitive.Close>
    </DialogPrimitive.Content>
  </DialogPortal>
));
DialogContent.displayName = DialogPrimitive.Content.displayName;

export const DialogTitle = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Title>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Title>
>(({ className, ...props }, ref) =>
  <DialogPrimitive.Title ref={ref} className={cn('text-lg font-black', className)} {...props} />
);
DialogTitle.displayName = DialogPrimitive.Title.displayName;

export const DialogDescription = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Description>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Description>
>(({ className, ...props }, ref) =>
  <DialogPrimitive.Description ref={ref} className={cn('text-sm text-zinc-400', className)} {...props} />
);
DialogDescription.displayName = DialogPrimitive.Description.displayName;
