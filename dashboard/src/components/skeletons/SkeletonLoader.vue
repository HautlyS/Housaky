<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'

interface SkeletonConfig {
  variant: 'text' | 'title' | 'avatar' | 'card' | 'chart' | 'table' | 'list' | 'stat'
  width?: string
  height?: string
  lines?: number
}

const props = defineProps<{
  variant?: 'text' | 'title' | 'avatar' | 'card' | 'chart' | 'table' | 'list' | 'stat' | 'random'
  count?: number
  config?: SkeletonConfig[]
}>()

const randomConfigs: Record<string, () => SkeletonConfig> = {
  text: () => ({
    variant: 'text',
    width: `${60 + Math.random() * 40}%`,
    height: '14px'
  }),
  title: () => ({
    variant: 'title',
    width: `${40 + Math.random() * 30}%`,
    height: '24px'
  }),
  avatar: () => ({
    variant: 'avatar',
    width: '40px',
    height: '40px'
  }),
  card: () => ({
    variant: 'card',
    width: '100%',
    height: `${80 + Math.random() * 100}px`
  }),
  chart: () => ({
    variant: 'chart',
    width: '100%',
    height: '120px'
  }),
  table: () => ({
    variant: 'table',
    lines: 4 + Math.floor(Math.random() * 4)
  }),
  list: () => ({
    variant: 'list',
    lines: 3 + Math.floor(Math.random() * 5)
  }),
  stat: () => ({
    variant: 'stat',
    width: '100px',
    height: '60px'
  })
}

const generatedConfigs = ref<SkeletonConfig[]>([])

function generateRandomConfigs(count: number): SkeletonConfig[] {
  const variants = Object.keys(randomConfigs)
  const configs: SkeletonConfig[] = []
  
  for (let i = 0; i < count; i++) {
    const variant = variants[Math.floor(Math.random() * variants.length)] as string
    configs.push(randomConfigs[variant]())
  }
  
  return configs
}

const finalConfigs = computed(() => {
  if (props.config) return props.config
  if (props.variant === 'random') {
    return generatedConfigs.value
  }
  const count = props.count || 1
  const variant = props.variant || 'text'
  return Array(count).fill(null).map(() => randomConfigs[variant]())
})

onMounted(() => {
  if (props.variant === 'random') {
    generatedConfigs.value = generateRandomConfigs(props.count || 6)
  }
})
</script>

<template>
  <div class="skeleton-wrapper">
    <template v-for="(config, idx) in finalConfigs" :key="idx">
      <!-- Text skeleton -->
      <div 
        v-if="config.variant === 'text'"
        class="skeleton skeleton-text"
        :style="{ width: config.width, height: config.height }"
      />
      
      <!-- Title skeleton -->
      <div 
        v-else-if="config.variant === 'title'"
        class="skeleton skeleton-title"
        :style="{ width: config.width, height: config.height }"
      />
      
      <!-- Avatar skeleton -->
      <div 
        v-else-if="config.variant === 'avatar'"
        class="skeleton skeleton-avatar"
        :style="{ width: config.width, height: config.height }"
      />
      
      <!-- Card skeleton -->
      <div 
        v-else-if="config.variant === 'card'"
        class="skeleton skeleton-card"
        :style="{ width: config.width, height: config.height }"
      />
      
      <!-- Chart skeleton -->
      <div 
        v-else-if="config.variant === 'chart'"
        class="skeleton"
        :style="{ width: config.width, height: config.height }"
      >
        <div class="flex items-end justify-around h-full pb-2 px-2 gap-1">
          <div 
            v-for="i in 8" 
            :key="i" 
            class="skeleton flex-1"
            :style="{ height: `${20 + Math.random() * 70}%` }"
          />
        </div>
      </div>
      
      <!-- Table skeleton -->
      <div 
        v-else-if="config.variant === 'table'"
        class="space-y-2"
      >
        <div class="flex gap-4">
          <div class="skeleton skeleton-text w-20" />
          <div class="skeleton skeleton-text w-32" />
          <div class="skeleton skeleton-text w-24" />
        </div>
        <div 
          v-for="i in (config.lines || 4)" 
          :key="i" 
          class="flex gap-4"
        >
          <div class="skeleton skeleton-text w-16" />
          <div class="skeleton skeleton-text w-40" />
          <div class="skeleton skeleton-text w-20" />
        </div>
      </div>
      
      <!-- List skeleton -->
      <div 
        v-else-if="config.variant === 'list'"
        class="space-y-3"
      >
        <div 
          v-for="i in (config.lines || 3)" 
          :key="i" 
          class="flex items-center gap-3"
        >
          <div class="skeleton skeleton-avatar" />
          <div class="flex-1 space-y-2">
            <div class="skeleton skeleton-text w-3/4" />
            <div class="skeleton skeleton-text w-1/2" />
          </div>
        </div>
      </div>
      
      <!-- Stat skeleton -->
      <div 
        v-else-if="config.variant === 'stat'"
        class="p-4 border border-border rounded"
        :style="{ width: config.width }"
      >
        <div class="skeleton skeleton-text w-12 mb-2" />
        <div class="skeleton skeleton-title w-16" />
      </div>
    </template>
  </div>
</template>
