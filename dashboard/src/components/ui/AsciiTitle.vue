<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  text?: string
  variant?: 'default' | 'minimal' | 'block' | 'double'
  color?: 'cyan' | 'magenta' | 'green' | 'yellow' | 'orange'
  size?: 'sm' | 'md' | 'lg'
}>(), {
  variant: 'default',
  color: 'cyan',
  size: 'md'
})

const asciiArt = {
  default: {
    sm: `
╔══════════╗
║  HOUSAKY ║
╚══════════╝`,
    md: `
╔══════════════════╗
║                  ║
║      HOUSAKY     ║
║                  ║
╚══════════════════╝`,
    lg: `
╔═══════════════════════════════╗
║                               ║
║                               ║
║           HOUSAKY             ║
║                               ║
║                               ║
╚═══════════════════════════════╝`
  },
  minimal: {
    sm: `[ HOUSAKY ]`,
    md: `[ HOUSAKY ]`,
    lg: `[ HOUSAKY ]`
  },
  block: {
    sm: `
████████████
█  HOUSAKY █
████████████`,
    md: `
████████████████
█              █
█    HOUSAKY   █
█              █
████████████████`,
    lg: `
████████████████████████
█                    █
█                    █
█      HOUSAKY       █
█                    █
█                    █
████████████████████████`
  },
  double: {
    sm: `
╔══════════╗
║  HOUSAKY ║
╚══════════╝`,
    md: `
╔══════════════════╗
║                  ║
║      HOUSAKY     ║
║                  ║
╚══════════════════╝`,
    lg: `
╔═══════════════════════════════╗
║                               ║
║                               ║
║           HOUSAKY             ║
║                               ║
║                               ║
╚═══════════════════════════════╝`
  }
}

const colorClass = computed(() => {
  const map = {
    cyan: 'text-cyan-400',
    magenta: 'text-fuchsia-400',
    green: 'text-green-400',
    yellow: 'text-yellow-400',
    orange: 'text-orange-400'
  }
  return map[props.color]
})

const art = computed(() => asciiArt[props.variant]?.[props.size] || asciiArt.default[props.size])
</script>

<template>
  <pre :class="['ascii-title whitespace-pre leading-none', colorClass]">{{ art }}</pre>
</template>
