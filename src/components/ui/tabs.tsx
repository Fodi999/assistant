import * as TabsPrimitive from '@radix-ui/react-tabs';
import type * as React from 'react';
import { cn } from '../../lib/utils';

export const Tabs = TabsPrimitive.Root;

export const TabsList = ({ className, ...props }: React.ComponentPropsWithoutRef<typeof TabsPrimitive.List>) => (
  <TabsPrimitive.List className={cn('inline-flex h-10 items-center rounded-md border border-zinc-800 bg-zinc-950 p-1', className)} {...props} />
);

export const TabsTrigger = ({ className, ...props }: React.ComponentPropsWithoutRef<typeof TabsPrimitive.Trigger>) => (
  <TabsPrimitive.Trigger className={cn('inline-flex h-8 items-center justify-center rounded px-3 text-sm font-black text-zinc-400 transition-colors data-[state=active]:bg-orange-500/20 data-[state=active]:text-orange-300', className)} {...props} />
);

export const TabsContent = ({ className, ...props }: React.ComponentPropsWithoutRef<typeof TabsPrimitive.Content>) => (
  <TabsPrimitive.Content className={cn('mt-3 focus-visible:outline-none', className)} {...props} />
);
