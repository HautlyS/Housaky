<script setup lang="ts">
import { cva, type VariantProps } from 'class-variance-authority'
import { cn } from '@/lib/utils'

const badgeVariants = cva(
  'badge-retro inline-flex items-center rounded-none px-3 py-1 text-xs font-semibold transition-all focus:outline-none focus:ring-2 focus:ring-cyan-500 focus:ring-offset-2 focus:ring-offset-zinc-900',
  {
    variants: {
      variant: {
        default: 'border-cyan-500/50 text-cyan-400 bg-cyan-500/10',
        secondary: 'border-zinc-600 text-zinc-400 bg-zinc-800/50',
        destructive: 'border-red-500/50 text-red-400 bg-red-500/10',
        outline: 'border-zinc-600 text-zinc-400',
        success: 'border-green-500/50 text-green-400 bg-green-500/10',
        warning: 'border-yellow-500/50 text-yellow-400 bg-yellow-500/10',
        cyan: 'border-cyan-500/50 text-cyan-400 bg-cyan-500/10',
        magenta: 'border-fuchsia-500/50 text-fuchsia-400 bg-fuchsia-500/10',
        green: 'border-green-500/50 text-green-400 bg-green-500/10',
        orange: 'border-orange-500/50 text-orange-400 bg-orange-500/10',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  }
)

type BadgeVariant = VariantProps<typeof badgeVariants>['variant']

interface Props {
  variant?: BadgeVariant
  class?: string
}

const props = withDefaults(defineProps<Props>(), {
  variant: 'default',
})
</script>

<template>
  <div :class="cn(badgeVariants({ variant }), props.class)">
    <slot />
  </div>
</template>
