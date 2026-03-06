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
import Textarea from '@/components/ui/textarea.vue'
import { 
  MessageSquare, Send, Inbox, Outbox, RefreshCw, Trash2,
  CheckCircle, XCircle, Clock, ArrowRight, Filter,
  Zap, Brain, BookOpen, Code, ListChecks, HeartPulse,
  Target, Shield, Activity, Lightbulb
} from 'lucide-vue-next'

interface A2AMessage {
  id: string
  type: 'Task' | 'TaskResult' | 'Context' | 'Learning' | 'CodeImprove' | 'SyncRequest' | 'SyncResponse' | 'Ping' | 'Pong' | 'Metrics' | 'GoalStatus' | 'Stop'
  from: string
  to: string
  priority: 0 | 1 | 2 | 3
  timestamp: number
  payload: Record<string, unknown>
  processed: boolean
  direction: 'inbox' | 'outbox'
}

const messages = ref<A2AMessage[]>([])
const loading = ref(true)
const filter = ref<'all' | 'inbox' | 'outbox'>('all')
const selectedMessage = ref<A2AMessage | null>(null)
const composingMessage = ref(false)
const newMessage = ref('')
const sendLoading = ref(false)

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const messageTypeIcons: Record<string, typeof Brain> = {
  Task: Brain,
  TaskResult: CheckCircle,
  Context: BookOpen,
  Learning: Lightbulb,
  CodeImprove: Code,
  SyncRequest: RefreshCw,
  SyncResponse: RefreshCw,
  Ping: Zap,
  Pong: Zap,
  Metrics: Activity,
  GoalStatus: Target,
  Stop: Shield
}

const priorityLabels: Record<number, string> = {
  0: 'CRITICAL',
  1: 'HIGH',
  2: 'NORMAL',
  3: 'LOW'
}

const priorityColors: Record<number, string> = {
  0: 'bg-red-500',
  1: 'bg-orange-500',
  2: 'bg-blue-500',
  3: 'bg-gray-400'
}

const filteredMessages = computed(() => {
  if (filter.value === 'all') return messages.value
  return messages.value.filter(m => m.direction === filter.value)
})

async function loadMessages() {
  loading.value = true
  try {
    if (isTauri) {
      const result = await invoke<A2AMessage[]>('get_a2a_messages')
      messages.value = result
    } else {
      messages.value = [
        {
          id: 'msg-001',
          type: 'Task',
          from: 'openclaw',
          to: 'native',
          priority: 2,
          timestamp: Date.now() - 60000,
          payload: { action: 'analyze', params: { file: 'src/core.rs' } },
          processed: true,
          direction: 'inbox'
        },
        {
          id: 'msg-002',
          type: 'TaskResult',
          from: 'native',
          to: 'openclaw',
          priority: 2,
          timestamp: Date.now() - 50000,
          payload: { taskId: 'task-1', result: { findings: ['bottleneck in function X'], confidence: 0.85 }, success: true },
          processed: true,
          direction: 'outbox'
        },
        {
          id: 'msg-003',
          type: 'Learning',
          from: 'openclaw',
          to: 'native',
          priority: 1,
          timestamp: Date.now() - 120000,
          payload: { category: 'optimization', content: 'Use Cow<str> for zero-copy operations', confidence: 0.92 },
          processed: false,
          direction: 'inbox'
        },
        {
          id: 'msg-004',
          type: 'Ping',
          from: 'native',
          to: 'openclaw',
          priority: 3,
          timestamp: Date.now() - 180000,
          payload: {},
          processed: true,
          direction: 'outbox'
        },
        {
          id: 'msg-005',
          type: 'Metrics',
          from: 'openclaw',
          to: 'native',
          priority: 3,
          timestamp: Date.now() - 240000,
          payload: { cpu: 45.2, memory: 2.1, tasksDone: 150, errors: 3 },
          processed: true,
          direction: 'inbox'
        },
        {
          id: 'msg-006',
          type: 'GoalStatus',
          from: 'openclaw',
          to: 'native',
          priority: 2,
          timestamp: Date.now() - 300000,
          payload: { goals: [{ id: 'g1', title: 'Improve speed', progress: 0.7, status: 'active' }] },
          processed: false,
          direction: 'inbox'
        }
      ]
    }
  } catch (e) {
    console.error('Failed to load messages:', e)
  } finally {
    loading.value = false
  }
}

async function processMessage(msg: A2AMessage) {
  if (!isTauri || msg.processed) return
  try {
    await invoke('process_a2a_message', { messageId: msg.id })
    msg.processed = true
  } catch (e) {
    console.error('Failed to process message:', e)
  }
}

async function sendMessage() {
  if (!newMessage.value.trim() || sendLoading.value) return
  sendLoading.value = true
  try {
    await invoke('send_a2a_message', { message: newMessage.value })
    newMessage.value = ''
    composingMessage.value = false
    await loadMessages()
  } catch (e) {
    console.error('Failed to send message:', e)
  } finally {
    sendLoading.value = false
  }
}

function formatTime(timestamp: number): string {
  return new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

function formatPayload(payload: Record<string, unknown>): string {
  return JSON.stringify(payload, null, 2)
}

onMounted(() => {
  loadMessages()
})
</script>

<template>
  <div class="space-y-6 max-w-7xl mx-auto">
    <!-- Header -->
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div class="flex items-center gap-4">
        <div class="w-14 h-14 rounded-2xl bg-gradient-to-br from-violet-500 via-purple-500 to-fuchsia-500 flex items-center justify-center shadow-lg shadow-purple-500/30">
          <MessageSquare class="w-7 h-7 text-white" />
        </div>
        <div>
          <h1 class="text-2xl font-bold text-gray-900 dark:text-white">A2A Messages</h1>
          <p class="text-sm text-muted-foreground">Agent-to-Agent Protocol Messages</p>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <div class="flex bg-gray-100 dark:bg-white/10 rounded-lg p-1">
          <button 
            v-for="f in ['all', 'inbox', 'outbox']" 
            :key="f"
            :class="[
              'px-3 py-1 rounded-md text-xs font-medium transition-colors',
              filter === f ? 'bg-white dark:bg-white/20 shadow-sm' : 'text-muted-foreground hover:text-foreground'
            ]"
            @click="filter = f as 'all' | 'inbox' | 'outbox'"
          >
            {{ f === 'inbox' ? 'Inbox' : f === 'outbox' ? 'Outbox' : 'All' }}
          </button>
        </div>
        <Button variant="outline" size="sm" class="rounded-full" @click="loadMessages" :disabled="loading">
          <RefreshCw :class="['w-3.5 h-3.5', loading && 'animate-spin']" />
        </Button>
      </div>
    </div>

    <!-- Stats -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <div class="rounded-2xl p-5 bg-gradient-to-br from-violet-500/10 to-purple-500/10 border border-gray-200/50 dark:border-white/10">
        <div class="flex items-start justify-between mb-3">
          <span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Total</span>
          <Inbox class="w-5 h-5 text-violet-500" />
        </div>
        <div class="text-2xl font-bold">{{ messages.length }}</div>
        <p class="text-xs text-muted-foreground mt-1">Messages</p>
      </div>

      <div class="rounded-2xl p-5 bg-gradient-to-br from-green-500/10 to-emerald-500/10 border border-gray-200/50 dark:border-white/10">
        <div class="flex items-start justify-between mb-3">
          <span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Processed</span>
          <CheckCircle class="w-5 h-5 text-green-500" />
        </div>
        <div class="text-2xl font-bold text-green-600 dark:text-green-400">{{ messages.filter(m => m.processed).length }}</div>
        <p class="text-xs text-muted-foreground mt-1">Completed</p>
      </div>

      <div class="rounded-2xl p-5 bg-gradient-to-br from-amber-500/10 to-orange-500/10 border border-gray-200/50 dark:border-white/10">
        <div class="flex items-start justify-between mb-3">
          <span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Pending</span>
          <Clock class="w-5 h-5 text-amber-500" />
        </div>
        <div class="text-2xl font-bold text-amber-600 dark:text-amber-400">{{ messages.filter(m => !m.processed).length }}</div>
        <p class="text-xs text-muted-foreground mt-1">Awaiting</p>
      </div>

      <div class="rounded-2xl p-5 bg-gradient-to-br from-blue-500/10 to-cyan-500/10 border border-gray-200/50 dark:border-white/10">
        <div class="flex items-start justify-between mb-3">
          <span class="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Critical</span>
          <Zap class="w-5 h-5 text-blue-500" />
        </div>
        <div class="text-2xl font-bold text-blue-600 dark:text-blue-400">{{ messages.filter(m => m.priority === 0).length }}</div>
        <p class="text-xs text-muted-foreground mt-1">High Priority</p>
      </div>
    </div>

    <!-- Messages List -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <div class="lg:col-span-2 rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
        <CardHeader class="pb-3 pt-5 px-5">
          <CardTitle class="flex items-center gap-2 text-base">
            <MessageSquare class="w-4 h-4" />
            Message Queue
          </CardTitle>
        </CardHeader>
        <CardContent class="px-5 pb-5">
          <div class="space-y-2 max-h-[500px] overflow-y-auto">
            <div v-if="!filteredMessages.length" class="text-sm text-muted-foreground text-center py-8">
              No messages
            </div>
            <div 
              v-for="msg in filteredMessages" 
              :key="msg.id"
              :class="[
                'p-4 rounded-xl border transition-colors cursor-pointer',
                selectedMessage?.id === msg.id 
                  ? 'border-violet-300 dark:border-violet-700 bg-violet-50 dark:bg-violet-900/20' 
                  : 'border-gray-100 dark:border-white/10 hover:bg-gray-50 dark:hover:bg-white/5'
              ]"
              @click="selectedMessage = msg"
            >
              <div class="flex items-start justify-between gap-2">
                <div class="flex items-center gap-2">
                  <div :class="['w-2 h-2 rounded-full flex-shrink-0', priorityColors[msg.priority]]" />
                  <span class="font-medium text-sm">{{ msg.type }}</span>
                </div>
                <span class="text-xs text-muted-foreground">{{ formatTime(msg.timestamp) }}</span>
              </div>
              <div class="flex items-center gap-2 mt-2 text-xs text-muted-foreground">
                <span>{{ msg.from }}</span>
                <ArrowRight class="w-3 h-3" />
                <span>{{ msg.to }}</span>
              </div>
              <div class="flex items-center gap-2 mt-2">
                <Badge :class="msg.processed ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 rounded-lg text-xs' : 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400 rounded-lg text-xs'">
                  {{ msg.processed ? 'Processed' : 'Pending' }}
                </Badge>
                <Badge variant="outline" class="rounded-lg text-xs">
                  {{ priorityLabels[msg.priority] }}
                </Badge>
                <Badge :class="msg.direction === 'inbox' ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400 rounded-lg text-xs' : 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400 rounded-lg text-xs'">
                  {{ msg.direction.toUpperCase() }}
                </Badge>
              </div>
            </div>
          </div>
        </CardContent>
      </div>

      <!-- Message Detail -->
      <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
        <CardHeader class="pb-3 pt-5 px-5">
          <CardTitle class="flex items-center gap-2 text-base">
            <component :is="messageTypeIcons[selectedMessage?.type || 'Task']" class="w-4 h-4" />
            Message Details
          </CardTitle>
        </CardHeader>
        <CardContent class="px-5 pb-5">
          <div v-if="!selectedMessage" class="text-sm text-muted-foreground text-center py-8">
            Select a message to view details
          </div>
          <div v-else class="space-y-4">
            <div class="p-4 rounded-xl bg-gray-50 dark:bg-white/5 space-y-3">
              <div class="flex items-center justify-between">
                <span class="text-xs text-muted-foreground">Type</span>
                <Badge class="rounded-lg">{{ selectedMessage.type }}</Badge>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-xs text-muted-foreground">Priority</span>
                <Badge :class="['rounded-lg', priorityColors[selectedMessage.priority] + ' text-white']">
                  {{ priorityLabels[selectedMessage.priority] }}
                </Badge>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-xs text-muted-foreground">From</span>
                <code class="text-xs bg-gray-100 dark:bg-white/10 px-2 py-1 rounded">{{ selectedMessage.from }}</code>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-xs text-muted-foreground">To</span>
                <code class="text-xs bg-gray-100 dark:bg-white/10 px-2 py-1 rounded">{{ selectedMessage.to }}</code>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-xs text-muted-foreground">ID</span>
                <code class="text-xs bg-gray-100 dark:bg-white/10 px-2 py-1 rounded">{{ selectedMessage.id }}</code>
              </div>
            </div>

            <div>
              <p class="text-xs text-muted-foreground mb-2">Payload</p>
              <pre class="rounded-xl bg-gray-900 dark:bg-black text-green-400 font-mono text-xs p-4 max-h-48 overflow-auto">{{ formatPayload(selectedMessage.payload) }}</pre>
            </div>

            <Button 
              v-if="!selectedMessage.processed"
              class="w-full rounded-xl gap-2"
              @click="processMessage(selectedMessage)"
            >
              <CheckCircle class="w-4 h-4" />
              Process Message
            </Button>
            <div v-else class="text-center text-sm text-green-600 dark:text-green-400">
              <CheckCircle class="w-4 h-4 inline mr-1" />
              Already Processed
            </div>
          </div>
        </CardContent>
      </div>
    </div>

    <!-- Quick Compose -->
    <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
      <CardHeader class="pb-3 pt-5 px-5">
        <CardTitle class="flex items-center gap-2 text-base">
          <Send class="w-4 h-4" />
          Quick Compose
        </CardTitle>
        <CardDescription>Send a quick A2A message to a connected instance</CardDescription>
      </CardHeader>
      <CardContent class="px-5 pb-5">
        <Textarea 
          v-model="newMessage"
          placeholder='{"t":"Task","d":{"id":"task-1","action":"analyze"}}'
          class="font-mono text-sm"
          rows="3"
        />
        <div class="flex justify-end mt-3">
          <Button 
            class="rounded-xl gap-2 bg-gradient-to-r from-violet-500 to-purple-500 hover:from-violet-600 hover:to-purple-600 border-0"
            @click="sendMessage"
            :disabled="!newMessage.trim() || sendLoading"
          >
            <Send class="w-4 h-4" />
            {{ sendLoading ? 'Sending...' : 'Send Message' }}
          </Button>
        </div>
      </CardContent>
    </div>
  </div>
</template>
