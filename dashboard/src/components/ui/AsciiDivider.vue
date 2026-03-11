<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(defineProps<{
  variant?: 'line' | 'dots' | 'stars' | 'arrows' | 'waves' | 'blocks' | 'random'
  color?: 'cyan' | 'magenta' | 'green' | 'yellow' | 'orange' | 'muted'
  length?: number
  direction?: 'horizontal' | 'vertical'
}>(), {
  variant: 'line',
  color: 'muted',
  length: 40,
  direction: 'horizontal'
})

const patterns = {
  line: '─'.repeat(40),
  dots: '·'.repeat(40),
  stars: '★'.repeat(40),
  arrows: '─►'.repeat(13) + '─',
  waves: '〰'.repeat(20),
  blocks: '█'.repeat(40),
  random: computed(() => {
    const chars = '─·━═░▒▓█'
    return Array(40).fill(0).map(() => chars[Math.floor(Math.random() * chars.length)]).join('')
  }).value
}

const colorClass = computed(() => {
  const map = {
    cyan: 'text-cyan-500/40',
    magenta: 'text-fuchsia-500/40',
    green: 'text-green-500/40',
    yellow: 'text-yellow-500/40',
    orange: 'text-orange-500/40',
    muted: 'text-zinc-600'
  }
  return map[props.color]
})

const pattern = computed(() => {
  const p = patterns[props.variant as keyof typeof patterns]
  if (typeof p === 'string') return p
  return p
})
</script>

<template>
  <div v-if="direction === 'horizontal'" :class="['whitespace-nowrap overflow-hidden', colorClass]">
    {{ pattern.slice(0, length) }}
  </div>
  <div v-else class="flex flex-col" :class="colorClass">
    <span v-for="i in length" :key="i">│</span>
  </div>
</template>
