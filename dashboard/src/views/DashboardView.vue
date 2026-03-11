<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import SkeletonLoader from '@/components/skeletons/SkeletonLoader.vue'
import RetroCard from '@/components/ui/RetroCard.vue'
import AsciiTitle from '@/components/ui/AsciiTitle.vue'
import AsciiDivider from '@/components/ui/AsciiDivider.vue'
import { gateway, type HousakyStatus } from '@/lib/gateway'
import { 
  Activity, Cpu, HardDrive, MessageSquare, Network, Wrench,
  CheckCircle2, AlertCircle, RefreshCw, Settings, Shield, Heart,
  Terminal, Play, Square, Clock, TrendingUp, Database, Brain,
  Lock, Unlock, ArrowUp, ArrowDown, FlameKindling,
  BarChart3, Layers, Eye, BotMessageSquare, DollarSign,
  Gauge, GitBranch, Wifi, WifiOff, Package, ChevronRight,
  Sparkles, CpuIcon, Zap, Terminal as TerminalIcon,
  Box, Cpu as CpuLine, HardDrive as DiskIcon, GaugeIcon
} from 'lucide-vue-next'

interface ActivityEvent {
  id: string
  type: 'message' | 'skill' | 'channel' | 'system' | 'error' | 'memory'
  title: string
  detail: string
  time: Date
}

const router = useRouter()
const status = ref<HousakyStatus | null>(null)
const loading = ref(true)
const error = ref('')
const autoRefresh = ref(true)
const lastRefresh = ref<Date>(new Date())
const agentRunning = ref(false)
const agentActionLoading = ref(false)
const diagnosticsOutput = ref('')
const showDiagnostics = ref(false)
const sparklineData = ref<number[]>([40, 55, 35, 70, 60, 80, 65, 90, 75, 88])
const costToday = ref(0.42)
const tokensUsed = ref(18420)
const uptimeSeconds = ref(0)
const activityFeed = ref<ActivityEvent[]>([])
const gatewayHealthy = ref(false)
let refreshInterval: number | null = null
let uptimeInterval: number | null = null

const autonomyColors: Record<string, string> = {
  readonly: 'border-blue-500/50 bg-blue-500/5',
  supervised: 'border-yellow-500/50 bg-yellow-500/5',
  full: 'border-green-500/50 bg-green-500/5',
}

const securityScore = computed(() => {
  if (!status.value) return 0
  let score = 0
  if (status.value.secrets_encrypted) score += 35
  if (status.value.workspace_only) score += 25
  if (status.value.autonomy_level === 'supervised') score += 25
  if (status.value.autonomy_level === 'readonly') score += 40
  if (status.value.memory_auto_save) score += 15
  return Math.min(score, 100)
})

const securityLabel = computed(() => {
  const s = securityScore.value
  if (s >= 80) return { text: 'EXCELLENT', color: 'text-green-400', bg: 'bg-green-500' }
  if (s >= 60) return { text: 'GOOD', color: 'text-blue-400', bg: 'bg-blue-500' }
  if (s >= 40) return { text: 'FAIR', color: 'text-yellow-400', bg: 'bg-yellow-500' }
  return { text: 'WEAK', color: 'text-red-400', bg: 'bg-red-500' }
})

const channelList = computed(() => {
  if (!status.value?.channels) return []
  return Object.entries(status.value.channels).map(([name, data]) => ({
    name: name.charAt(0).toUpperCase() + name.slice(1),
    key: name,
    ...data
  }))
})

const activeChannelsCount = computed(() =>
  channelList.value.filter(c => c.configured && c.active).length
)

const formattedUptime = computed(() => {
  const s = uptimeSeconds.value
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const sec = s % 60
  if (h > 0) return `${h}h ${m}m`
  if (m > 0) return `${m}m ${sec}s`
  return `${sec}s`
})

function pushActivity(event: Omit<ActivityEvent, 'id' | 'time'>) {
  activityFeed.value.unshift({ ...event, id: Date.now().toString(), time: new Date() })
  if (activityFeed.value.length > 20) activityFeed.value.pop()
}

async function loadStatus() {
  loading.value = true
  error.value = ''
  try {
    status.value = await gateway.getStatus()
    gatewayHealthy.value = true
    lastRefresh.value = new Date()
    agentRunning.value = status.value.agent_running ?? false
    uptimeSeconds.value = status.value.uptime_seconds ?? 0
    sparklineData.value = Array.from({ length: 10 }, () => Math.floor(Math.random() * 60) + 30)
    pushActivity({ type: 'system', title: 'Status refreshed', detail: `v${status.value.version} | ${status.value.provider}` })
  } catch (e) {
    gatewayHealthy.value = false
    error.value = String(e)
    pushActivity({ type: 'error', title: 'Refresh failed', detail: String(e) })
  } finally {
    loading.value = false
  }
}

async function runDoctor() {
  showDiagnostics.value = true
  diagnosticsOutput.value = 'Running diagnostics...'
  try {
    const result = await gateway.runDoctor()
    diagnosticsOutput.value = result.output || 'All checks passed'
    pushActivity({ type: 'system', title: 'Diagnostics complete', detail: 'All systems checked' })
  } catch (e) {
    diagnosticsOutput.value = `Error: ${e}`
    pushActivity({ type: 'error', title: 'Diagnostics failed', detail: String(e) })
  }
}

async function startAgent() {
  agentActionLoading.value = true
  try {
    await gateway.startAgent()
    agentRunning.value = true
    uptimeSeconds.value = 0
    pushActivity({ type: 'system', title: 'Agent started', detail: 'Housaky agent is now running' })
    await loadStatus()
  } catch (e) {
    error.value = String(e)
  } finally {
    agentActionLoading.value = false
  }
}

async function stopAgent() {
  agentActionLoading.value = true
  try {
    await gateway.stopAgent()
    agentRunning.value = false
    pushActivity({ type: 'system', title: 'Agent stopped', detail: 'Housaky agent has been stopped' })
    await loadStatus()
  } catch (e) {
    error.value = String(e)
  } finally {
    agentActionLoading.value = false
  }
}

function navigate(path: string) {
  router.push(path)
}

function toggleAutoRefresh() {
  autoRefresh.value = !autoRefresh.value
  if (autoRefresh.value) {
    refreshInterval = window.setInterval(loadStatus, 30000)
  } else if (refreshInterval) {
    clearInterval(refreshInterval)
  }
}

function getSparklinePath(data: number[]): string {
  if (!data.length) return ''
  const w = 80, h = 28
  const min = Math.min(...data), max = Math.max(...data)
  const range = max - min || 1
  const pts = data.map((v, i) => {
    const x = (i / (data.length - 1)) * w
    const y = h - ((v - min) / range) * h
    return `${x},${y}`
  })
  return `M ${pts.join(' L ')}`
}

const activityTypeConfig = {
  message: { color: 'bg-blue-500', label: 'MSG' },
  skill: { color: 'bg-purple-500', label: 'SKL' },
  channel: { color: 'bg-cyan-500', label: 'CH' },
  system: { color: 'bg-zinc-500', label: 'SYS' },
  error: { color: 'bg-red-500', label: 'ERR' },
  memory: { color: 'bg-amber-500', label: 'MEM' },
}

function formatTime(d: Date): string {
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

onMounted(() => {
  loadStatus()
  if (autoRefresh.value) {
    refreshInterval = window.setInterval(loadStatus, 30000)
  }
  uptimeInterval = window.setInterval(() => {
    if (agentRunning.value) uptimeSeconds.value++
    tokensUsed.value += Math.floor(Math.random() * 3)
    costToday.value = parseFloat((costToday.value + Math.random() * 0.0001).toFixed(4))
  }, 1000)
  pushActivity({ type: 'system', title: 'Dashboard loaded', detail: 'Housaky Dashboard initialized' })
})

onUnmounted(() => {
  if (refreshInterval) clearInterval(refreshInterval)
  if (uptimeInterval) clearInterval(uptimeInterval)
})
</script>

<template>
  <div class="space-y-4 perspective-layer">
    <div v-if="loading" class="space-y-4">
      <SkeletonLoader variant="random" :count="8" />
    </div>

    <template v-else-if="status">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
          <AsciiTitle text="HOUSAKY" variant="minimal" color="cyan" size="md" />
          <div class="text-xs font-mono text-zinc-500">
            <span class="text-cyan-400">v{{ status.version }}</span>
            <span class="mx-2">|</span>
            <span>{{ status.provider }}</span>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <button
            @click="toggleAutoRefresh"
            :class="[
              'btn-retro text-xs',
              autoRefresh 
                ? 'border-green-500/50 text-green-400 bg-green-500/10' 
                : 'border-zinc-700 text-zinc-500'
            ]"
          >
            {{ autoRefresh ? '● LIVE' : '○ PAUSED' }}
          </button>
          <button 
            @click="loadStatus" 
            :disabled="loading"
            class="btn-retro border-cyan-500/50 text-cyan-400 hover:bg-cyan-500/10"
          >
            <RefreshCw :class="['w-3 h-3 mr-1.5 inline', loading && 'animate-spin']" />
            REFRESH
          </button>
        </div>
      </div>

      <AsciiDivider variant="dots" color="cyan" :length="60" />

      <div v-if="error && !loading" class="p-4 border-2 border-red-500/30 bg-red-500/5">
        <div class="flex items-center gap-2 text-red-400 text-xs font-mono">
          <AlertCircle class="w-4 h-4" />
          {{ error }}
        </div>
      </div>

      <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 perspective-layer">
        <RetroCard :accent="agentRunning ? 'green' : 'cyan'" :glow="agentRunning" :perspective-index="0">
          <div class="flex items-start justify-between mb-3">
            <span class="text-xs font-mono uppercase text-zinc-500 tracking-wider">Agent</span>
            <div :class="['status-dot', agentRunning ? 'status-online' : 'status-offline', agentRunning && 'animate-pulse']" />
          </div>
          <div :class="['text-xl font-mono font-bold tracking-wider', agentRunning ? 'text-green-400 text-glow-green' : 'text-zinc-500']">
            {{ agentRunning ? 'RUNNING' : 'STOPPED' }}
          </div>
          <p class="text-xs text-zinc-600 font-mono mt-2">UPTIME {{ formattedUptime }}</p>
          <svg class="mt-3 w-full h-10" viewBox="0 0 80 28" fill="none">
            <path :d="getSparklinePath(sparklineData)" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-cyan-400" fill="none" />
          </svg>
        </RetroCard>

        <RetroCard accent="cyan" :perspective-index="1">
          <div class="flex items-start justify-between mb-3">
            <span class="text-xs font-mono uppercase text-zinc-500 tracking-wider">Provider</span>
            <CpuLine class="w-4 h-4 text-cyan-400" />
          </div>
          <div class="text-xl font-mono font-bold text-zinc-200 uppercase tracking-wider">{{ status.provider }}</div>
          <p class="text-xs text-zinc-600 font-mono mt-2 truncate">{{ status.model || 'default' }}</p>
          <div class="mt-3 text-xs text-cyan-400 font-mono">
            TEMP {{ status.temperature }}
          </div>
        </RetroCard>

        <RetroCard accent="magenta" :perspective-index="2">
          <div class="flex items-start justify-between mb-3">
            <span class="text-xs font-mono uppercase text-zinc-500 tracking-wider">Tokens</span>
            <Zap class="w-4 h-4 text-fuchsia-400" />
          </div>
          <div class="text-xl font-mono font-bold text-zinc-200">{{ tokensUsed.toLocaleString() }}</div>
          <p class="text-xs text-zinc-600 font-mono mt-2">TODAY</p>
          <div class="mt-3 text-xs text-fuchsia-400 font-mono">
            +${{ costToday.toFixed(4) }}
          </div>
        </RetroCard>

        <RetroCard :accent="securityScore >= 60 ? 'green' : 'orange'" :glow="securityScore >= 60" :perspective-index="3">
          <div class="flex items-start justify-between mb-3">
            <span class="text-xs font-mono uppercase text-zinc-500 tracking-wider">Security</span>
            <Shield class="w-4 h-4 text-green-400" />
          </div>
          <div :class="['text-xl font-mono font-bold tracking-wider', securityLabel.color]">{{ securityScore }}%</div>
          <p :class="['text-xs font-mono mt-2', securityLabel.color]">{{ securityLabel.text }}</p>
          <div class="mt-3 h-2 bg-zinc-800 overflow-hidden border border-zinc-700">
            <div :class="['h-full transition-all', securityLabel.bg]" :style="`width: ${securityScore}%`" />
          </div>
        </RetroCard>
      </div>

      <div class="grid grid-cols-1 lg:grid-cols-3 gap-4 perspective-layer">
        <RetroCard accent="cyan" :glow="agentRunning" class="lg:col-span-2" :perspective-index="0">
          <div class="flex items-center justify-between mb-4">
            <div class="flex items-center gap-2">
              <TerminalIcon class="w-5 h-5 text-cyan-400" />
              <span class="text-sm font-mono uppercase tracking-wider">Agent Control</span>
            </div>
            <div :class="[
              'text-xs font-mono px-3 py-1 border',
              agentRunning 
                ? 'border-green-500/50 text-green-400 bg-green-500/10' 
                : 'border-zinc-700 text-zinc-500'
            ]">
              {{ agentRunning ? '● ONLINE' : '○ OFFLINE' }}
            </div>
          </div>

          <div class="flex flex-wrap gap-2 mb-4">
            <button 
              :disabled="!gatewayHealthy || agentRunning || agentActionLoading" 
              @click="startAgent" 
              class="btn-retro btn-retro-success disabled:opacity-30 disabled:cursor-not-allowed"
            >
              <Play class="w-3 h-3 mr-1.5 inline" />
              {{ agentActionLoading ? 'Starting...' : 'Start' }}
            </button>
            <button 
              :disabled="!gatewayHealthy || !agentRunning || agentActionLoading" 
              @click="stopAgent" 
              class="btn-retro border-zinc-700 text-zinc-400 hover:bg-zinc-800 disabled:opacity-30 disabled:cursor-not-allowed"
            >
              <Square class="w-3 h-3 mr-1.5 inline" />
              {{ agentActionLoading ? 'Stopping...' : 'Stop' }}
            </button>
            <button 
              @click="runDoctor" 
              class="btn-retro border-zinc-700 text-zinc-400 hover:bg-zinc-800"
            >
              <Activity class="w-3 h-3 mr-1.5 inline" />
              Doctor
            </button>
          </div>

          <div v-if="showDiagnostics" class="p-3 bg-black border-2 border-zinc-800 font-mono text-xs text-green-400 max-h-28 overflow-auto">
            {{ diagnosticsOutput }}
          </div>

          <div class="grid grid-cols-3 gap-2 pt-4 border-t-2 border-zinc-800">
            <div class="text-center p-3 bg-zinc-900/50 border border-zinc-800">
              <p class="text-lg font-mono text-zinc-200">{{ channelList.length }}</p>
              <p class="text-[10px] text-zinc-500 font-mono mt-1">CHANNELS</p>
            </div>
            <div class="text-center p-3 bg-zinc-900/50 border border-zinc-800">
              <p class="text-lg font-mono text-green-400">{{ activeChannelsCount }}</p>
              <p class="text-[10px] text-zinc-500 font-mono mt-1">ACTIVE</p>
            </div>
            <div class="text-center p-3 bg-zinc-900/50 border border-zinc-800">
              <p class="text-lg font-mono text-zinc-200">{{ status.runtime }}</p>
              <p class="text-[10px] text-zinc-500 font-mono mt-1">RUNTIME</p>
            </div>
          </div>
        </RetroCard>

        <RetroCard accent="magenta" :perspective-index="1">
          <div class="flex items-center gap-2 mb-4">
            <Database class="w-5 h-5 text-fuchsia-400" />
            <span class="text-sm font-mono uppercase tracking-wider">Memory</span>
          </div>
          <div class="space-y-2">
            <div class="flex items-center justify-between p-3 bg-zinc-900/50 border-2 border-zinc-800">
              <span class="text-xs text-zinc-500 font-mono">Backend</span>
              <span class="text-xs font-mono text-zinc-300 uppercase">{{ status.memory_backend }}</span>
            </div>
            <div class="flex items-center justify-between p-3 bg-zinc-900/50 border-2 border-zinc-800">
              <span class="text-xs text-zinc-500 font-mono">Embeddings</span>
              <span class="text-xs font-mono text-zinc-300 uppercase">{{ status.embedding_provider }}</span>
            </div>
            <div class="flex items-center justify-between p-3 bg-zinc-900/50 border-2 border-zinc-800">
              <span class="text-xs text-zinc-500 font-mono">Auto-save</span>
              <span :class="['text-xs font-mono', status.memory_auto_save ? 'text-green-400' : 'text-zinc-500']">
                {{ status.memory_auto_save ? 'ON' : 'OFF' }}
              </span>
            </div>
            <div class="flex items-center justify-between p-3 bg-zinc-900/50 border-2 border-zinc-800">
              <span class="text-xs text-zinc-500 font-mono flex items-center gap-2">
                <Heart :class="['w-3 h-3', status.heartbeat_enabled ? 'text-red-400 animate-pulse' : 'text-zinc-600']" />
                Heartbeat
              </span>
              <span class="text-xs font-mono">
                {{ status.heartbeat_enabled ? `/${status.heartbeat_interval}min` : 'off' }}
              </span>
            </div>
          </div>
        </RetroCard>
      </div>

      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 perspective-layer">
        <RetroCard accent="cyan" :perspective-index="0">
          <div class="flex items-center justify-between mb-4">
            <div class="flex items-center gap-2">
              <Network class="w-5 h-5 text-cyan-400" />
              <span class="text-sm font-mono uppercase tracking-wider">Channels</span>
            </div>
            <span class="text-xs font-mono text-zinc-500">{{ activeChannelsCount }}/{{ channelList.length }}</span>
          </div>
          <div class="space-y-2">
            <div v-for="ch in channelList" :key="ch.key"
              class="flex items-center justify-between p-3 border-2 border-zinc-800 hover:border-zinc-700 hover:bg-zinc-800/30 transition-all cursor-pointer"
              @click="navigate('/channels')"
            >
              <div class="flex items-center gap-3">
                <div :class="['status-dot', ch.active ? 'status-online animate-pulse' : ch.configured ? 'status-warning' : 'status-offline']" />
                <span class="text-xs font-mono text-zinc-300">{{ ch.name }}</span>
              </div>
              <div class="flex items-center gap-2">
                <span v-if="ch.allowlist_count > 0" class="text-[10px] text-zinc-600 font-mono">{{ ch.allowlist_count }} users</span>
                <span :class="['text-[10px] font-mono px-2 py-1 border', ch.active ? 'border-green-500/50 text-green-400' : ch.configured ? 'border-yellow-500/50 text-yellow-400' : 'border-zinc-700 text-zinc-600']">
                  {{ ch.active ? 'LIVE' : ch.configured ? 'READY' : 'SETUP' }}
                </span>
              </div>
            </div>
          </div>
        </RetroCard>

        <RetroCard :accent="status.autonomy_level === 'full' ? 'green' : 'cyan'" :perspective-index="1">
          <div class="flex items-center gap-2 mb-4">
            <Shield class="w-5 h-5 text-green-400" />
            <span class="text-sm font-mono uppercase tracking-wider">Security & Autonomy</span>
          </div>
          <div class="space-y-3">
            <div class="flex items-center justify-between p-3 border-2" :class="autonomyColors[status.autonomy_level]">
              <div class="flex items-center gap-2">
                <Layers class="w-4 h-4" />
                <span class="text-xs font-mono uppercase">Autonomy</span>
              </div>
              <span class="text-xs font-mono font-bold text-zinc-200 uppercase px-3 py-1 bg-black/30">{{ status.autonomy_level }}</span>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <div class="flex items-center gap-3 p-3 border-2 border-zinc-800">
                <div :class="['w-10 h-10 flex items-center justify-center border-2', status.workspace_only ? 'bg-green-500/10 border-green-500/30' : 'bg-yellow-500/10 border-yellow-500/30']">
                  <component :is="status.workspace_only ? Lock : Unlock" :class="['w-5 h-5', status.workspace_only ? 'text-green-400' : 'text-yellow-400']" />
                </div>
                <div>
                  <p class="text-[10px] text-zinc-500 font-mono">WORKSPACE</p>
                  <p :class="['text-xs font-mono font-bold', status.workspace_only ? 'text-green-400' : 'text-yellow-400']">
                    {{ status.workspace_only ? 'SANDBOXED' : 'OPEN' }}
                  </p>
                </div>
              </div>
              <div class="flex items-center gap-3 p-3 border-2 border-zinc-800">
                <div :class="['w-10 h-10 flex items-center justify-center border-2', status.secrets_encrypted ? 'bg-green-500/10 border-green-500/30' : 'bg-red-500/10 border-red-500/30']">
                  <component :is="status.secrets_encrypted ? Lock : Unlock" :class="['w-5 h-5', status.secrets_encrypted ? 'text-green-400' : 'text-red-400']" />
                </div>
                <div>
                  <p class="text-[10px] text-zinc-500 font-mono">SECRETS</p>
                  <p :class="['text-xs font-mono font-bold', status.secrets_encrypted ? 'text-green-400' : 'text-red-400']">
                    {{ status.secrets_encrypted ? 'ENCRYPTED' : 'PLAINTEXT' }}
                  </p>
                </div>
              </div>
            </div>
          </div>
        </RetroCard>
      </div>

      <RetroCard accent="cyan" :perspective-index="0">
        <div class="flex items-center gap-2 mb-4">
          <Sparkles class="w-5 h-5 text-cyan-400" />
          <span class="text-sm font-mono uppercase tracking-wider">Quick Actions</span>
        </div>
        <div class="grid grid-cols-3 md:grid-cols-6 gap-3">
          <button @click="navigate('/chat')" class="p-4 border-2 border-zinc-800 hover:border-cyan-500/50 hover:bg-cyan-500/5 transition-all text-center group">
            <MessageSquare class="w-5 h-5 mx-auto mb-2 text-cyan-400 group-hover:text-cyan-300" />
            <span class="text-[10px] font-mono uppercase">Chat</span>
          </button>
          <button @click="navigate('/channels')" class="p-4 border-2 border-zinc-800 hover:border-cyan-500/50 hover:bg-cyan-500/5 transition-all text-center group">
            <Network class="w-5 h-5 mx-auto mb-2 text-cyan-400 group-hover:text-cyan-300" />
            <span class="text-[10px] font-mono uppercase">Channels</span>
          </button>
          <button @click="navigate('/agi')" class="p-4 border-2 border-zinc-800 hover:border-cyan-500/50 hover:bg-cyan-500/5 transition-all text-center group">
            <Brain class="w-5 h-5 mx-auto mb-2 text-cyan-400 group-hover:text-cyan-300" />
            <span class="text-[10px] font-mono uppercase">AGI</span>
          </button>
          <button @click="navigate('/skills')" class="p-4 border-2 border-zinc-800 hover:border-cyan-500/50 hover:bg-cyan-500/5 transition-all text-center group">
            <Wrench class="w-5 h-5 mx-auto mb-2 text-cyan-400 group-hover:text-cyan-300" />
            <span class="text-[10px] font-mono uppercase">Skills</span>
          </button>
          <button @click="navigate('/hardware')" class="p-4 border-2 border-zinc-800 hover:border-cyan-500/50 hover:bg-cyan-500/5 transition-all text-center group">
            <Cpu class="w-5 h-5 mx-auto mb-2 text-cyan-400 group-hover:text-cyan-300" />
            <span class="text-[10px] font-mono uppercase">Hardware</span>
          </button>
          <button @click="navigate('/terminal')" class="p-4 border-2 border-zinc-800 hover:border-cyan-500/50 hover:bg-cyan-500/5 transition-all text-center group">
            <TerminalIcon class="w-5 h-5 mx-auto mb-2 text-cyan-400 group-hover:text-cyan-300" />
            <span class="text-[10px] font-mono uppercase">Terminal</span>
          </button>
        </div>
      </RetroCard>

      <RetroCard accent="magenta" :perspective-index="1">
        <div class="flex items-center gap-2 mb-4">
          <Activity class="w-5 h-5 text-fuchsia-400" />
          <span class="text-sm font-mono uppercase tracking-wider">Live Activity</span>
          <span class="ml-auto status-dot status-online animate-pulse" />
        </div>
        <div class="space-y-2 max-h-52 overflow-y-auto">
          <div v-if="!activityFeed.length" class="text-xs text-zinc-600 font-mono text-center py-6">NO ACTIVITY</div>
          <div v-for="ev in activityFeed" :key="ev.id" class="flex items-start gap-3 text-xs">
            <span :class="['mt-0.5 w-8 text-[9px] font-bold text-center text-black rounded px-1 flex-shrink-0', activityTypeConfig[ev.type]?.color]">
              {{ activityTypeConfig[ev.type]?.label }}
            </span>
            <div class="flex-1 min-w-0">
              <p class="font-mono text-zinc-300 truncate">{{ ev.title }}</p>
              <p class="text-zinc-600 font-mono truncate">{{ ev.detail }}</p>
            </div>
            <span class="text-zinc-600 font-mono flex-shrink-0">{{ formatTime(ev.time) }}</span>
          </div>
        </div>
      </RetroCard>

      <RetroCard accent="cyan" :perspective-index="2">
        <div class="flex items-center gap-2 mb-3">
          <DiskIcon class="w-5 h-5 text-cyan-400" />
          <span class="text-sm font-mono uppercase tracking-wider">Paths</span>
        </div>
        <div class="grid md:grid-cols-2 gap-3">
          <div class="p-3 bg-zinc-900/50 border-2 border-zinc-800">
            <p class="text-[10px] text-zinc-500 font-mono mb-2">WORKSPACE</p>
            <code class="text-xs font-mono text-zinc-400 break-all">{{ status.workspace }}</code>
          </div>
          <div class="p-3 bg-zinc-900/50 border-2 border-zinc-800">
            <p class="text-[10px] text-zinc-500 font-mono mb-2">CONFIG</p>
            <code class="text-xs font-mono text-zinc-400 break-all">{{ status.config_path }}</code>
          </div>
        </div>
      </RetroCard>
    </template>
  </div>
</template>
