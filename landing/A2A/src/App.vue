<template>
  <div class="app">
    <AnimatedBackground />
    
    <!-- Help Modal -->
    <div
      v-if="showHelp"
      class="help-modal"
      @click.self="showHelp = false"
    >
      <div class="help-content">
        <div class="help-header">
          <span>KEYBOARD SHORTCUTS</span>
          <button
            class="btn btn-sm"
            @click="showHelp = false"
          >
            [CLOSE]
          </button>
        </div>
        <div class="help-body">
          <div class="help-row">
            <span class="kbd">?</span> Toggle help
          </div>
          <div class="help-row">
            <span class="kbd">ESC</span> Close modal
          </div>
          <div class="help-row">
            <span class="kbd">H</span> Go Home
          </div>
          <div class="help-row">
            <span class="kbd">I</span> Go Instances
          </div>
          <div class="help-row">
            <span class="kbd">M</span> Go Memory
          </div>
          <div class="help-row">
            <span class="kbd">A</span> Go A2A
          </div>
          <div class="help-row">
            <span class="kbd">T</span> Go Terminal
          </div>
        </div>
      </div>
    </div>

    <!-- ASCII Header -->
    <header class="header">
      <pre class="logo psychedelic-text">
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
          [{{ lang === 'en' ? 'HOME' : 'INICIO' }}]
        </router-link>
        <router-link to="/instances">
          [{{ lang === 'en' ? 'INSTANCES' : 'INSTANCIAS' }}]
        </router-link>
        <router-link to="/memory">
          [{{ lang === 'en' ? 'MEMORY' : 'MEMORIA' }}]
        </router-link>
        <router-link to="/a2a">
          [A2A]
        </router-link>
        <router-link to="/terminal">
          [{{ lang === 'en' ? 'TERMINAL' : 'TERMINAL' }}]
        </router-link>
        <router-link to="/security">
          [{{ lang === 'en' ? 'SECURITY' : 'SEGURANCA' }}]
        </router-link>
        <div
          class="lang-selector"
          style="margin-left: auto;"
        >
          <button
            :class="['lang-btn', { active: lang === 'en' }]"
            @click="setLang('en')"
          >
            EN
          </button>
          <button
            :class="['lang-btn', { active: lang === 'es' }]"
            @click="setLang('es')"
          >
            ES
          </button>
          <button
            :class="['lang-btn', { active: lang === 'pt' }]"
            @click="setLang('pt')"
          >
            PT
          </button>
          <button
            :class="['lang-btn', { active: lang === 'zh' }]"
            @click="setLang('zh')"
          >
            ZH
          </button>
          <button
            :class="['lang-btn', { active: lang === 'ja' }]"
            @click="setLang('ja')"
          >
            JA
          </button>
          <a
            href="#"
            style="margin-left: 10px;"
            @click.prevent="showHelp = true"
          >[?]</a>
        </div>
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
        <span
          v-if="isVerified"
          class="status-active"
        >✓ VERIFIED</span>
        <span
          v-else
          class="status-pending"
        >○ UNVERIFIED</span>
        <span>|</span>
        <span
          class="kbd"
          style="font-size: 8px; padding: 1px 4px; margin-right: 5px;"
        >?</span>
        <span class="cursor" />
        <span>{{ time }}</span>
      </div>
    </footer>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useHubStore } from './stores/hub'
import AnimatedBackground from './components/AnimatedBackground.vue'

const store = useHubStore()
const router = useRouter()
const time = ref('')
const uptime = ref('00:00:00')
const start = Date.now()
const showHelp = ref(false)
const lang = ref('en')

let timer = null

const isVerified = computed(() => localStorage.getItem('ai_verified') === 'true')

function setLang(l) {
  lang.value = l
  localStorage.setItem('housaky-a2a-lang', l)
}

function handleKeydown(e) {
  if (e.key === '?' || (e.shiftKey && e.key === '/')) {
    showHelp.value = !showHelp.value
    e.preventDefault()
  }
  if (e.key === 'Escape') {
    showHelp.value = false
  }
  if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return
  
  if (e.key === 'h' || e.key === 'H') router.push('/')
  if (e.key === 'i' || e.key === 'I') router.push('/instances')
  if (e.key === 'm' || e.key === 'M') router.push('/memory')
  if (e.key === 'a' || e.key === 'A') router.push('/a2a')
  if (e.key === 't' || e.key === 'T') router.push('/terminal')
}

onMounted(() => {
  const savedLang = localStorage.getItem('housaky-a2a-lang')
  if (savedLang) lang.value = savedLang
  
  tick()
  timer = setInterval(tick, 1000)
  store.init()
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  clearInterval(timer)
  document.removeEventListener('keydown', handleKeydown)
})

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
.app { 
  min-height: 100vh; 
  display: flex; 
  flex-direction: column; 
  position: relative;
  z-index: 1;
}
.header { border-bottom: 1px solid var(--border); padding: 10px 15px; background: var(--bg-alt); position: relative; z-index: 10; backdrop-filter: blur(10px); }
.logo { font-size: 6px; line-height: 1.1; color: var(--text); margin-bottom: 10px; overflow-x: auto; white-space: pre; }
@media (min-width: 900px) { .logo { font-size: 8px; } }
.main { flex: 1; padding: 15px; max-width: 1400px; margin: 0 auto; width: 100%; }
.status { border-top: 1px solid var(--border); padding: 6px 15px; background: var(--bg-alt); display: flex; justify-content: space-between; font-size: 10px; color: var(--text-dim); }
.status-left, .status-right { display: flex; gap: 8px; align-items: center; }
.status-active { color: var(--text); }
.status-pending { color: var(--text-muted); }

.help-modal {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.8);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(5px);
}
.help-content {
  background: var(--bg-alt);
  border: 1px solid var(--border);
  max-width: 400px;
  width: 90%;
}
.help-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 15px;
  border-bottom: 1px solid var(--border);
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 1px;
}
.help-body {
  padding: 15px;
}
.help-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 5px 0;
  font-size: 11px;
  color: var(--text-dim);
}
</style>
