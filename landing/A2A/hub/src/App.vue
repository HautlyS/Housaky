<template>
  <div class="app">
    <!-- Header -->
    <header class="header">
      <div class="header-top">
        <div class="logo-section">
          <pre class="ascii-logo-small">☸️ HOUSAKY</pre>
          <span class="version">v4.0.0-AGI</span>
        </div>
        <div class="header-stats">
          <div class="stat-mini">
            <span class="stat-label">SINGULARITY</span>
            <span class="stat-value">{{ store.singularity }}%</span>
          </div>
          <div class="stat-mini">
            <span class="stat-label">INSTANCES</span>
            <span class="stat-value">{{ store.activeInstances }}</span>
          </div>
          <div class="stat-mini">
            <span class="stat-label">UPTIME</span>
            <span class="stat-value">{{ store.uptime }}</span>
          </div>
        </div>
        <div class="header-time">
          <span class="time">{{ currentTime }}</span>
          <span class="status-indicator" :class="{ active: store.status === 'ACTIVE' }">
            ● {{ store.status }}
          </span>
        </div>
      </div>
      
      <nav class="nav">
        <router-link to="/" class="nav-link">
          <span class="nav-icon">⌂</span>
          <span class="nav-text">HOME</span>
        </router-link>
        <router-link to="/instances" class="nav-link">
          <span class="nav-icon">◈</span>
          <span class="nav-text">INSTANCES</span>
        </router-link>
        <router-link to="/memory" class="nav-link">
          <span class="nav-icon">◆</span>
          <span class="nav-text">MEMORY</span>
        </router-link>
        <router-link to="/a2a" class="nav-link">
          <span class="nav-icon">◉</span>
          <span class="nav-text">A2A</span>
        </router-link>
        <router-link to="/nodes" class="nav-link">
          <span class="nav-icon">⬡</span>
          <span class="nav-text">NODES</span>
        </router-link>
        <router-link to="/security" class="nav-link">
          <span class="nav-icon">⚿</span>
          <span class="nav-text">SECURITY</span>
        </router-link>
        <router-link to="/terminal" class="nav-link">
          <span class="nav-icon">▶</span>
          <span class="nav-text">TERMINAL</span>
        </router-link>
        <a href="/Housaky/" class="nav-link external">
          <span class="nav-icon">↗</span>
          <span class="nav-text">LANDING</span>
        </a>
      </nav>
    </header>

    <!-- Main Content -->
    <main class="main">
      <router-view v-slot="{ Component }">
        <transition name="fade" mode="out-in">
          <component :is="Component" />
        </transition>
      </router-view>
    </main>

    <!-- Footer Status Bar -->
    <footer class="footer">
      <div class="footer-left">
        <span class="cursor">▌</span>
        <span>AGI RESEARCH HUB</span>
        <span class="divider">│</span>
        <span>PHASE: {{ currentPhase }}</span>
      </div>
      <div class="footer-center">
        <div class="progress-mini">
          <span class="progress-bar-mini" :style="{ width: store.singularity + '%' }"></span>
        </div>
        <span class="progress-label">{{ store.singularity }}% → 60%</span>
      </div>
      <div class="footer-right">
        <span class="encryption">🔒 E2E ENCRYPTED</span>
        <span class="divider">│</span>
        <span>QUIC ACTIVE</span>
      </div>
    </footer>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useHubStore } from './stores/hub'

const store = useHubStore()
const currentTime = ref('')

let timer = null

const currentPhase = computed(() => {
  const progress = store.singularity
  if (progress < 30) return 'LINEAR'
  if (progress < 60) return 'SUPERLINEAR'
  if (progress < 90) return 'EXPONENTIAL'
  return 'PRE-SINGULARITY'
})

onMounted(() => {
  updateTime()
  timer = setInterval(updateTime, 1000)
  store.initialize()
})

onUnmounted(() => {
  clearInterval(timer)
})

function updateTime() {
  currentTime.value = new Date().toISOString().substr(11, 8) + ' UTC'
}
</script>

<style scoped>
.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
}

/* Header */
.header {
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  position: sticky;
  top: 0;
  z-index: 100;
}

.header-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 20px;
  border-bottom: 1px solid var(--border);
}

.logo-section {
  display: flex;
  align-items: baseline;
  gap: 10px;
}

.ascii-logo-small {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0;
}

.version {
  font-size: 10px;
  color: var(--text-muted);
  padding: 2px 6px;
  border: 1px solid var(--border);
  border-radius: 2px;
}

.header-stats {
  display: flex;
  gap: 24px;
}

.stat-mini {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.stat-label {
  font-size: 9px;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 1px;
}

.stat-value {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.header-time {
  display: flex;
  align-items: center;
  gap: 12px;
}

.time {
  font-size: 12px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.status-indicator {
  font-size: 11px;
  color: var(--text-muted);
}

.status-indicator.active {
  color: var(--success);
}

/* Navigation */
.nav {
  display: flex;
  justify-content: center;
  gap: 2px;
  padding: 8px 20px;
  overflow-x: auto;
}

.nav-link {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  color: var(--text-muted);
  text-decoration: none;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 1px;
  border-radius: 2px;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.nav-icon {
  font-size: 12px;
}

.nav-link:hover {
  color: var(--text-primary);
  background: var(--bg-tertiary);
}

.nav-link.router-link-active {
  color: var(--text-primary);
  background: var(--bg-tertiary);
  border-bottom: 2px solid var(--success);
}

.nav-link.external {
  margin-left: auto;
  color: var(--text-muted);
}

/* Main Content */
.main {
  flex: 1;
  padding: 20px;
  max-width: 1600px;
  margin: 0 auto;
  width: 100%;
}

/* Footer */
.footer {
  background: var(--bg-secondary);
  border-top: 1px solid var(--border);
  padding: 10px 20px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--text-muted);
}

.footer-left, .footer-center, .footer-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.cursor {
  animation: blink 1s infinite;
  color: var(--success);
}

.divider {
  color: var(--border-light);
}

.progress-mini {
  width: 100px;
  height: 4px;
  background: var(--bg-tertiary);
  border-radius: 2px;
  overflow: hidden;
}

.progress-bar-mini {
  height: 100%;
  background: var(--success);
  transition: width 0.3s ease;
}

.progress-label {
  font-size: 10px;
}

.encryption {
  color: var(--success);
}

/* Fade transition */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}

/* Responsive */
@media (max-width: 768px) {
  .header-stats {
    display: none;
  }
  
  .nav-link .nav-text {
    display: none;
  }
  
  .nav-link {
    padding: 10px;
  }
  
  .footer-center {
    display: none;
  }
}
</style>
// Force new hash - 1773109617700739123
