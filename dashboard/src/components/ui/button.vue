<script setup lang="ts">
import { cn } from '@/lib/utils'

interface Props {
  variant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link' | 'retro' | 'retro-primary' | 'retro-success' | 'retro-danger'
  size?: 'default' | 'sm' | 'lg' | 'icon'
  asChild?: boolean
}

withDefaults(defineProps<Props>(), {
  variant: 'default',
  size: 'default',
  asChild: false,
})
</script>

<template>
  <component
    :is="$attrs.asChild ? 'span' : 'button'"
    :class="cn(
      'btn-retro inline-flex items-center justify-center whitespace-nowrap transition-all duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-500 focus-visible:ring-offset-2 focus-visible:ring-offset-zinc-900 disabled:pointer-events-none disabled:opacity-50',
      {
        'bg-cyan-500 text-zinc-900 hover:bg-cyan-400 font-semibold': $props.variant === 'default',
        'border-red-500/50 text-red-400 hover:bg-red-500/10': $props.variant === 'destructive',
        'border border-zinc-700 text-zinc-300 hover:bg-zinc-800 hover:border-zinc-600': $props.variant === 'outline',
        'bg-zinc-800 text-zinc-200 hover:bg-zinc-700': $props.variant === 'secondary',
        'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/50': $props.variant === 'ghost',
        'text-cyan-400 underline-offset-4 hover:underline': $props.variant === 'link',
        'border-zinc-700 text-zinc-300 hover:border-cyan-500 hover:text-cyan-400 hover:bg-cyan-500/5': $props.variant === 'retro',
        'bg-cyan-500 text-zinc-900 border-2 border-cyan-500 hover:bg-transparent hover:text-cyan-400': $props.variant === 'retro-primary',
        'border-green-500/50 text-green-400 hover:bg-green-500/10': $props.variant === 'retro-success',
        'border-2 border-red-600/50 text-red-400 hover:bg-red-600/10': $props.variant === 'retro-danger',
        'h-10 px-4 py-2': $props.size === 'default',
        'h-9 rounded-md px-3 text-xs': $props.size === 'sm',
        'h-11 rounded-md px-8': $props.size === 'lg',
        'h-10 w-10': $props.size === 'icon',
      },
      $attrs.class as string
    )"
  >
    <slot />
  </component>
</template>
