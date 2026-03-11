<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'

interface TextureParticle {
  id: number
  x: number
  y: number
  z: number
  char: string
  color: string
  size: number
  speed: number
  rotation: number
}

const props = withDefaults(defineProps<{
  texture?: 'velvet' | 'paper' | 'wood' | 'marble' | 'concrete' | 'fabric'
  intensity?: number
  showParticles?: boolean
  colorScheme?: 'cyan' | 'magenta' | 'green' | 'amber' | 'mixed'
}>(), {
  texture: 'velvet',
  intensity: 50,
  showParticles: true,
  colorScheme: 'cyan'
})

const particles = ref<TextureParticle[]>([])
const animationId = ref<number>(0)
const canvasRef = ref<HTMLCanvasElement | null>(null)

const textureConfigs = {
  velvet: {
    baseColor: '#1a1520',
    accentColor: '#3d2a4d',
    highlight: '#5a4070',
    noiseScale: 0.4,
    pattern: 'fractal'
  },
  paper: {
    baseColor: '#1c1815',
    accentColor: '#2a2420',
    highlight: '#3d3530',
    noiseScale: 0.8,
    pattern: 'grain'
  },
  wood: {
    baseColor: '#1a1612',
    accentColor: '#2d2518',
    highlight: '#4a3828',
    noiseScale: 0.3,
    pattern: 'grain'
  },
  marble: {
    baseColor: '#151820',
    accentColor: '#252a35',
    highlight: '#3a4050',
    noiseScale: 0.5,
    pattern: 'veins'
  },
  concrete: {
    baseColor: '#181818',
    accentColor: '#252525',
    highlight: '#353535',
    noiseScale: 0.9,
    pattern: 'speckle'
  },
  fabric: {
    baseColor: '#1a1820',
    accentColor: '#2a2835',
    highlight: '#3a3845',
    noiseScale: 0.6,
    pattern: 'weave'
  }
}

const colorSchemes = {
  cyan: ['#00ffff', '#00cccc', '#00aaaa', '#00ffff', '#40ffff'],
  magenta: ['#ff00ff', '#cc00cc', '#aa00aa', '#ff00ff', '#ff40ff'],
  green: ['#00ff41', '#00cc33', '#00aa2a', '#00ff41', '#40ff63'],
  amber: ['#ffb000', '#cc8c00', '#aa7500', '#ffb000', '#ffc040'],
  mixed: ['#00ffff', '#ff00ff', '#00ff41', '#ffb000', '#ffffff']
}

const currentColors = computed(() => colorSchemes[props.colorScheme])

const particleChars = ['@', '#', '$', '%', '&', '*', '?', '!', '+', '=', '·', '•', '○', '●', '◊', '▲', '▼', '◆', '■']

function initParticles() {
  particles.value = []
  const count = Math.floor(props.intensity / 2)
  
  for (let i = 0; i < count; i++) {
    particles.value.push({
      id: i,
      x: Math.random() * 100,
      y: Math.random() * 100,
      z: Math.random() * 100,
      char: particleChars[Math.floor(Math.random() * particleChars.length)],
      color: currentColors.value[Math.floor(Math.random() * currentColors.value.length)],
      size: 8 + Math.random() * 16,
      speed: 0.005 + Math.random() * 0.02,
      rotation: Math.random() * 360
    })
  }
}

function animate() {
  particles.value.forEach(p => {
    p.y -= p.speed
    p.rotation += 0.1
    
    if (p.y < -10) {
      p.y = 110
      p.x = Math.random() * 100
      p.char = particleChars[Math.floor(Math.random() * particleChars.length)]
    }
  })
  
  animationId.value = requestAnimationFrame(animate)
}

function getParticleStyle(p: TextureParticle) {
  const opacity = 0.15 + (p.z / 100) * 0.35
  const scale = 0.5 + (p.z / 100) * 1
  return {
    left: `${p.x}%`,
    top: `${p.y}%`,
    color: p.color,
    fontSize: `${p.size * scale}px`,
    transform: `rotate(${p.rotation}deg) scale(${scale})`,
    opacity: opacity,
    textShadow: `0 0 ${5 + p.z / 10}px ${p.color}`,
    zIndex: Math.floor(p.z)
  }
}

const textureConfig = computed(() => textureConfigs[props.texture])

const textureStyle = computed(() => {
  const cfg = textureConfig.value
  return {
    '--texture-base': cfg.baseColor,
    '--texture-accent': cfg.accentColor,
    '--texture-highlight': cfg.highlight,
    '--noise-scale': cfg.noiseScale
  }
})

onMounted(() => {
  initParticles()
  if (props.showParticles) {
    animate()
  }
})

onUnmounted(() => {
  if (animationId.value) {
    cancelAnimationFrame(animationId.value)
  }
})
</script>

<template>
  <div class="texture-bg" :style="textureStyle">
    <canvas ref="canvasRef" class="texture-canvas" />
    
    <div class="texture-overlay" />
    
    <div class="texture-noise" />
    
    <div v-if="showParticles" class="particle-layer">
      <div 
        v-for="p in particles" 
        :key="p.id"
        class="texture-particle"
        :style="getParticleStyle(p)"
      >
        {{ p.char }}
      </div>
    </div>
    
    <div class="perspective-grid" />
    
    <div class="vignette" />
  </div>
</template>

<style scoped>
.texture-bg {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  z-index: 0;
  background: var(--texture-base);
  transition: background 0.5s ease;
}

.texture-canvas {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  opacity: 0.4;
}

.texture-overlay {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: 
    radial-gradient(ellipse at 30% 20%, var(--texture-accent) 0%, transparent 50%),
    radial-gradient(ellipse at 70% 80%, var(--texture-highlight) 0%, transparent 40%),
    radial-gradient(ellipse at 50% 50%, var(--texture-accent) 0%, transparent 60%);
  opacity: 0.3;
}

.texture-noise {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)'/%3E%3C/svg%3E");
  opacity: calc(var(--noise-scale, 0.5) * 0.08);
  pointer-events: none;
}

.particle-layer {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.texture-particle {
  position: absolute;
  font-family: 'VT323', 'Courier New', monospace;
  font-weight: bold;
  white-space: nowrap;
  transition: none;
  animation: particle-float 6s ease-in-out infinite;
}

@keyframes particle-float {
  0%, 100% {
    opacity: 0.1;
  }
  50% {
    opacity: 0.4;
  }
}

.perspective-grid {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-image: 
    linear-gradient(rgba(255,255,255,0.015) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255,255,255,0.015) 1px, transparent 1px);
  background-size: 60px 60px;
  transform: perspective(800px) rotateX(55deg) scale(1.8);
  transform-origin: center 30%;
  opacity: 0.4;
  animation: grid-pulse 8s ease-in-out infinite;
}

@keyframes grid-pulse {
  0%, 100% {
    opacity: 0.3;
    transform: perspective(800px) rotateX(55deg) scale(1.8);
  }
  50% {
    opacity: 0.5;
    transform: perspective(800px) rotateX(55deg) scale(1.9);
  }
}

.vignette {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: radial-gradient(ellipse at center, transparent 0%, rgba(0,0,0,0.4) 100%);
  pointer-events: none;
}
</style>
