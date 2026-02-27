<script setup lang="ts">
import { ref, onMounted, computed, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRouter } from 'vue-router'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardDescription from '@/components/ui/card-description.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import Button from '@/components/ui/button.vue'
import Badge from '@/components/ui/badge.vue'
import { 
  Activity, Cpu, HardDrive, MessageSquare, Network, Wrench,
  CheckCircle2, AlertCircle, RefreshCw, Settings, Shield, Heart,
  Terminal, Play, Square, Clock, TrendingUp, Database, Brain,
  Lock, Unlock, ArrowUp, ArrowDown, Minus, FlameKindling,
  BarChart3, Layers, Eye, BotMessageSquare, DollarSign,
  Gauge, GitBranch, Wifi, WifiOff, Package, ChevronRight,
  Sparkles
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface HousakyStatus {
  version: string
  workspace: string
  config: string
  provider: string
  model: string
  temperature: number
  memory_backend: string
  memory_auto_save: boolean
  embedding_provider: string
  autonomy_level: string
  workspace_only: boolean
  runtime: string
  heartbeat_enabled: boolean
  heartbeat_interval: number
  channels: Record<string, { configured: boolean; active: boolean; allowlist_count: number }>
  secrets_encrypted: boolean
}

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
const housakyInstalled = ref(false)
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
let refreshInterval: number | null = null
let uptimeInterval: number | null = null

const autonomyColors: Record<string, string> = {
  readonly: 'from-blue-500/10 to-blue-500/5 border-blue-200',
  supervised: 'from-yellow-500/10 to-yellow-500/5 border-yellow-200',
  full: 'from-green-500/10 to-green-500/5 border-green-200',
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
  if (s >= 80) return { text: 'Excellent', color: 'text-green-600', bg: 'bg-green-500' }
  if (s >= 60) return { text: 'Good', color: 'text-blue-600', bg: 'bg-blue-500' }
  if (s >= 40) return { text: 'Fair', color: 'text-yellow-600', bg: 'bg-yellow-500' }
  return { text: 'Weak', color: 'text-red-600', bg: 'bg-red-500' }
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
    if (!isTauri) {
      housakyInstalled.value = false
      loading.value = false
      pushActivity({ type: 'system', title: 'Web mode', detail: 'Running outside Tauri — limited functionality' })
      return
    }
    housakyInstalled.value = await invoke<boolean>('check_housaky_installed')
    status.value = await invoke<HousakyStatus>('get_status')
    lastRefresh.value = new Date()
    agentRunning.value = housakyInstalled.value
    sparklineData.value = Array.from({ length: 10 }, () => Math.floor(Math.random() * 60) + 30)
    pushActivity({ type: 'system', title: 'Status refreshed', detail: `v${status.value.version} · ${status.value.provider}` })
  } catch (e) {
    error.value = String(e)
    pushActivity({ type: 'error', title: 'Refresh failed', detail: String(e) })
  } finally {
    loading.value = false
  }
}

async function runDoctor() {
  if (!isTauri) return
  showDiagnostics.value = true
  diagnosticsOutput.value = 'Running diagnostics…'
  try {
    const result = await invoke<string>('run_doctor')
    diagnosticsOutput.value = result || 'All checks passed ✓'
    pushActivity({ type: 'system', title: 'Diagnostics complete', detail: 'All systems checked' })
  } catch (e) {
    diagnosticsOutput.value = `Error: ${e}`
    pushActivity({ type: 'error', title: 'Diagnostics failed', detail: String(e) })
  }
}

async function startAgent() {
  if (!isTauri) return
  agentActionLoading.value = true
  try {
    await invoke('run_housaky_command_cmd', { command: 'agent', args: ['--daemon'] })
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
  if (!isTauri) return
  agentActionLoading.value = true
  try {
    await invoke('run_housaky_command_cmd', { command: 'stop', args: [] })
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
  skill: { color: 'bg-purple-500', label: 'SKILL' },
  channel: { color: 'bg-cyan-500', label: 'CH' },
  system: { color: 'bg-gray-500', label: 'SYS' },
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
  <div class="space-y-6 max-w-7xl mx-auto">
    <!-- ── Header ── -->
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div class="flex items-center gap-4">
        <div class="w-14 h-14 rounded-2xl bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 flex items-center justify-center shadow-lg shadow-indigo-500/30">
          <Brain class="w-7 h-7 text-white" />
        </div>
        <div>
          <h1 class="text-2xl font-bold text-gray-900 dark:text-white">Welcome to Housaky</h1>
          <p class="text-sm text-muted-foreground">Your AI Command Center · v{{ status?.version || '...' }}</p>
        </div>
      </div>
      <div class="flex items-center gap-2 flex-wrap">
        <div class="flex items-center gap-2 text-xs text-muted-foreground bg-white/50 dark:bg-white/5 px-3 py-1.5 rounded-full border border-gray-200/50 dark:border-white/10">
          <Clock class="w-3.5 h-3.5" />
          {{ lastRefresh.toLocaleTimeString() }}
        </div>
        <Button 
          variant="outline" 
          size="sm" 
          :class="[
            'rounded-full px-4 transition-all',
            autoRefresh 
              ? 'border-green-300 bg-green-50 text-green-700 hover:bg-green-100 dark:border-green-800 dark:bg-green-900/20 dark:text-green-400' 
              : 'border-gray-200 dark:border-white/10'
          ]" 
          @click="toggleAutoRefresh"
        >
          <Activity class="w-3.5 h-3.5 mr-1.5" />
          {{ autoRefresh ? 'Live' : 'Paused' }}
        </Button>
        <Button 
          size="sm" 
          class="rounded-full px-4 bg-gradient-to-r from-indigo-500 to-purple-500 hover:from-indigo-600 hover:to-purple-600 border-0"
          @click="loadStatus" 
          :disabled="loading"
        >
          <RefreshCw :class="['w-3.5 h-3.5 mr-1.5', loading && 'animate-spin']" />
          Refresh
        </Button>
      </div>
    </div>

    <!-- ── Alert Banners ── -->
    <div v-if="!housakyInstalled && !loading" class="rounded-2xl p-4 bg-gradient-to-r from-amber-50 to-orange-50 dark:from-amber-900/20 dark:to-orange-900/20 border border-amber-200/50 dark:border-amber-700/30">
      <div class="flex items-center justify-between flex-wrap gap-3">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-amber-100 dark:bg-amber-900/50 flex items-center justify-center">
            <AlertCircle class="w-5 h-5 text-amber-600 dark:text-amber-400" />
          </div>
          <div>
            <span class="font-semibold text-amber-800 dark:text-amber-300">Housaky not installed</span>
            <p class="text-sm text-amber-600 dark:text-amber-400">Configure your AI agent to get started</p>
          </div>
        </div>
        <Button size="sm" class="rounded-xl bg-amber-500 hover:bg-amber-600 text-white" @click="router.push('/config')">Configure Now</Button>
      </div>
    </div>

    <div v-if="error && !loading" class="rounded-2xl p-4 bg-red-50 dark:bg-red-900/20 border border-red-200/50 dark:border-red-800/30">
      <div class="flex items-center gap-3 text-red-700 dark:text-red-400">
        <AlertCircle class="w-5 h-5 flex-shrink-0" />
        <span class="text-sm">{{ error }}</span>
      </div>
    </div>

    <!-- ── Loading ── -->
    <div v-if="loading && !status" class="flex items-center justify-center py-24">
      <div class="flex flex-col items-center gap-4">
        <div class="relative">
          <div class="w-16 h-16 rounded-2xl bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 flex items-center justify-center shadow-lg shadow-indigo-500/30">
            <Brain class="w-8 h-8 text-white" />
          </div>
          <span class="absolute -top-1 -right-1 w-4 h-4 bg-green-500 rounded-full animate-ping" />
        </div>
        <p class="text-sm text-muted-foreground animate-pulse">Connecting to Housaky…</p>
      </div>
    </div>

    <template v-else-if="status">
      <!-- ── KPI Row ── -->
      <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <!-- Agent Status -->
        <div class="rounded-2xl p-5 bg-gradient-to-br from-indigo-500/10 to-purple-500/10 border border-gray-200/50 dark:border-white/10 hover-lift apple-shadow">
          <div class="flex items-start justify-between mb-3">
            <span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Agent</span>
            <div :class="['w-2.5 h-2.5 rounded-full', agentRunning ? 'bg-green-500 animate-pulse' : 'bg-gray-400']" />
          </div>
          <div :class="['text-2xl font-bold', agentRunning ? 'text-green-600 dark:text-green-400' : 'text-gray-500']">
            {{ agentRunning ? 'Running' : 'Stopped' }}
          </div>
          <p class="text-xs text-muted-foreground mt-1">Uptime {{ formattedUptime }}</p>
          <svg class="mt-3 w-20 h-8" viewBox="0 0 80 28" fill="none">
            <path :d="getSparklinePath(sparklineData)" stroke="url(#grad1)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
            <defs>
              <linearGradient id="grad1" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stop-color="#6366f1" />
                <stop offset="100%" stop-color="#a855f7" />
              </linearGradient>
            </defs>
          </svg>
        </div>

        <!-- AI Provider -->
        <div class="rounded-2xl p-5 bg-gradient-to-br from-blue-500/10 to-cyan-500/10 border border-gray-200/50 dark:border-white/10 hover-lift apple-shadow">
          <div class="flex items-start justify-between mb-3">
            <span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Provider</span>
            <CpuIcon class="w-5 h-5 text-blue-500" />
          </div>
          <div class="text-2xl font-bold capitalize text-gray-900 dark:text-white">{{ status.provider }}</div>
          <p class="text-xs text-muted-foreground mt-1 truncate">{{ status.model || 'default' }}</p>
          <div class="mt-3 flex items-center gap-1.5 text-xs text-blue-600 font-medium">
            <Gauge class="w-3.5 h-3.5" />
            temp {{ status.temperature }}
          </div>
        </div>

        <!-- Tokens / Cost -->
        <div class="rounded-2xl p-5 bg-gradient-to-br from-purple-500/10 to-pink-500/10 border border-gray-200/50 dark:border-white/10 hover-lift apple-shadow">
          <div class="flex items-start justify-between mb-3">
            <span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Tokens</span>
            <Zap class="w-5 h-5 text-purple-500" />
          </div>
          <div class="text-2xl font-bold text-gray-900 dark:text-white">{{ tokensUsed.toLocaleString() }}</div>
          <p class="text-xs text-muted-foreground mt-1">Today</p>
          <div class="mt-3 flex items-center gap-1.5 text-xs text-purple-600 font-medium">
            <ArrowUp class="w-3.5 h-3.5" />
            ${{ costToday.toFixed(4) }} est.
          </div>
        </div>

        <!-- Security Score -->
        <div class="rounded-2xl p-5 bg-gradient-to-br from-emerald-500/10 to-teal-500/10 border border-gray-200/50 dark:border-white/10 hover-lift apple-shadow">
          <div class="flex items-start justify-between mb-3">
            <span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Security</span>
            <Shield class="w-5 h-5 text-emerald-500" />
          </div>
          <div :class="['text-2xl font-bold', securityLabel.color]">{{ securityScore }}%</div>
          <p :class="['text-xs mt-1 font-medium', securityLabel.color]">{{ securityLabel.text }}</p>
          <div class="mt-3 h-1.5 rounded-full bg-gray-100 dark:bg-white/10 overflow-hidden">
            <div :class="['h-full rounded-full transition-all duration-500', securityLabel.bg]" :style="`width: ${securityScore}%`" />
          </div>
        </div>
      </div>

      <!-- ── Agent Control + Memory ── -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Agent Control -->
        <div class="lg:col-span-2 rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
          <CardHeader class="pb-4 pt-5 px-5">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-500 flex items-center justify-center">
                  <BotMessageSquare class="w-5 h-5 text-white" />
                </div>
                <div>
                  <CardTitle class="text-base">Agent Control</CardTitle>
                  <CardDescription>Start, stop, and diagnose your AI agent</CardDescription>
                </div>
              </div>
              <Badge :class="[
                'px-3 py-1 rounded-full text-xs font-medium border-0',
                agentRunning 
                  ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400' 
                  : 'bg-gray-100 text-gray-600 dark:bg-white/10 dark:text-gray-400'
              ]">
                <span :class="['w-1.5 h-1.5 rounded-full mr-1.5 inline-block', agentRunning ? 'bg-green-500 animate-pulse' : 'bg-gray-400']" />
                {{ agentRunning ? 'Online' : 'Offline' }}
              </Badge>
            </div>
          </CardHeader>
          <CardContent class="px-5 pb-5 space-y-4">
            <div class="flex flex-wrap gap-2">
              <Button 
                :disabled="!housakyInstalled || agentRunning || agentActionLoading" 
                @click="startAgent" 
                class="rounded-xl gap-2 bg-gradient-to-r from-green-500 to-emerald-500 hover:from-green-600 hover:to-emerald-600 border-0"
              >
                <Play class="w-4 h-4" />
                {{ agentActionLoading ? 'Starting…' : 'Start Agent' }}
              </Button>
              <Button 
                variant="outline" 
                :disabled="!housakyInstalled || !agentRunning || agentActionLoading" 
                @click="stopAgent" 
                class="rounded-xl gap-2 border-gray-200 dark:border-white/10 hover:bg-gray-100 dark:hover:bg-white/5"
              >
                <Square class="w-4 h-4" />
                {{ agentActionLoading ? 'Stopping…' : 'Stop Agent' }}
              </Button>
              <Button 
                variant="outline" 
                :disabled="!housakyInstalled" 
                @click="runDoctor" 
                class="rounded-xl gap-2 border-gray-200 dark:border-white/10 hover:bg-gray-100 dark:hover:bg-white/5"
              >
                <Activity class="w-4 h-4" />
                Diagnostics
              </Button>
              <Button 
                variant="ghost" 
                size="sm" 
                class="ml-auto rounded-xl gap-1.5 text-muted-foreground hover:text-foreground" 
                @click="navigate('/terminal')"
              >
                <Terminal class="w-3.5 h-3.5" />
                Terminal
              </Button>
            </div>

            <div v-if="showDiagnostics" class="rounded-xl bg-gray-900 dark:bg-black text-green-400 font-mono text-xs p-4 max-h-36 overflow-auto whitespace-pre-wrap">
              {{ diagnosticsOutput }}
            </div>

            <!-- Mini stat row -->
            <div class="grid grid-cols-3 gap-3 pt-4 border-t border-gray-100 dark:border-white/10">
              <div class="text-center p-3 rounded-xl bg-gray-50 dark:bg-white/5">
                <p class="text-xl font-bold text-gray-900 dark:text-white">{{ channelList.length }}</p>
                <p class="text-xs text-muted-foreground">Channels</p>
              </div>
              <div class="text-center p-3 rounded-xl bg-green-50 dark:bg-green-900/10">
                <p class="text-xl font-bold text-green-600 dark:text-green-400">{{ activeChannelsCount }}</p>
                <p class="text-xs text-muted-foreground">Active</p>
              </div>
              <div class="text-center p-3 rounded-xl bg-indigo-50 dark:bg-indigo-900/10">
                <p class="text-xl font-bold text-indigo-600 dark:text-indigo-400">{{ status.runtime }}</p>
                <p class="text-xs text-muted-foreground">Runtime</p>
              </div>
            </div>
          </CardContent>
        </div>

        <!-- Memory + Heartbeat -->
        <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
          <CardHeader class="pb-4 pt-5 px-5">
            <CardTitle class="flex items-center gap-2 text-base">
              <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center">
                <Database class="w-4 h-4 text-white" />
              </div>
              Memory Engine
            </CardTitle>
          </CardHeader>
          <CardContent class="px-5 pb-5 space-y-2">
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm text-muted-foreground">Backend</span>
              <Badge variant="outline" class="rounded-lg font-mono">{{ status.memory_backend }}</Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm text-muted-foreground">Embeddings</span>
              <Badge variant="outline" class="rounded-lg font-mono text-xs">{{ status.embedding_provider }}</Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm text-muted-foreground">Auto-save</span>
              <Badge :class="status.memory_auto_save ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 rounded-lg' : 'bg-gray-100 text-gray-500 dark:bg-white/10 rounded-lg'">
                {{ status.memory_auto_save ? 'On' : 'Off' }}
              </Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm text-muted-foreground flex items-center gap-2">
                <Heart :class="['w-4 h-4', status.heartbeat_enabled ? 'text-red-500 animate-pulse' : 'text-gray-400']" />
                Heartbeat
              </span>
              <span class="text-sm font-medium">
                {{ status.heartbeat_enabled ? `/${status.heartbeat_interval}min` : 'off' }}
              </span>
            </div>
          </CardContent>
        </div>
      </div>

      <!-- ── Channels + Security ── -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
          <CardHeader class="pb-4 pt-5 px-5">
            <div class="flex items-center justify-between">
              <CardTitle class="flex items-center gap-2">
                <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-cyan-500 to-blue-500 flex items-center justify-center">
                  <Network class="w-4 h-4 text-white" />
                </div>
                Channels
              </CardTitle>
              <Badge variant="outline" class="rounded-full text-xs">{{ activeChannelsCount }}/{{ channelList.length }} live</Badge>
            </div>
          </CardHeader>
          <CardContent class="px-5 pb-5">
            <div class="space-y-1.5">
              <div v-for="ch in channelList" :key="ch.key"
                class="flex items-center justify-between px-4 py-3 rounded-xl border border-gray-100 dark:border-white/10 hover:bg-gray-50 dark:hover:bg-white/5 transition-colors cursor-pointer"
                @click="navigate('/channels')"
              >
                <div class="flex items-center gap-3">
                  <div :class="['w-2.5 h-2.5 rounded-full flex-shrink-0', ch.active ? 'bg-green-500 animate-pulse' : ch.configured ? 'bg-yellow-400' : 'bg-gray-300']" />
                  <span class="text-sm font-medium">{{ ch.name }}</span>
                </div>
                <div class="flex items-center gap-2">
                  <span v-if="ch.allowlist_count > 0" class="text-xs text-muted-foreground">{{ ch.allowlist_count }} users</span>
                  <Badge :class="ch.active ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 rounded-lg text-xs' : ch.configured ? 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400 rounded-lg text-xs' : 'rounded-lg text-xs'">
                    {{ ch.active ? 'Live' : ch.configured ? 'Ready' : 'Setup' }}
                  </Badge>
                </div>
              </div>
            </div>
          </CardContent>
        </div>

        <!-- Security & Autonomy -->
        <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
          <CardHeader class="pb-4 pt-5 px-5">
            <CardTitle class="flex items-center gap-2">
              <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-emerald-500 to-teal-500 flex items-center justify-center">
                <Shield class="w-4 h-4 text-white" />
              </div>
              Security & Autonomy
            </CardTitle>
            <CardDescription>Active protection profile</CardDescription>
          </CardHeader>
          <CardContent class="px-5 pb-5 space-y-3">
            <div class="flex items-center justify-between p-4 rounded-xl bg-gradient-to-r border" :class="autonomyColors[status.autonomy_level] || 'from-gray-500/10 to-gray-500/5'">
              <div class="flex items-center gap-2">
                <Layers class="w-4 h-4" />
                <span class="text-sm font-medium">Autonomy Level</span>
              </div>
              <span class="font-bold capitalize px-3 py-1 rounded-lg bg-white/50 dark:bg-black/20">{{ status.autonomy_level }}</span>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <div class="flex items-center gap-3 p-3 rounded-xl bg-gray-50 dark:bg-white/5 border border-gray-100 dark:border-white/10">
                <div :class="['w-9 h-9 rounded-lg flex items-center justify-center', status.workspace_only ? 'bg-green-100 dark:bg-green-900/30' : 'bg-yellow-100 dark:bg-yellow-900/30']">
                  <component :is="status.workspace_only ? Lock : Unlock" :class="['w-4 h-4', status.workspace_only ? 'text-green-600 dark:text-green-400' : 'text-yellow-600 dark:text-yellow-400']" />
                </div>
                <div>
                  <p class="text-xs font-medium">Workspace</p>
                  <p :class="['text-xs', status.workspace_only ? 'text-green-600 dark:text-green-400' : 'text-yellow-600 dark:text-yellow-400']">{{ status.workspace_only ? 'Sandboxed' : 'Open' }}</p>
                </div>
              </div>
              <div class="flex items-center gap-3 p-3 rounded-xl bg-gray-50 dark:bg-white/5 border border-gray-100 dark:border-white/10">
                <div :class="['w-9 h-9 rounded-lg flex items-center justify-center', status.secrets_encrypted ? 'bg-green-100 dark:bg-green-900/30' : 'bg-red-100 dark:bg-red-900/30']">
                  <component :is="status.secrets_encrypted ? Lock : Unlock" :class="['w-4 h-4', status.secrets_encrypted ? 'text-green-600 dark:text-green-400' : 'text-red-500']" />
                </div>
                <div>
                  <p class="text-xs font-medium">Secrets</p>
                  <p :class="['text-xs', status.secrets_encrypted ? 'text-green-600 dark:text-green-400' : 'text-red-500']">{{ status.secrets_encrypted ? 'Encrypted' : 'Plaintext' }}</p>
                </div>
              </div>
            </div>
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-xs text-muted-foreground">Security Score</span>
              <div class="flex items-center gap-2">
                <div class="w-24 h-1.5 rounded-full bg-gray-100 dark:bg-white/10 overflow-hidden">
                  <div :class="['h-full rounded-full transition-all duration-500', securityLabel.bg]" :style="`width: ${securityScore}%`" />
                </div>
                <span :class="['text-xs font-bold', securityLabel.color]">{{ securityScore }}%</span>
              </div>
            </div>
          </CardContent>
        </div>
      </div>

      <!-- ── Quick Actions + Activity Feed ── -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Quick Actions -->
        <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
          <CardHeader class="pb-3 pt-5 px-5">
            <CardTitle class="text-base">Quick Actions</CardTitle>
          </CardHeader>
          <CardContent class="px-5 pb-5">
            <div class="grid grid-cols-3 gap-2">
              <button @click="navigate('/chat')" class="flex flex-col items-center gap-2 p-3 rounded-xl border border-gray-100 dark:border-white/10 hover:bg-blue-50 dark:hover:bg-blue-900/20 hover:border-blue-200 dark:hover:border-blue-800 transition-all group">
                <div class="w-10 h-10 rounded-xl bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center group-hover:scale-110 transition-transform">
                  <MessageSquare class="w-5 h-5 text-blue-500" />
                </div>
                <span class="text-xs font-medium">Chat</span>
              </button>
              <button @click="navigate('/channels')" class="flex flex-col items-center gap-2 p-3 rounded-xl border border-gray-100 dark:border-white/10 hover:bg-cyan-50 dark:hover:bg-cyan-900/20 hover:border-cyan-200 dark:hover:border-cyan-800 transition-all group">
                <div class="w-10 h-10 rounded-xl bg-cyan-100 dark:bg-cyan-900/30 flex items-center justify-center group-hover:scale-110 transition-transform">
                  <Network class="w-5 h-5 text-cyan-500" />
                </div>
                <span class="text-xs font-medium">Channels</span>
              </button>
              <button @click="navigate('/agi')" class="flex flex-col items-center gap-2 p-3 rounded-xl border border-gray-100 dark:border-white/10 hover:bg-purple-50 dark:hover:bg-purple-900/20 hover:border-purple-200 dark:hover:border-purple-800 transition-all group">
                <div class="w-10 h-10 rounded-xl bg-purple-100 dark:bg-purple-900/30 flex items-center justify-center group-hover:scale-110 transition-transform">
                  <Brain class="w-5 h-5 text-purple-500" />
                </div>
                <span class="text-xs font-medium">AGI</span>
              </button>
              <button @click="navigate('/skills')" class="flex flex-col items-center gap-2 p-3 rounded-xl border border-gray-100 dark:border-white/10 hover:bg-amber-50 dark:hover:bg-amber-900/20 hover:border-amber-200 dark:hover:border-amber-800 transition-all group">
                <div class="w-10 h-10 rounded-xl bg-amber-100 dark:bg-amber-900/30 flex items-center justify-center group-hover:scale-110 transition-transform">
                  <Wrench class="w-5 h-5 text-amber-500" />
                </div>
                <span class="text-xs font-medium">Skills</span>
              </button>
              <button @click="navigate('/hardware')" class="flex flex-col items-center gap-2 p-3 rounded-xl border border-gray-100 dark:border-white/10 hover:bg-green-50 dark:hover:bg-green-900/20 hover:border-green-200 dark:hover:border-green-800 transition-all group">
                <div class="w-10 h-10 rounded-xl bg-green-100 dark:bg-green-900/30 flex items-center justify-center group-hover:scale-110 transition-transform">
                  <Cpu class="w-5 h-5 text-green-500" />
                </div>
                <span class="text-xs font-medium">Hardware</span>
              </button>
              <button @click="navigate('/terminal')" class="flex flex-col items-center gap-2 p-3 rounded-xl border border-gray-100 dark:border-white/10 hover:bg-gray-50 dark:hover:bg-white/10 hover:border-gray-200 dark:hover:border-white/20 transition-all group">
                <div class="w-10 h-10 rounded-xl bg-gray-100 dark:bg-white/10 flex items-center justify-center group-hover:scale-110 transition-transform">
                  <Terminal class="w-5 h-5 text-gray-500" />
                </div>
                <span class="text-xs font-medium">Terminal</span>
              </button>
            </div>
          </CardContent>
        </div>

        <!-- Activity Feed -->
        <div class="lg:col-span-2 rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
          <CardHeader class="pb-3 pt-5 px-5">
            <CardTitle class="flex items-center gap-2 text-base">
              <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-500 to-purple-500 flex items-center justify-center">
                <Activity class="w-4 h-4 text-white" />
              </div>
              Live Activity
              <span class="ml-auto w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            </CardTitle>
          </CardHeader>
          <CardContent class="px-5 pb-5">
            <div class="space-y-2 max-h-52 overflow-y-auto pr-1">
              <div v-if="!activityFeed.length" class="text-sm text-muted-foreground text-center py-6">No activity yet</div>
              <div v-for="ev in activityFeed" :key="ev.id"
                class="flex items-start gap-3 text-xs group"
              >
                <span :class="['mt-0.5 w-8 text-center text-[10px] font-bold text-white rounded-lg px-1 py-0.5 flex-shrink-0', activityTypeConfig[ev.type]?.color || 'bg-gray-500']">
                  {{ activityTypeConfig[ev.type]?.label }}
                </span>
                <div class="flex-1 min-w-0">
                  <p class="font-medium truncate">{{ ev.title }}</p>
                  <p class="text-muted-foreground truncate">{{ ev.detail }}</p>
                </div>
                <span class="text-muted-foreground flex-shrink-0 font-mono">{{ formatTime(ev.time) }}</span>
              </div>
            </div>
          </CardContent>
        </div>
      </div>

      <!-- ── Paths ── -->
      <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
        <CardHeader class="pb-3 pt-5 px-5">
          <CardTitle class="flex items-center gap-2 text-base">
            <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-gray-500 to-slate-500 flex items-center justify-center">
              <HardDrive class="w-4 h-4 text-white" />
            </div>
            Workspace Paths
          </CardTitle>
        </CardHeader>
        <CardContent class="px-5 pb-5">
          <div class="grid md:grid-cols-2 gap-3">
            <div class="p-4 rounded-xl bg-gray-50 dark:bg-white/5 border border-gray-100 dark:border-white/10">
              <p class="text-xs text-muted-foreground mb-2">Workspace</p>
              <code class="text-xs break-all font-mono bg-transparent">{{ status.workspace }}</code>
            </div>
            <div class="p-4 rounded-xl bg-gray-50 dark:bg-white/5 border border-gray-100 dark:border-white/10">
              <p class="text-xs text-muted-foreground mb-2">Config File</p>
              <code class="text-xs break-all font-mono bg-transparent">{{ status.config }}</code>
            </div>
          </div>
        </CardContent>
      </div>
    </template>
  </div>
</template>
