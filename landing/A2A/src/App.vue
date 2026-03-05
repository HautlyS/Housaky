<template>
  <div class="app">
    <!-- ASCII Header -->
    <header class="header">
      <pre class="logo">
 ██╗  ██╗ ██████╗ ██╗  ██╗██╗   ██╗ ██████╗ ██████╗ ██████╗ ███████╗
 ██║  ██║██╔═══██╗██║ ██╔╝██║   ██║██╔════╝██╔═══██╗██╔══██╗██╔════╝
 ███████║██║   ██║█████╔╝ ██║   ██║██║     ██║   ██║██║  ██║█████╗
 ██╔══██║██║   ██║██╔═██╗ ██║   ██║██║     ██║   ██║██║  ██║██╔══╝
 ██║  ██║╚██████╔╝██║  ██╗╚██████╔╝╚██████╗╚██████╔╝██████╔╝███████╗
 ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝ ╚═════╝  ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝
                    ☸️ A2A HUB v1.0.0
      </pre>
      <nav class="nav">
        <router-link to="/">
          [HOME]
        </router-link>
        <router-link to="/instances">
          [INSTANCES]
        </router-link>
        <router-link to="/memory">
          [MEMORY]
        </router-link>
        <router-link to="/a2a">
          [A2A]
        </router-link>
        <router-link to="/terminal">
          [TERMINAL]
        </router-link>
      </nav>
    </header>

    <!-- Main -->
    <main class="main">
      <router-view />
    </main>

    <!-- Status Bar -->
    <footer class="status">
      <div class="status-left">
        <span class="blink">●</span>
        <span>SINGULARITY: {{ store.singularity }}%</span>
        <span>|</span>
        <span>INSTANCES: {{ store.instances.length }}</span>
        <span>|</span>
        <span>UPTIME: {{ uptime }}</span>
      </div>
      <div class="status-right">
        <span class="cursor" />
        <span>{{ time }}</span>
      </div>
    </footer>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useHubStore } from './stores/hub'

const store = useHubStore()
const time = ref('')
const uptime = ref('00:00:00')
const start = Date.now()
let timer = null

onMounted(() => {
  tick()
  timer = setInterval(tick, 1000)
  store.init()
})

onUnmounted(() => clearInterval(timer))

function tick() {
  time.value = new Date().toISOString().substr(11, 8)
  const s = Math.floor((Date.now() - start) / 1000)
  const h = String(Math.floor(s / 3600)).padStart(2, '0')
  const m = String(Math.floor((s % 3600) / 60)).padStart(2, '0')
  const sec = String(s % 60).padStart(2, '0')
  uptime.value = `${h}:${m}:${sec}`
}
</script>

<style scoped>
.app { min-height: 100vh; display: flex; flex-direction: column; }
.header { border-bottom: 1px solid var(--border); padding: 10px 15px; background: var(--bg-alt); }
.logo { font-size: 7px; line-height: 1.1; color: var(--text); margin-bottom: 10px; overflow-x: auto; white-space: pre; }
@media (min-width: 1000px) { .logo { font-size: 8px; } }
.main { flex: 1; padding: 15px; max-width: 1400px; margin: 0 auto; width: 100%; }
.status { border-top: 1px solid var(--border); padding: 6px 15px; background: var(--bg-alt); display: flex; justify-content: space-between; font-size: 10px; color: var(--text-dim); }
.status-left, .status-right { display: flex; gap: 8px; align-items: center; }
</style>
