<script setup lang="ts">
import { cn } from '@/lib/utils'

interface Props {
  class?: string
  active?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  active: false,
})
</script>

<template>
  <button
    :class="cn(
      'inline-flex items-center justify-center whitespace-nowrap rounded-sm px-3 py-1.5 text-sm font-medium ring-offset-background transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50',
      props.active
        ? 'bg-background text-foreground shadow-sm'
        : 'text-muted-foreground hover:text-foreground',
      props.class
    )"
  >
    <slot />
  </button>
</template>
