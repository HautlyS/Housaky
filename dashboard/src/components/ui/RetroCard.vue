<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'

const props = withDefaults(defineProps<{
  accent?: 'cyan' | 'magenta' | 'green' | 'orange' | 'yellow'
  glow?: boolean
  padding?: boolean
  variant?: 'default' | 'ascii' | 'minimal' | 'retro'
  resizable?: boolean
  minWidth?: number
  minHeight?: number
  maxWidth?: number
  maxHeight?: number
  initialWidth?: number
  initialHeight?: number
  perspectiveIndex?: number
}>(), {
  accent: 'cyan',
  glow: false,
  padding: true,
  variant: 'default',
  resizable: false,
  minWidth: 200,
  minHeight: 100,
  maxWidth: 1200,
  maxHeight: 800,
  initialWidth: undefined,
  initialHeight: undefined,
  perspectiveIndex: 0
})

const emit = defineEmits<{
  resize: [width: number, height: number]
}>()

const cardRef = ref<HTMLElement | null>(null)
const width = ref(props.initialWidth ?? 0)
const height = ref(props.initialHeight ?? 0)
const isResizing = ref(false)
const resizeDirection = ref('')

const accentClass = computed(() => {
  const map = {
    cyan: 'card-retro-accent',
    magenta: 'card-retro-magenta',
    green: 'card-retro-green',
    orange: 'card-retro-orange',
    yellow: 'card-retro-yellow'
  }
  return map[props.accent]
})

const glowClass = computed(() => {
  if (!props.glow) return ''
  const map = {
    cyan: 'glow-cyan',
    magenta: 'glow-magenta',
    green: 'glow-green',
    orange: 'glow-amber',
    yellow: 'glow-amber'
  }
  return map[props.accent]
})

const perspectiveOpacity = computed(() => {
  const baseOpacity = 1
  const decrement = 0.05
  return Math.max(baseOpacity - (props.perspectiveIndex * decrement), 0.5)
})

const cardStyle = computed(() => {
  if (!props.resizable || !width.value) return {}
  return {
    width: `${width.value}px`,
    height: height.value ? `${height.value}px` : 'auto',
    opacity: perspectiveOpacity.value
  }
})

const asciiBorders = [
  ['┌─────────────────────────────┐', '│', '│', '│', '└─────────────────────────────┘'],
  ['╔═════════════════════════════╗', '║', '║', '║', '╚═════════════════════════════╝'],
  ['┍━━━━━━━━━━━━━━━━━━━━━━━━━━━━┑', '│', '│', '│', '┕━━━━━━━━━━━━━━━━━━━━━━━━━━━━┙'],
  ['╭─────────────────────────────╮', '│', '│', '│', '╰─────────────────────────────╯'],
  ['┌▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓┐', '│', '│', '│', '└▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓┘']
]

const selectedBorder = ref(asciiBorders[0])

onMounted(() => {
  selectedBorder.value = asciiBorders[Math.floor(Math.random() * asciiBorders.length)]
  if (props.resizable && cardRef.value) {
    const rect = cardRef.value.getBoundingClientRect()
    if (!width.value) width.value = rect.width
    if (!height.value) height.value = rect.height
  }
})

function startResize(e: MouseEvent, direction: string) {
  if (!props.resizable) return
  e.preventDefault()
  isResizing.value = true
  resizeDirection.value = direction
  
  const startX = e.clientX
  const startY = e.clientY
  const startWidth = width.value
  const startHeight = height.value
  
  function onMouseMove(e: MouseEvent) {
    const dx = e.clientX - startX
    const dy = e.clientY - startY
    
    let newWidth = startWidth
    let newHeight = startHeight
    
    if (direction.includes('e')) newWidth = Math.min(Math.max(startWidth + dx, props.minWidth), props.maxWidth)
    if (direction.includes('w')) newWidth = Math.min(Math.max(startWidth - dx, props.minWidth), props.maxWidth)
    if (direction.includes('s')) newHeight = Math.min(Math.max(startHeight + dy, props.minHeight), props.maxHeight)
    if (direction.includes('n')) newHeight = Math.min(Math.max(startHeight - dy, props.minHeight), props.maxHeight)
    
    width.value = newWidth
    height.value = newHeight
  }
  
  function onMouseUp() {
    isResizing.value = false
    resizeDirection.value = ''
    emit('resize', width.value, height.value)
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
  }
  
  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}
</script>

<template>
  <div 
    ref="cardRef"
    :class="[
      'card-retro',
      accentClass,
      glowClass,
      padding ? 'p-4' : '',
      resizable ? 'relative' : ''
    ]"
    :style="cardStyle"
  >
    <div v-if="variant === 'retro'" class="corner-decor corner-decor-tl" />
    <div v-if="variant === 'retro'" class="corner-decor corner-decor-tr" />
    <div v-if="variant === 'retro'" class="corner-decor corner-decor-bl" />
    <div v-if="variant === 'retro'" class="corner-decor corner-decor-br" />
    
    <div v-if="resizable" class="resize-handle resize-handle-n" @mousedown="(e) => startResize(e, 'n')" />
    <div v-if="resizable" class="resize-handle resize-handle-s" @mousedown="(e) => startResize(e, 's')" />
    <div v-if="resizable" class="resize-handle resize-handle-e" @mousedown="(e) => startResize(e, 'e')" />
    <div v-if="resizable" class="resize-handle resize-handle-w" @mousedown="(e) => startResize(e, 'w')" />
    <div v-if="resizable" class="resize-handle resize-handle-ne" @mousedown="(e) => startResize(e, 'ne')" />
    <div v-if="resizable" class="resize-handle resize-handle-nw" @mousedown="(e) => startResize(e, 'nw')" />
    <div v-if="resizable" class="resize-handle resize-handle-se" @mousedown="(e) => startResize(e, 'se')" />
    <div v-if="resizable" class="resize-handle resize-handle-sw" @mousedown="(e) => startResize(e, 'sw')" />
    
    <div v-if="variant === 'ascii'" class="relative">
      <pre class="ascii-box absolute top-2 left-2 text-[10px] opacity-25 whitespace-pre leading-tight">{{ selectedBorder.join('\n') }}</pre>
      <div class="pt-6">
        <slot />
      </div>
    </div>
    
    <div v-else-if="variant === 'minimal'">
      <slot />
    </div>
    
    <div v-else>
      <slot />
    </div>
  </div>
</template>
