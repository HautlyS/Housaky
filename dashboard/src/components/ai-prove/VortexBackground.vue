<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'

interface VortexChar {
  char: string
  x: number
  y: number
  speed: number
  color: string
  size: number
  rotation: number
}

const japaneseChars = 'アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン'

const symbols = '@#$%&*<>[]{}~`'

const vortexChars = ref<VortexChar[]>([])
const canvasRef = ref<HTMLCanvasElement | null>(null)
const animationId = ref<number>(0)

const charSets = [japaneseChars, symbols, '0123456789ABCDEF', 'qwertyuiopasdfghjklzxcvbnm']

const colors = [
  '#ff0000', '#ff4400', '#ff8800', '#ffcc00',
  '#00ff00', '#00ffcc', '#00ccff', '#0088ff',
  '#8800ff', '#ff00ff', '#ff0088'
]

const bgColors = [
  'rgba(0, 0, 0, 0.95)',
  'rgba(10, 5, 20, 0.95)',
  'rgba(5, 15, 10, 0.95)',
]

const bgColor = ref(0)

function initChars() {
  vortexChars.value = []
  const count = 80
  
  for (let i = 0; i < count; i++) {
    const charSet = charSets[Math.floor(Math.random() * charSets.length)]
    vortexChars.value.push({
      char: charSet[Math.floor(Math.random() * charSet.length)],
      x: Math.random() * 100,
      y: Math.random() * 100,
      speed: 0.01 + Math.random() * 0.03,
      color: colors[Math.floor(Math.random() * colors.length)],
      size: 10 + Math.random() * 20,
      rotation: Math.random() * 360
    })
  }
}

function animate() {
  vortexChars.value.forEach(char => {
    char.y -= char.speed
    char.rotation += 0.5
    
    if (char.y < -10) {
      char.y = 110
      char.x = Math.random() * 100
      const charSet = charSets[Math.floor(Math.random() * charSets.length)]
      char.char = charSet[Math.floor(Math.random() * charSet.length)]
    }
  })
  
  animationId.value = requestAnimationFrame(animate)
}

function cycleBackground() {
  bgColor.value = (bgColor.value + 1) % bgColors.length
}

onMounted(() => {
  initChars()
  animate()
  
  setInterval(cycleBackground, 5000)
})

onUnmounted(() => {
  if (animationId.value) {
    cancelAnimationFrame(animationId.value)
  }
})
</script>

<template>
  <div class="vortex-container" :style="{ backgroundColor: bgColors[bgColor] }">
    <canvas ref="canvasRef" class="vortex-canvas"></canvas>
    
    <div class="vortex-layer">
      <div 
        v-for="(item, idx) in vortexChars" 
        :key="idx"
        class="vortex-char"
        :style="{
          left: `${item.x}%`,
          top: `${item.y}%`,
          color: item.color,
          fontSize: `${item.size}px`,
          transform: `rotate(${item.rotation}deg)`,
          animationDelay: `${idx * 0.1}s`
        }"
      >
        {{ item.char }}
      </div>
    </div>
    
    <div class="vortex-center">
      <div class="spiral"></div>
    </div>
    
    <div class="vortex-grid"></div>
  </div>
</template>

<style scoped>
.vortex-container {
  position: fixed;
  top: 0
  left: 0;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  transition: background-color 2s ease;
  z-index: 0;
}

.vortex-canvas {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  opacity: 0.3;
}

.vortex-layer {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.vortex-char {
  position: absolute;
  font-family: 'Courier New', monospace;
  font-weight: bold;
  text-shadow: 0 0 10px currentColor, 0 0 20px currentColor;
  opacity: 0.8;
  animation: float 8s ease-in-out infinite;
  white-space: nowrap;
}

@keyframes float {
  0%, 100% {
    opacity: 0.6;
    transform: rotate(var(--rotation, 0deg)) scale(1);
  }
  50% {
    opacity: 1;
    transform: rotate(var(--rotation, 0deg)) scale(1.1);
  }
}

.vortex-center {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 300px;
  height: 300px;
}

.spiral {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  background: 
    radial-gradient(circle, transparent 30%, rgba(255, 100, 0, 0.1) 40%, transparent 50%),
    radial-gradient(circle, transparent 50%, rgba(255, 150, 0, 0.1) 60%, transparent 70%),
    radial-gradient(circle, transparent 70%, rgba(255, 200, 0, 0.1) 80%, transparent 90%);
  animation: pulse 4s ease-in-out infinite, spin 20s linear infinite;
}

@keyframes pulse {
  0%, 100% {
    transform: scale(0.8);
    opacity: 0.5;
  }
  50% {
    transform: scale(1.2);
    opacity: 1;
  }
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.vortex-grid {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-image: 
    linear-gradient(rgba(255, 255, 255, 0.03) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255, 255, 255, 0.03) 1px, transparent 1px);
  background-size: 50px 50px;
  animation: gridMove 20s linear infinite;
}

@keyframes gridMove {
  0% {
    transform: perspective(500px) rotateX(60deg) translateY(0);
  }
  100% {
    transform: perspective(500px) rotateX(60deg) translateY(50px);
  }
}
</style>
