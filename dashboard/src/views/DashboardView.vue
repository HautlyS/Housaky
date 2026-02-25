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
  Activity,
  Cpu,
  HardDrive,
  MessageSquare,
  Network,
  Wrench,
  Zap,
  CheckCircle2,
  AlertCircle,
  RefreshCw,
  Settings,
  Shield,
  Heart,
  Terminal,
  Play,
  Square,
  Clock,
  TrendingUp,
  Database
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

interface Metric {
  label: string
  value: string | number
  change?: string
  trend?: 'up' | 'down' | 'neutral'
}

const router = useRouter()
const status = ref<HousakyStatus | null>(null)
const loading = ref(true)
const error = ref('')
const housakyInstalled = ref(false)
const autoRefresh = ref(true)
const lastRefresh = ref<Date>(new Date())
const metrics = ref<Metric[]>([])
let refreshInterval: number | null = null

async function loadStatus() {
  loading.value = true
  error.value = ''
  
  try {
    if (!isTauri) {
      housakyInstalled.value = false
      loading.value = false
      return
    }
    housakyInstalled.value = await invoke<boolean>('check_housaky_installed')
    status.value = await invoke<HousakyStatus>('get_status')
    lastRefresh.value = new Date()
    
    metrics.value = [
      { label: 'Messages Today', value: 127, change: '+12%', trend: 'up' },
      { label: 'Avg Response', value: '1.2s', change: '-0.3s', trend: 'up' },
      { label: 'Active Skills', value: Object.keys(status.value.channels || {}).filter(k => status.value?.channels[k]?.active).length, trend: 'neutral' },
      { label: 'Memory Size', value: '24MB', trend: 'neutral' }
    ]
  } catch (e) {
    error.value = String(e)
    console.error('Failed to load status:', e)
  } finally {
    loading.value = false
  }
}

async function runDoctor() {
  if (!isTauri) {
    alert('Running in server mode - doctor not available')
    return
  }
  try {
    const result = await invoke<string>('run_doctor')
    alert(result || 'Doctor check completed')
  } catch (e) {
    alert(`Error: ${e}`)
  }
}

async function startAgent() {
  if (!isTauri) {
    error.value = 'Running in server mode - agent control not available'
    return
  }
  try {
    await invoke('run_housaky_command_cmd', { command: 'agent', args: ['--daemon'] })
    loadStatus()
  } catch (e) {
    error.value = String(e)
  }
}

async function stopAgent() {
  if (!isTauri) {
    error.value = 'Running in server mode - agent control not available'
    return
  }
  try {
    await invoke('run_housaky_command_cmd', { command: 'stop', args: [] })
    loadStatus()
  } catch (e) {
    error.value = String(e)
  }
}

const channelList = computed(() => {
  if (!status.value?.channels) return []
  return Object.entries(status.value.channels).map(([name, data]) => ({
    name: name.charAt(0).toUpperCase() + name.slice(1),
    ...data
  }))
})

const activeChannelsCount = computed(() => 
  channelList.value.filter(c => c.configured && c.active).length
)

function navigate(path: string) {
  router.push(path)
}

function formatUptime() {
  return '2h 34m'
}

function toggleAutoRefresh() {
  autoRefresh.value = !autoRefresh.value
  if (autoRefresh.value) {
    refreshInterval = window.setInterval(loadStatus, 30000)
  } else if (refreshInterval) {
    clearInterval(refreshInterval)
  }
}

onMounted(() => {
  loadStatus()
  if (autoRefresh.value) {
    refreshInterval = window.setInterval(loadStatus, 30000)
  }
})

onUnmounted(() => {
  if (refreshInterval) {
    clearInterval(refreshInterval)
  }
})
</script>

<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">Dashboard</h1>
        <p class="text-sm text-muted-foreground">Monitor and control your Housaky AI Assistant</p>
      </div>
      <div class="flex items-center gap-2">
        <div class="flex items-center gap-2 text-xs text-muted-foreground mr-2">
          <Clock class="w-3 h-3" />
          Last updated: {{ lastRefresh.toLocaleTimeString() }}
        </div>
        <Button 
          variant="outline" 
          size="sm"
          :class="autoRefresh ? 'text-green-600 border-green-200' : ''"
          @click="toggleAutoRefresh"
        >
          <Activity class="w-4 h-4 mr-2" />
          {{ autoRefresh ? 'Auto' : 'Manual' }}
        </Button>
        <Button size="sm" @click="loadStatus" :disabled="loading">
          <RefreshCw v-if="!loading" class="w-4 h-4 mr-2" />
          <RefreshCw v-else class="w-4 h-4 mr-2 animate-spin" />
          <span v-if="loading">Loading...</span>
          <span v-else>Refresh</span>
        </Button>
      </div>
    </div>

    <!-- Not Installed Warning -->
    <Card v-if="!housakyInstalled && !loading" class="border-yellow-500 bg-yellow-50 dark:bg-yellow-900/20">
      <CardContent class="pt-6">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <AlertCircle class="w-5 h-5 text-yellow-600" />
            <div>
              <p class="font-medium text-yellow-700 dark:text-yellow-400">Housaky is not installed</p>
              <p class="text-sm text-yellow-600 dark:text-yellow-500">Install it to enable full functionality.</p>
            </div>
          </div>
          <Button variant="outline" @click="router.push('/config')">
            Configure
          </Button>
        </div>
      </CardContent>
    </Card>

    <!-- Error State -->
    <Card v-if="error && !loading" class="border-destructive bg-destructive/10">
      <CardContent class="pt-6">
        <div class="flex items-center gap-2 text-destructive">
          <AlertCircle class="w-5 h-5" />
          <span>{{ error }}</span>
        </div>
      </CardContent>
    </Card>

    <!-- Loading State -->
    <div v-if="loading && !status" class="flex items-center justify-center py-12">
      <div class="flex flex-col items-center gap-3">
        <RefreshCw class="w-8 h-8 animate-spin text-muted-foreground" />
        <p class="text-sm text-muted-foreground">Loading status...</p>
      </div>
    </div>

    <template v-else-if="status">
      <!-- Quick Stats -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card class="hover:shadow-md transition-shadow">
          <CardHeader class="flex flex-row items-center justify-between pb-2">
            <CardTitle class="text-sm font-medium">Version</CardTitle>
            <Zap class="w-4 h-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold">{{ status.version || 'N/A' }}</div>
            <p class="text-xs text-muted-foreground flex items-center gap-1 mt-1">
              <CheckCircle2 class="w-3 h-3 text-green-500" />
              Housaky AI
            </p>
          </CardContent>
        </Card>

        <Card class="hover:shadow-md transition-shadow">
          <CardHeader class="flex flex-row items-center justify-between pb-2">
            <CardTitle class="text-sm font-medium">AI Provider</CardTitle>
            <Cpu class="w-4 h-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold capitalize">{{ status.provider }}</div>
            <p class="text-xs text-muted-foreground">{{ status.model || 'Default model' }}</p>
          </CardContent>
        </Card>

        <Card class="hover:shadow-md transition-shadow">
          <CardHeader class="flex flex-row items-center justify-between pb-2">
            <CardTitle class="text-sm font-medium">Memory</CardTitle>
            <Database class="w-4 h-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold capitalize">{{ status.memory_backend }}</div>
            <p class="text-xs text-muted-foreground">
              {{ status.embedding_provider }} · Auto: {{ status.memory_auto_save ? 'on' : 'off' }}
            </p>
          </CardContent>
        </Card>

        <Card class="hover:shadow-md transition-shadow">
          <CardHeader class="flex flex-row items-center justify-between pb-2">
            <CardTitle class="text-sm font-medium">Runtime</CardTitle>
            <Activity class="w-4 h-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold capitalize">{{ status.runtime }}</div>
            <p class="text-xs text-muted-foreground flex items-center gap-1">
              <Heart :class="['w-3 h-3', status.heartbeat_enabled ? 'text-red-500 animate-pulse' : 'text-gray-400']" />
              {{ status.heartbeat_enabled ? `Every ${status.heartbeat_interval}min` : 'No heartbeat' }}
            </p>
          </CardContent>
        </Card>
      </div>

      <!-- Agent Controls -->
      <Card>
        <CardHeader>
          <div class="flex items-center justify-between">
            <div>
              <CardTitle>Agent Control</CardTitle>
              <CardDescription>Start or stop the AI agent</CardDescription>
            </div>
            <Badge :variant="housakyInstalled ? 'success' : 'secondary'" class="flex items-center gap-1">
              <div :class="['w-2 h-2 rounded-full', housakyInstalled ? 'bg-green-500' : 'bg-gray-400']"></div>
              {{ housakyInstalled ? 'Ready' : 'Not Installed' }}
            </Badge>
          </div>
        </CardHeader>
        <CardContent>
          <div class="flex flex-wrap gap-3">
            <Button 
              :disabled="!housakyInstalled"
              @click="startAgent"
              class="flex items-center gap-2"
            >
              <Play class="w-4 h-4" />
              Start Agent
            </Button>
            <Button 
              variant="outline"
              :disabled="!housakyInstalled"
              @click="stopAgent"
              class="flex items-center gap-2"
            >
              <Square class="w-4 h-4" />
              Stop Agent
            </Button>
            <Button 
              variant="outline"
              :disabled="!housakyInstalled"
              @click="runDoctor"
              class="flex items-center gap-2"
            >
              <Activity class="w-4 h-4" />
              Run Diagnostics
            </Button>
          </div>
        </CardContent>
      </Card>

      <!-- Two Column Layout -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- Channels Status -->
        <Card>
          <CardHeader>
            <div class="flex items-center justify-between">
              <div>
                <CardTitle class="flex items-center gap-2">
                  <Network class="w-5 h-5" />
                  Channels
                </CardTitle>
                <CardDescription>{{ activeChannelsCount }} of {{ channelList.length }} active</CardDescription>
              </div>
              <Button variant="outline" size="sm" @click="navigate('/channels')">
                Configure
              </Button>
            </div>
          </CardHeader>
          <CardContent>
            <div class="space-y-2">
              <div
                v-for="channel in channelList"
                :key="channel.name"
                class="flex items-center justify-between p-3 rounded-lg border hover:bg-muted/50 transition-colors"
              >
                <div class="flex items-center gap-3">
                  <div :class="[
                    'w-2 h-2 rounded-full',
                    channel.active ? 'bg-green-500' : channel.configured ? 'bg-yellow-500' : 'bg-gray-300'
                  ]"></div>
                  <span class="font-medium">{{ channel.name }}</span>
                </div>
                <Badge :variant="channel.active ? 'success' : channel.configured ? 'warning' : 'secondary'">
                  {{ channel.active ? 'Active' : channel.configured ? 'Ready' : 'Not configured' }}
                </Badge>
              </div>
            </div>
          </CardContent>
        </Card>

        <!-- Security & Autonomy -->
        <Card>
          <CardHeader>
            <CardTitle class="flex items-center gap-2">
              <Shield class="w-5 h-5" />
              Security & Autonomy
            </CardTitle>
            <CardDescription>Current security settings</CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="flex items-center justify-between p-3 rounded-lg bg-muted/50">
              <span class="text-muted-foreground">Autonomy Level</span>
              <Badge :variant="status.autonomy_level === 'full' ? 'success' : status.autonomy_level === 'supervised' ? 'warning' : 'secondary'">
                {{ status.autonomy_level }}
              </Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-lg bg-muted/50">
              <span class="text-muted-foreground">Workspace Only</span>
              <Badge :variant="status.workspace_only ? 'success' : 'warning'">
                {{ status.workspace_only ? 'Enabled' : 'Disabled' }}
              </Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-lg bg-muted/50">
              <span class="text-muted-foreground">Secrets Encrypted</span>
              <Badge :variant="status.secrets_encrypted ? 'success' : 'destructive'">
                {{ status.secrets_encrypted ? 'Yes' : 'No' }}
              </Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-lg bg-muted/50">
              <span class="text-muted-foreground">Temperature</span>
              <code class="text-sm font-mono bg-background px-2 py-1 rounded">{{ status.temperature }}</code>
            </div>
          </CardContent>
        </Card>
      </div>

      <!-- Quick Actions -->
      <Card>
        <CardHeader>
          <CardTitle>Quick Actions</CardTitle>
          <CardDescription>Common tasks and operations</CardDescription>
        </CardHeader>
        <CardContent>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
            <Button variant="outline" class="h-auto py-4 flex flex-col gap-2" @click="navigate('/chat')">
              <MessageSquare class="w-5 h-5" />
              <span>Start Chat</span>
            </Button>
            <Button variant="outline" class="h-auto py-4 flex flex-col gap-2" @click="navigate('/channels')">
              <Network class="w-5 h-5" />
              <span>Channels</span>
            </Button>
            <Button variant="outline" class="h-auto py-4 flex flex-col gap-2" @click="navigate('/config')">
              <Settings class="w-5 h-5" />
              <span>Config</span>
            </Button>
            <Button variant="outline" class="h-auto py-4 flex flex-col gap-2" @click="navigate('/terminal')">
              <Terminal class="w-5 h-5" />
              <span>Terminal</span>
            </Button>
          </div>
        </CardContent>
      </Card>

      <!-- Workspace Info -->
      <Card>
        <CardHeader>
          <CardTitle class="flex items-center gap-2">
            <HardDrive class="w-5 h-5" />
            Paths & Locations
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div class="grid md:grid-cols-2 gap-4">
            <div class="p-3 rounded-lg bg-muted/50">
              <p class="text-xs text-muted-foreground mb-1">Workspace</p>
              <code class="text-sm break-all">{{ status.workspace }}</code>
            </div>
            <div class="p-3 rounded-lg bg-muted/50">
              <p class="text-xs text-muted-foreground mb-1">Config File</p>
              <code class="text-sm break-all">{{ status.config }}</code>
            </div>
          </div>
        </CardContent>
      </Card>
    </template>
  </div>
</template>
