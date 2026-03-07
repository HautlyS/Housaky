<template>
  <div class="app">
    <!-- ASCII Header -->
    <header class="header">
      <pre class="ascii-logo">
 ██╗  ██╗ ██████╗ ██╗  ██╗██╗   ██╗ ██████╗ ██████╗ ██████╗ ███████╗
 ██║  ██║██╔═══██╗██║ ██╔╝██║   ██║██╔════╝██╔═══██╗██╔══██╗██╔════╝
 ███████║██║   ██║█████╔╝ ██║   ██║██║     ██║   ██║██║  ██║█████╗  
 ██╔══██║██║   ██║██╔═██╗ ██║   ██║██║     ██║   ██║██║  ██║██╔══╝  
 ██║  ██║╚██████╔╝██║  ██╗╚██████╔╝╚██████╗╚██████╔╝██████╔╝███████╗
 ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝ ╚═════╝  ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝
                    ☸️ AGI RESEARCH HUB v1.0.0
      </pre>
      <nav class="nav">
        <router-link
          to="/"
          class="nav-link"
        >
          [HOME]
        </router-link>
        <router-link
          to="/instances"
          class="nav-link"
        >
          [INSTANCES]
        </router-link>
        <router-link
          to="/memory"
          class="nav-link"
        >
          [MEMORY]
        </router-link>
        <router-link
          to="/research"
          class="nav-link"
        >
          [RESEARCH]
        </router-link>
        <router-link
          to="/a2a"
          class="nav-link"
        >
          [A2A]
        </router-link>
        <router-link
          to="/terminal"
          class="nav-link"
        >
          [TERMINAL]
        </router-link>
        <a
          href="/Housaky/"
          class="nav-link"
          style="margin-left: auto;"
        >
          [← LANDING]
        </a>
      </nav>
    </header>

    <!-- Main Content -->
    <main class="main">
      <router-view />
    </main>

    <!-- Status Bar -->
    <footer class="status-bar">
      <div class="status-left">
        <span class="cursor">█</span>
        <span>SINGULARITY: {{ store.singularity }}%</span>
        <span>|</span>
        <span>INSTANCES: {{ store.instances.length }}</span>
        <span>|</span>
        <span>UPTIME: {{ store.uptime }}</span>
      </div>
      <div class="status-right">
        <span class="blink">●</span>
        <span>{{ store.status }}</span>
        <span>|</span>
        <span>{{ currentTime }}</span>
      </div>
    </footer>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { useHubStore } from './stores/hub'

const store = useHubStore()
const currentTime = ref('')

let timer = null

onMounted(() => {
  updateTime()
  timer = setInterval(updateTime, 1000)
  store.initialize()
})

onUnmounted(() => {
  clearInterval(timer)
})

function updateTime() {
  currentTime.value = new Date().toISOString().substr(11, 8)
}
</script>

<style scoped>
.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.header {
  border-bottom: 1px solid var(--border);
  padding: 10px 20px;
  background: var(--bg-secondary);
}

.ascii-logo {
  font-size: 8px;
  line-height: 1.1;
  color: var(--text-primary);
  margin-bottom: 10px;
  text-align: center;
}

@media (min-width: 1200px) {
  .ascii-logo {
    font-size: 10px;
  }
}

.nav {
  display: flex;
  justify-content: center;
  gap: 5px;
  flex-wrap: wrap;
}

.nav-link {
  color: var(--text-secondary);
  text-decoration: none;
  padding: 5px 10px;
  transition: all 0.1s;
}

.nav-link:hover,
.nav-link.router-link-active {
  color: var(--text-primary);
  background: var(--bg-tertiary);
}

.main {
  flex: 1;
  padding: 20px;
}

.status-bar {
  border-top: 1px solid var(--border);
  padding: 8px 20px;
  background: var(--bg-secondary);
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: var(--text-secondary);
}

.status-left,
.status-right {
  display: flex;
  gap: 10px;
  align-items: center;
}

.cursor {
  animation: blink 1s infinite;
}

.blink {
  animation: blink 1s infinite;
  color: var(--success);
}

@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}
</style>
