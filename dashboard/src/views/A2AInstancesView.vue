<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import CardDescription from '@/components/ui/card-description.vue'
import Button from '@/components/ui/button.vue'
import Badge from '@/components/ui/badge.vue'
import { 
  Network, Wifi, WifiOff, Cpu, Clock, Zap, Shield, 
  Lock, Unlock, Activity, RefreshCw, Send, Users,
  Server, Smartphone, Globe, Key, CheckCircle, XCircle,
  ArrowRight, Signal, Gauge, Terminal
} from 'lucide-vue-next'

interface A2AInstance {
  id: string
  name: string
  type: 'native' | 'openclaw' | 'external'
  status: 'connected' | 'disconnected' | 'connecting'
  endpoint: string
  protocol: string
  uptime: number
  lastPing: number
  tasksProcessed: number
  messagesExchanged: number
  encryption: string
  quicEnabled: boolean
  pqReady: boolean
}

const instances = ref<A2AInstance[]>([])
const loading = ref(true)
const selectedInstance = ref<A2AInstance | null>(null)
const sendingTask = ref(false)
const taskResult = ref('')

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const connectedCount = computed(() => instances.value.filter(i => i.status === 'connected').length)

const statusColors: Record<string, string> = {
  connected: 'bg-green-500',
  disconnected: 'bg-gray-400',
  connecting: 'bg-yellow-500 animate-pulse'
}

async function loadInstances() {
  loading.value = true
  try {
    if (isTauri) {
      const result = await invoke<A2AInstance[]>('get_a2a_instances')
      instances.value = result
    } else {
      instances.value = [
        {
          id: 'native-1',
          name: 'Housaky-Native',
          type: 'native',
          status: 'connected',
          endpoint: 'quic://localhost:4444',
          protocol: 'A2A/1.0 + QUIC',
          uptime: 3600,
          lastPing: Date.now(),
          tasksProcessed: 42,
          messagesExchanged: 156,
          encryption: 'Kyber-1024 + AES-256-GCM',
          quicEnabled: true,
          pqReady: true
        },
        {
          id: 'openclaw-1',
          name: 'OpenClaw-Agent',
          type: 'openclaw',
          status: 'connected',
          endpoint: 'quic://192.168.1.100:4445',
          protocol: 'A2A/1.0 + QUIC',
          uptime: 7200,
          lastPing: Date.now() - 5000,
          tasksProcessed: 89,
          messagesExchanged: 234,
          encryption: 'Kyber-1024 + AES-256-GCM',
          quicEnabled: true,
          pqReady: true
        },
        {
          id: 'external-1',
          name: 'Remote-Worker-01',
          type: 'external',
          status: 'disconnected',
          endpoint: 'quic://worker.example.com:4446',
          protocol: 'A2A/1.0',
          uptime: 0,
          lastPing: Date.now() - 3600000,
          tasksProcessed: 12,
          messagesExchanged: 45,
          encryption: 'TLS 1.3',
          quicEnabled: false,
          pqReady: false
        }
      ]
    }
  } catch (e) {
    console.error('Failed to load instances:', e)
  } finally {
    loading.value = false
  }
}

async function pingInstance(instance: A2AInstance) {
  if (!isTauri) return
  try {
    await invoke('a2a_ping', { instanceId: instance.id })
    instance.lastPing = Date.now()
  } catch (e) {
    console.error('Ping failed:', e)
  }
}

async function syncInstance(instance: A2AInstance) {
  if (!isTauri || sendingTask.value) return
  sendingTask.value = true
  taskResult.value = ''
  try {
    const result = await invoke<string>('a2a_sync', { instanceId: instance.id })
    taskResult.value = result
  } catch (e) {
    taskResult.value = `Error: ${e}`
  } finally {
    sendingTask.value = false
  }
}

function formatUptime(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

function formatLastPing(timestamp: number): string {
  const diff = Date.now() - timestamp
  if (diff < 60000) return 'Just now'
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`
  return `${Math.floor(diff / 3600000)}h ago`
}

onMounted(() => {
  loadInstances()
})
</script>

<template>
  <div class="space-y-6 max-w-7xl mx-auto">
    <!-- Header -->
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div class="flex items-center gap-4">
        <div class="w-14 h-14 rounded-2xl bg-gradient-to-br from-cyan-500 via-blue-500 to-indigo-500 flex items-center justify-center shadow-lg shadow-cyan-500/30">
          <Network class="w-7 h-7 text-white" />
        </div>
        <div>
          <h1 class="text-2xl font-bold text-gray-900 dark:text-white">A2A Cross-Device</h1>
          <p class="text-sm text-muted-foreground">Agent-to-Agent Network · QUIC Protocol</p>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" class="rounded-full" @click="loadInstances" :disabled="loading">
          <RefreshCw :class="['w-3.5 h-3.5 mr-1.5', loading && 'animate-spin']" />
          Refresh
        </Button>
      </div>
    </div>

    <!-- Network Overview -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <div class="rounded-2xl p-5 bg-gradient-to-br from-cyan-500/10 to-blue-500/10 border border-gray-200/50 dark:border-white/10">
        <div class="flex items-start justify-between mb-3">
          <span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Network</span>
          <Wifi class="w-5 h-5 text-cyan-500" />
        </div>
        <div class="text-2xl font-bold text-cyan-600 dark:text-cyan-400">{{ connectedCount }}</div>
        <p class="text-xs text-muted-foreground mt-1">Connected</p>
      </div>

      <div class="rounded-2xl p-5 bg-gradient-to-br from-blue-500/10 to-indigo-500/10 border border-gray-200/50 dark:border-white/10">
        <div class="flex items-start justify-between mb-3">
          <span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Protocol</span>
          <Signal class="w-5 h-5 text-blue-500" />
        </div>
        <div class="text-2xl font-bold text-gray-900 dark:text-white">QUIC</div>
        <p class="text-xs text-muted-foreground mt-1">0-RTT Connection</p>
      </div>

      <div class="rounded-2xl p-5 bg-gradient-to-br from-indigo-500/10 to-purple-500/10 border border-gray-200/50 dark:border-white/10">
        <div class="flex items-start justify-between mb-3">
          <span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Encryption</span>
          <Shield class="w-5 h-5 text-indigo-500" />
        </div>
        <div class="text-2xl font-bold text-indigo-600 dark:text-indigo-400">PQ-Safe</div>
        <p class="text-xs text-muted-foreground mt-1">Kyber-1024</p>
      </div>

      <div class="rounded-2xl p-5 bg-gradient-to-br from-purple-500/10 to-pink-500/10 border border-gray-200/50 dark:border-white/10">
        <div class="flex items-start justify-between mb-3">
          <span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Messages</span>
          <Activity class="w-5 h-5 text-purple-500" />
        </div>
        <div class="text-2xl font-bold text-gray-900 dark:text-white">{{ instances.reduce((sum, i) => sum + i.messagesExchanged, 0) }}</div>
        <p class="text-xs text-muted-foreground mt-1">Total Exchanged</p>
      </div>
    </div>

    <!-- Instances Grid -->
    <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
      <div 
        v-for="instance in instances" 
        :key="instance.id"
        class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow hover:border-cyan-200 dark:hover:border-cyan-800 transition-colors cursor-pointer"
        @click="selectedInstance = instance"
      >
        <CardHeader class="pb-3 pt-5 px-5">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div :class="['w-10 h-10 rounded-xl flex items-center justify-center', instance.type === 'native' ? 'bg-gradient-to-br from-green-500 to-emerald-500' : instance.type === 'openclaw' ? 'bg-gradient-to-br from-purple-500 to-pink-500' : 'bg-gradient-to-br from-blue-500 to-cyan-500']">
                <component :is="instance.type === 'native' ? Server : instance.type === 'openclaw' ? Terminal : Globe" class="w-5 h-5 text-white" />
              </div>
              <div>
                <CardTitle class="text-base">{{ instance.name }}</CardTitle>
                <CardDescription class="text-xs">{{ instance.type.toUpperCase() }}</CardDescription>
              </div>
            </div>
            <div :class="['w-3 h-3 rounded-full', statusColors[instance.status]]" />
          </div>
        </CardHeader>
        <CardContent class="px-5 pb-5 space-y-3">
          <div class="flex items-center justify-between text-sm">
            <span class="text-muted-foreground">Endpoint</span>
            <code class="text-xs bg-gray-100 dark:bg-white/10 px-2 py-1 rounded">{{ instance.endpoint }}</code>
          </div>
          <div class="flex items-center justify-between text-sm">
            <span class="text-muted-foreground">Protocol</span>
            <Badge variant="outline" class="rounded-lg text-xs">{{ instance.protocol }}</Badge>
          </div>
          <div class="flex items-center justify-between text-sm">
            <span class="text-muted-foreground">Uptime</span>
            <span class="font-medium">{{ formatUptime(instance.uptime) }}</span>
          </div>
          <div class="flex items-center justify-between text-sm">
            <span class="text-muted-foreground">Last Ping</span>
            <span class="font-medium">{{ formatLastPing(instance.lastPing) }}</span>
          </div>
          <div class="pt-3 border-t border-gray-100 dark:border-white/10">
            <div class="flex items-center justify-between text-sm">
              <div class="flex items-center gap-2">
                <component :is="instance.quicEnabled ? Wifi : WifiOff" :class="['w-4 h-4', instance.quicEnabled ? 'text-green-500' : 'text-gray-400']" />
                <span class="text-muted-foreground">QUIC</span>
              </div>
              <Badge :class="instance.pqReady ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 rounded-lg' : 'bg-gray-100 text-gray-500 rounded-lg'">
                <Key class="w-3 h-3 mr-1" />
                {{ instance.pqReady ? 'PQ-Ready' : 'Standard' }}
              </Badge>
            </div>
          </div>
        </CardContent>
      </div>
    </div>

    <!-- Selected Instance Detail -->
    <div v-if="selectedInstance" class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
      <CardHeader class="pb-4 pt-5 px-5">
        <div class="flex items-center justify-between">
          <CardTitle class="flex items-center gap-2">
            <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-cyan-500 to-blue-500 flex items-center justify-center">
              <Users class="w-4 h-4 text-white" />
            </div>
            Instance Details
          </CardTitle>
          <Badge :class="selectedInstance.status === 'connected' ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 rounded-full' : 'bg-gray-100 text-gray-500 rounded-full'">
            {{ selectedInstance.status.toUpperCase() }}
          </Badge>
        </div>
      </CardHeader>
      <CardContent class="px-5 pb-5 space-y-4">
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div class="p-4 rounded-xl bg-gray-50 dark:bg-white/5">
            <p class="text-xs text-muted-foreground mb-1">Tasks Processed</p>
            <p class="text-xl font-bold">{{ selectedInstance.tasksProcessed }}</p>
          </div>
          <div class="p-4 rounded-xl bg-gray-50 dark:bg-white/5">
            <p class="text-xs text-muted-foreground mb-1">Messages Exchanged</p>
            <p class="text-xl font-bold">{{ selectedInstance.messagesExchanged }}</p>
          </div>
          <div class="p-4 rounded-xl bg-gray-50 dark:bg-white/5">
            <p class="text-xs text-muted-foreground mb-1">Encryption</p>
            <p class="text-sm font-bold">{{ selectedInstance.encryption }}</p>
          </div>
          <div class="p-4 rounded-xl bg-gray-50 dark:bg-white/5">
            <p class="text-xs text-muted-foreground mb-1">Post-Quantum</p>
            <div class="flex items-center gap-2">
              <component :is="selectedInstance.pqReady ? CheckCircle : XCircle" :class="['w-5 h-5', selectedInstance.pqReady ? 'text-green-500' : 'text-gray-400']" />
              <span class="font-bold">{{ selectedInstance.pqReady ? 'Ready' : 'Pending' }}</span>
            </div>
          </div>
        </div>
        
        <div class="flex gap-2 pt-4">
          <Button 
            variant="outline" 
            class="rounded-xl gap-2"
            @click="pingInstance(selectedInstance)"
            :disabled="selectedInstance.status !== 'connected'"
          >
            <Activity class="w-4 h-4" />
            Ping
          </Button>
          <Button 
            class="rounded-xl gap-2 bg-gradient-to-r from-cyan-500 to-blue-500 hover:from-cyan-600 hover:to-blue-600 border-0"
            @click="syncInstance(selectedInstance)"
            :disabled="selectedInstance.status !== 'connected' || sendingTask"
          >
            <Send class="w-4 h-4" />
            {{ sendingTask ? 'Syncing...' : 'Sync State' }}
          </Button>
        </div>

        <div v-if="taskResult" class="rounded-xl bg-gray-900 dark:bg-black text-green-400 font-mono text-xs p-4 max-h-40 overflow-auto">
          {{ taskResult }}
        </div>
      </CardContent>
    </div>
  </div>
</template>
