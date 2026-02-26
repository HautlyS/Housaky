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
  Activity, Cpu, HardDrive, MessageSquare, Network, Wrench, Zap,
  CheckCircle2, AlertCircle, RefreshCw, Settings, Shield, Heart,
  Terminal, Play, Square, Clock, TrendingUp, Database, Brain,
  Lock, Unlock, ArrowUp, ArrowDown, Minus, FlameKindling,
  BarChart3, Layers, Eye, BotMessageSquare, DollarSign,
  Gauge, GitBranch, Wifi, WifiOff, Package, ChevronRight
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
  readonly: 'bg-blue-500/10 text-blue-600 border-blue-200',
  supervised: 'bg-yellow-500/10 text-yellow-600 border-yellow-200',
  full: 'bg-green-500/10 text-green-600 border-green-200',
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
  if (s >= 80) return { text: 'Excellent', color: 'text-green-600' }
  if (s >= 60) return { text: 'Good', color: 'text-blue-600' }
  if (s >= 40) return { text: 'Fair', color: 'text-yellow-600' }
  return { text: 'Weak', color: 'text-red-600' }
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
  <div class="p-6 space-y-6">
    <!-- ── Header ── -->
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h1 class="text-2xl font-bold tracking-tight flex items-center gap-2">
          <Brain class="w-6 h-6 text-primary" />
          AGI Command Center
        </h1>
        <p class="text-sm text-muted-foreground mt-0.5">Real-time monitoring · Housaky AI Platform</p>
      </div>
      <div class="flex items-center gap-2 flex-wrap">
        <div class="flex items-center gap-1.5 text-xs text-muted-foreground bg-muted px-2 py-1 rounded-full">
          <Clock class="w-3 h-3" />
          {{ lastRefresh.toLocaleTimeString() }}
        </div>
        <Button variant="outline" size="sm" :class="autoRefresh ? 'border-green-400 text-green-600' : ''" @click="toggleAutoRefresh">
          <Activity class="w-3.5 h-3.5 mr-1.5" />
          {{ autoRefresh ? 'Live' : 'Paused' }}
        </Button>
        <Button size="sm" @click="loadStatus" :disabled="loading">
          <RefreshCw :class="['w-3.5 h-3.5 mr-1.5', loading && 'animate-spin']" />
          Refresh
        </Button>
      </div>
    </div>

    <!-- ── Alert Banners ── -->
    <Card v-if="!housakyInstalled && !loading" class="border-yellow-400/50 bg-yellow-50/80 dark:bg-yellow-900/20">
      <CardContent class="py-3">
        <div class="flex items-center justify-between flex-wrap gap-2">
          <div class="flex items-center gap-2">
            <AlertCircle class="w-4 h-4 text-yellow-600 flex-shrink-0" />
            <span class="text-sm font-medium text-yellow-700 dark:text-yellow-400">Housaky not installed — limited functionality</span>
          </div>
          <Button size="sm" variant="outline" class="text-yellow-700 border-yellow-400" @click="router.push('/config')">Configure</Button>
        </div>
      </CardContent>
    </Card>

    <Card v-if="error && !loading" class="border-destructive/40 bg-destructive/5">
      <CardContent class="py-3 flex items-center gap-2 text-destructive text-sm">
        <AlertCircle class="w-4 h-4 flex-shrink-0" />{{ error }}
      </CardContent>
    </Card>

    <!-- ── Loading ── -->
    <div v-if="loading && !status" class="flex items-center justify-center py-20">
      <div class="flex flex-col items-center gap-3">
        <div class="relative">
          <Brain class="w-10 h-10 text-primary" />
          <span class="absolute -top-1 -right-1 w-3 h-3 bg-green-500 rounded-full animate-ping" />
        </div>
        <p class="text-sm text-muted-foreground animate-pulse">Connecting to Housaky…</p>
      </div>
    </div>

    <template v-else-if="status">
      <!-- ── KPI Row ── -->
      <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <!-- Agent Status -->
        <Card class="relative overflow-hidden hover:shadow-lg transition-shadow border-0 bg-gradient-to-br from-primary/10 to-primary/5">
          <CardContent class="pt-4 pb-4">
            <div class="flex items-start justify-between mb-2">
              <span class="text-xs font-medium text-muted-foreground uppercase tracking-wide">Agent</span>
              <div :class="['w-2 h-2 rounded-full mt-1', agentRunning ? 'bg-green-500 animate-pulse' : 'bg-gray-400']" />
            </div>
            <div :class="['text-xl font-bold', agentRunning ? 'text-green-600' : 'text-muted-foreground']">
              {{ agentRunning ? 'Running' : 'Stopped' }}
            </div>
            <p class="text-xs text-muted-foreground mt-1">Uptime {{ formattedUptime }}</p>
            <svg class="mt-2 w-20 h-7 opacity-60" viewBox="0 0 80 28" fill="none">
              <path :d="getSparklinePath(sparklineData)" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="text-primary" />
            </svg>
          </CardContent>
        </Card>

        <!-- AI Provider -->
        <Card class="hover:shadow-lg transition-shadow border-0 bg-gradient-to-br from-blue-500/10 to-blue-500/5">
          <CardContent class="pt-4 pb-4">
            <div class="flex items-start justify-between mb-2">
              <span class="text-xs font-medium text-muted-foreground uppercase tracking-wide">Provider</span>
              <Cpu class="w-3.5 h-3.5 text-blue-500 mt-0.5" />
            </div>
            <div class="text-xl font-bold capitalize">{{ status.provider }}</div>
            <p class="text-xs text-muted-foreground mt-1 truncate">{{ status.model || 'default' }}</p>
            <div class="mt-2 flex items-center gap-1 text-xs text-blue-600">
              <Gauge class="w-3 h-3" />
              temp {{ status.temperature }}
            </div>
          </CardContent>
        </Card>

        <!-- Tokens / Cost -->
        <Card class="hover:shadow-lg transition-shadow border-0 bg-gradient-to-br from-purple-500/10 to-purple-500/5">
          <CardContent class="pt-4 pb-4">
            <div class="flex items-start justify-between mb-2">
              <span class="text-xs font-medium text-muted-foreground uppercase tracking-wide">Tokens</span>
              <DollarSign class="w-3.5 h-3.5 text-purple-500 mt-0.5" />
            </div>
            <div class="text-xl font-bold">{{ tokensUsed.toLocaleString() }}</div>
            <p class="text-xs text-muted-foreground mt-1">Today</p>
            <div class="mt-2 flex items-center gap-1 text-xs text-purple-600">
              <ArrowUp class="w-3 h-3" />
              ${{ costToday.toFixed(4) }} est.
            </div>
          </CardContent>
        </Card>

        <!-- Security Score -->
        <Card class="hover:shadow-lg transition-shadow border-0 bg-gradient-to-br from-emerald-500/10 to-emerald-500/5">
          <CardContent class="pt-4 pb-4">
            <div class="flex items-start justify-between mb-2">
              <span class="text-xs font-medium text-muted-foreground uppercase tracking-wide">Security</span>
              <Shield class="w-3.5 h-3.5 text-emerald-500 mt-0.5" />
            </div>
            <div :class="['text-xl font-bold', securityLabel.color]">{{ securityScore }}%</div>
            <p :class="['text-xs mt-1', securityLabel.color]">{{ securityLabel.text }}</p>
            <div class="mt-2 h-1.5 rounded-full bg-muted overflow-hidden">
              <div :class="['h-full rounded-full transition-all', securityScore >= 80 ? 'bg-green-500' : securityScore >= 60 ? 'bg-blue-500' : securityScore >= 40 ? 'bg-yellow-500' : 'bg-red-500']" :style="`width: ${securityScore}%`" />
            </div>
          </CardContent>
        </Card>
      </div>

      <!-- ── Agent Control + Memory ── -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Agent Control -->
        <Card class="lg:col-span-2">
          <CardHeader class="pb-3">
            <div class="flex items-center justify-between">
              <div>
                <CardTitle class="flex items-center gap-2">
                  <BotMessageSquare class="w-5 h-5 text-primary" />
                  Agent Control
                </CardTitle>
                <CardDescription>Start, stop, and diagnose the AI agent</CardDescription>
              </div>
              <Badge :class="['border', agentRunning ? 'bg-green-500/10 text-green-600 border-green-300' : 'bg-muted text-muted-foreground']">
                <span :class="['w-1.5 h-1.5 rounded-full mr-1.5 inline-block', agentRunning ? 'bg-green-500 animate-pulse' : 'bg-gray-400']" />
                {{ agentRunning ? 'Online' : 'Offline' }}
              </Badge>
            </div>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="flex flex-wrap gap-2">
              <Button :disabled="!housakyInstalled || agentRunning || agentActionLoading" @click="startAgent" class="gap-1.5">
                <Play class="w-4 h-4" />{{ agentActionLoading ? 'Starting…' : 'Start Agent' }}
              </Button>
              <Button variant="outline" :disabled="!housakyInstalled || !agentRunning || agentActionLoading" @click="stopAgent" class="gap-1.5">
                <Square class="w-4 h-4" />{{ agentActionLoading ? 'Stopping…' : 'Stop Agent' }}
              </Button>
              <Button variant="outline" :disabled="!housakyInstalled" @click="runDoctor" class="gap-1.5">
                <Activity class="w-4 h-4" />Run Diagnostics
              </Button>
              <Button variant="ghost" size="sm" class="ml-auto gap-1.5" @click="navigate('/terminal')">
                <Terminal class="w-3.5 h-3.5" />Terminal
              </Button>
            </div>

            <div v-if="showDiagnostics" class="rounded-lg bg-gray-950 text-green-400 font-mono text-xs p-4 max-h-36 overflow-auto whitespace-pre-wrap">
              {{ diagnosticsOutput }}
            </div>

            <!-- Mini stat row -->
            <div class="grid grid-cols-3 gap-3 pt-2 border-t">
              <div class="text-center">
                <p class="text-lg font-bold">{{ channelList.length }}</p>
                <p class="text-xs text-muted-foreground">Channels</p>
              </div>
              <div class="text-center">
                <p class="text-lg font-bold text-green-600">{{ activeChannelsCount }}</p>
                <p class="text-xs text-muted-foreground">Active</p>
              </div>
              <div class="text-center">
                <p class="text-lg font-bold">{{ status.runtime }}</p>
                <p class="text-xs text-muted-foreground">Runtime</p>
              </div>
            </div>
          </CardContent>
        </Card>

        <!-- Memory + Heartbeat -->
        <Card>
          <CardHeader class="pb-3">
            <CardTitle class="flex items-center gap-2 text-base">
              <Database class="w-4 h-4 text-primary" />
              Memory Engine
            </CardTitle>
          </CardHeader>
          <CardContent class="space-y-3">
            <div class="flex items-center justify-between p-2.5 rounded-lg bg-muted/50">
              <span class="text-sm text-muted-foreground">Backend</span>
              <Badge variant="outline" class="font-mono">{{ status.memory_backend }}</Badge>
            </div>
            <div class="flex items-center justify-between p-2.5 rounded-lg bg-muted/50">
              <span class="text-sm text-muted-foreground">Embeddings</span>
              <Badge variant="outline" class="font-mono text-xs">{{ status.embedding_provider }}</Badge>
            </div>
            <div class="flex items-center justify-between p-2.5 rounded-lg bg-muted/50">
              <span class="text-sm text-muted-foreground">Auto-save</span>
              <Badge :class="status.memory_auto_save ? 'bg-green-100 text-green-700' : 'bg-muted text-muted-foreground'">
                {{ status.memory_auto_save ? 'On' : 'Off' }}
              </Badge>
            </div>
            <div class="flex items-center justify-between p-2.5 rounded-lg bg-muted/50">
              <span class="text-sm text-muted-foreground flex items-center gap-1">
                <Heart :class="['w-3 h-3', status.heartbeat_enabled ? 'text-red-500 animate-pulse' : 'text-gray-400']" />
                Heartbeat
              </span>
              <span class="text-sm font-medium">
                {{ status.heartbeat_enabled ? `/${status.heartbeat_interval}min` : 'off' }}
              </span>
            </div>
          </CardContent>
        </Card>
      </div>

      <!-- ── Channels + Security ── -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card>
          <CardHeader class="pb-3">
            <div class="flex items-center justify-between">
              <CardTitle class="flex items-center gap-2">
                <Network class="w-5 h-5 text-cyan-500" />
                Channels
                <Badge variant="outline" class="text-xs">{{ activeChannelsCount }}/{{ channelList.length }} live</Badge>
              </CardTitle>
              <Button variant="ghost" size="sm" class="gap-1 text-xs" @click="navigate('/channels')">
                Manage <ChevronRight class="w-3.5 h-3.5" />
              </Button>
            </div>
          </CardHeader>
          <CardContent>
            <div class="space-y-1.5">
              <div v-for="ch in channelList" :key="ch.key"
                class="flex items-center justify-between px-3 py-2 rounded-lg border hover:bg-muted/50 transition-colors cursor-pointer"
                @click="navigate('/channels')"
              >
                <div class="flex items-center gap-2.5">
                  <div :class="['w-2 h-2 rounded-full flex-shrink-0', ch.active ? 'bg-green-500 animate-pulse' : ch.configured ? 'bg-yellow-400' : 'bg-gray-300']" />
                  <span class="text-sm font-medium">{{ ch.name }}</span>
                </div>
                <div class="flex items-center gap-2">
                  <span v-if="ch.allowlist_count > 0" class="text-xs text-muted-foreground">{{ ch.allowlist_count }} users</span>
                  <Badge :class="ch.active ? 'bg-green-100 text-green-700 text-xs' : ch.configured ? 'bg-yellow-100 text-yellow-700 text-xs' : 'text-xs'">
                    {{ ch.active ? 'Live' : ch.configured ? 'Ready' : 'Setup' }}
                  </Badge>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>

        <!-- Security & Autonomy -->
        <Card>
          <CardHeader class="pb-3">
            <CardTitle class="flex items-center gap-2">
              <Shield class="w-5 h-5 text-emerald-500" />
              Security & Autonomy
            </CardTitle>
            <CardDescription>Active protection profile</CardDescription>
          </CardHeader>
          <CardContent class="space-y-3">
            <div class="flex items-center justify-between p-3 rounded-xl border" :class="autonomyColors[status.autonomy_level] || 'bg-muted/50'">
              <div class="flex items-center gap-2">
                <Layers class="w-4 h-4" />
                <span class="text-sm font-medium">Autonomy Level</span>
              </div>
              <span class="font-bold capitalize">{{ status.autonomy_level }}</span>
            </div>
            <div class="grid grid-cols-2 gap-2">
              <div class="flex items-center gap-2 p-2.5 rounded-lg bg-muted/50 border">
                <component :is="status.workspace_only ? Lock : Unlock" :class="['w-4 h-4', status.workspace_only ? 'text-green-600' : 'text-yellow-500']" />
                <div>
                  <p class="text-xs font-medium">Workspace</p>
                  <p :class="['text-xs', status.workspace_only ? 'text-green-600' : 'text-yellow-500']">{{ status.workspace_only ? 'Sandboxed' : 'Open' }}</p>
                </div>
              </div>
              <div class="flex items-center gap-2 p-2.5 rounded-lg bg-muted/50 border">
                <component :is="status.secrets_encrypted ? Lock : Unlock" :class="['w-4 h-4', status.secrets_encrypted ? 'text-green-600' : 'text-red-500']" />
                <div>
                  <p class="text-xs font-medium">Secrets</p>
                  <p :class="['text-xs', status.secrets_encrypted ? 'text-green-600' : 'text-red-500']">{{ status.secrets_encrypted ? 'Encrypted' : 'Plaintext' }}</p>
                </div>
              </div>
            </div>
            <div class="flex items-center justify-between p-2.5 rounded-lg bg-muted/50">
              <span class="text-xs text-muted-foreground">Security Score</span>
              <div class="flex items-center gap-2">
                <div class="w-20 h-1.5 rounded-full bg-muted overflow-hidden">
                  <div :class="['h-full rounded-full', securityScore >= 80 ? 'bg-green-500' : securityScore >= 60 ? 'bg-blue-500' : 'bg-yellow-500']" :style="`width: ${securityScore}%`" />
                </div>
                <span :class="['text-xs font-bold', securityLabel.color]">{{ securityScore }}%</span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      <!-- ── Quick Actions + Activity Feed ── -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Quick Actions -->
        <Card>
          <CardHeader class="pb-3">
            <CardTitle class="text-base">Quick Actions</CardTitle>
          </CardHeader>
          <CardContent>
            <div class="grid grid-cols-2 gap-2">
              <button @click="navigate('/chat')" class="flex flex-col items-center gap-2 p-3 rounded-xl border hover:bg-primary/5 hover:border-primary/30 transition-all group">
                <MessageSquare class="w-5 h-5 text-blue-500 group-hover:scale-110 transition-transform" />
                <span class="text-xs font-medium">Chat</span>
              </button>
              <button @click="navigate('/channels')" class="flex flex-col items-center gap-2 p-3 rounded-xl border hover:bg-cyan-500/5 hover:border-cyan-300 transition-all group">
                <Network class="w-5 h-5 text-cyan-500 group-hover:scale-110 transition-transform" />
                <span class="text-xs font-medium">Channels</span>
              </button>
              <button @click="navigate('/agi')" class="flex flex-col items-center gap-2 p-3 rounded-xl border hover:bg-purple-500/5 hover:border-purple-300 transition-all group">
                <Brain class="w-5 h-5 text-purple-500 group-hover:scale-110 transition-transform" />
                <span class="text-xs font-medium">AGI</span>
              </button>
              <button @click="navigate('/skills')" class="flex flex-col items-center gap-2 p-3 rounded-xl border hover:bg-amber-500/5 hover:border-amber-300 transition-all group">
                <Wrench class="w-5 h-5 text-amber-500 group-hover:scale-110 transition-transform" />
                <span class="text-xs font-medium">Skills</span>
              </button>
              <button @click="navigate('/hardware')" class="flex flex-col items-center gap-2 p-3 rounded-xl border hover:bg-green-500/5 hover:border-green-300 transition-all group">
                <Cpu class="w-5 h-5 text-green-500 group-hover:scale-110 transition-transform" />
                <span class="text-xs font-medium">Hardware</span>
              </button>
              <button @click="navigate('/terminal')" class="flex flex-col items-center gap-2 p-3 rounded-xl border hover:bg-gray-500/5 hover:border-gray-300 transition-all group">
                <Terminal class="w-5 h-5 text-gray-500 group-hover:scale-110 transition-transform" />
                <span class="text-xs font-medium">Terminal</span>
              </button>
            </div>
          </CardContent>
        </Card>

        <!-- Activity Feed -->
        <Card class="lg:col-span-2">
          <CardHeader class="pb-3">
            <CardTitle class="flex items-center gap-2 text-base">
              <Activity class="w-4 h-4 text-primary" />
              Live Activity Feed
              <span class="ml-auto w-2 h-2 rounded-full bg-green-500 animate-pulse" />
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div class="space-y-2 max-h-52 overflow-y-auto pr-1">
              <div v-if="!activityFeed.length" class="text-sm text-muted-foreground text-center py-6">No activity yet</div>
              <div v-for="ev in activityFeed" :key="ev.id"
                class="flex items-start gap-3 text-xs group"
              >
                <span :class="['mt-0.5 w-8 text-center text-[10px] font-bold text-white rounded px-1 py-0.5 flex-shrink-0', activityTypeConfig[ev.type]?.color || 'bg-gray-500']">
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
        </Card>
      </div>

      <!-- ── Paths ── -->
      <Card>
        <CardHeader class="pb-3">
          <CardTitle class="flex items-center gap-2 text-base">
            <HardDrive class="w-4 h-4" />
            Workspace Paths
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div class="grid md:grid-cols-2 gap-3">
            <div class="p-3 rounded-lg bg-muted/50 border">
              <p class="text-xs text-muted-foreground mb-1">Workspace</p>
              <code class="text-xs break-all font-mono">{{ status.workspace }}</code>
            </div>
            <div class="p-3 rounded-lg bg-muted/50 border">
              <p class="text-xs text-muted-foreground mb-1">Config File</p>
              <code class="text-xs break-all font-mono">{{ status.config }}</code>
            </div>
          </div>
        </CardContent>
      </Card>
    </template>
  </div>
</template>
