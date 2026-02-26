<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import Button from '@/components/ui/button.vue'
import Badge from '@/components/ui/badge.vue'
import Textarea from '@/components/ui/textarea.vue'
import {
  Send, Bot, User, Loader2, Trash2, AlertCircle,
  Copy, Check, RotateCcw, Settings, Sparkles, Brain,
  ChevronDown, Eye, EyeOff, Clock, Hash, Plus,
  MessageSquare, ChevronRight, X
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface Message {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  streamingContent?: string
  isStreaming?: boolean
  timestamp: Date
  copied?: boolean
  tokenCount?: number
  thoughts?: ThoughtStep[]
  showThoughts?: boolean
}

interface ThoughtStep {
  type: 'thinking' | 'tool_call' | 'tool_result' | 'decision'
  content: string
}

interface Conversation {
  id: string
  title: string
  lastMessage: string
  timestamp: Date
  messageCount: number
}

const messages = ref<Message[]>([])
const inputMessage = ref('')
const isLoading = ref(false)
const messagesContainer = ref<HTMLElement | null>(null)
const housakyInstalled = ref(true)
const error = ref('')
const showSettings = ref(false)
const showSidebar = ref(false)
const selectedModel = ref('default')
const temperature = ref(0.7)
const systemPrompt = ref('')
const totalTokensSession = ref(0)
const streamingThoughts = ref<ThoughtStep[]>([])
const showThoughtsGlobal = ref(true)

const conversations = ref<Conversation[]>([
  { id: '1', title: 'Dashboard improvements', lastMessage: 'Added AGI pipeline view…', timestamp: new Date(Date.now() - 60000 * 5), messageCount: 12 },
  { id: '2', title: 'Rust async patterns', lastMessage: 'tokio::spawn vs async fn', timestamp: new Date(Date.now() - 60000 * 30), messageCount: 8 },
  { id: '3', title: 'Hardware config help', lastMessage: 'ESP32 pinout mapping', timestamp: new Date(Date.now() - 3600000), messageCount: 5 },
])

const availableModels = [
  { id: 'default', label: 'Default (from config)' },
  { id: 'claude-3-5-sonnet', label: 'Claude 3.5 Sonnet' },
  { id: 'gpt-4o', label: 'GPT-4o' },
  { id: 'gemini-2.0-flash', label: 'Gemini 2.0 Flash' },
  { id: 'deepseek-v3', label: 'DeepSeek V3' },
  { id: 'ollama/llama3', label: 'Llama 3 (local)' },
]

const quickPrompts = [
  { label: '💡 Explain', prompt: 'Explain how this works: ' },
  { label: '🐛 Debug', prompt: 'Help me debug this issue: ' },
  { label: '🧪 Tests', prompt: 'Write unit tests for: ' },
  { label: '⚡ Optimize', prompt: 'Optimize this code: ' },
  { label: '🔒 Security', prompt: 'Review security of: ' },
  { label: '📡 Hardware', prompt: 'List connected hardware devices' },
]

const messageCount = computed(() => messages.value.filter(m => m.role !== 'system').length)

let thinkingInterval: number | null = null

const sampleThoughts: ThoughtStep[] = [
  { type: 'thinking', content: 'Analyzing the user request and determining best approach…' },
  { type: 'tool_call', content: 'search_memory("relevant context")' },
  { type: 'tool_result', content: 'Found 3 relevant memories. Context assembled.' },
  { type: 'decision', content: 'Responding with retrieved context and reasoning.' },
]

async function checkHousaky() {
  if (!isTauri) { housakyInstalled.value = false; return }
  try {
    housakyInstalled.value = await invoke<boolean>('check_housaky_installed')
  } catch { housakyInstalled.value = false }
}

async function loadConversations() {
  if (!isTauri) return
  try {
    const convs = await invoke<any[]>('get_conversations')
    if (convs && convs.length) {
      conversations.value = convs.map((c: any) => ({
        id: c.id,
        title: c.title ?? 'Untitled',
        lastMessage: c.last_message ?? '',
        timestamp: new Date(c.timestamp ?? Date.now()),
        messageCount: c.message_count ?? 0,
      }))
    }
  } catch { /* fallback to sample */ }
}

async function sendMessage() {
  const text = inputMessage.value.trim()
  if (!text || isLoading.value) return

  // Allow sending even if not installed (show error gracefully)
  const userMsg: Message = {
    id: Date.now().toString(),
    role: 'user',
    content: text,
    timestamp: new Date(),
    tokenCount: Math.floor(text.length / 4),
  }
  messages.value.push(userMsg)
  inputMessage.value = ''
  isLoading.value = true
  error.value = ''
  streamingThoughts.value = []

  await nextTick()
  scrollToBottom()

  if (!isTauri) {
    await simulateStream('Running in web mode — connect Tauri for full AI capabilities.')
    isLoading.value = false
    return
  }

  // Start thought simulation while waiting
  let thoughtIdx = 0
  thinkingInterval = window.setInterval(() => {
    if (thoughtIdx < sampleThoughts.length) {
      streamingThoughts.value.push(sampleThoughts[thoughtIdx])
      thoughtIdx++
    }
  }, 600)

  try {
    const response = await invoke<string>('send_message', { message: text })

    clearInterval(thinkingInterval!)
    thinkingInterval = null

    const captured = [...streamingThoughts.value]
    streamingThoughts.value = []

    const assistantMsg: Message = {
      id: (Date.now() + 1).toString(),
      role: 'assistant',
      content: '',
      isStreaming: true,
      timestamp: new Date(),
      tokenCount: 0,
      thoughts: showThoughtsGlobal.value ? captured : [],
      showThoughts: false,
    }
    messages.value.push(assistantMsg)
    await simulateStream(response, assistantMsg.id)
  } catch (e) {
    clearInterval(thinkingInterval!)
    thinkingInterval = null
    error.value = String(e)
    messages.value.push({
      id: (Date.now() + 1).toString(),
      role: 'system',
      content: `⚠️ ${String(e)}`,
      timestamp: new Date(),
    })
  } finally {
    isLoading.value = false
    await nextTick()
    scrollToBottom()
  }
}

async function simulateStream(text: string, msgId?: string) {
  if (!msgId) {
    // web mode fallback
    messages.value.push({
      id: (Date.now() + 1).toString(),
      role: 'assistant',
      content: text,
      timestamp: new Date(),
      tokenCount: Math.floor(text.length / 4),
    })
    return
  }
  const msg = messages.value.find(m => m.id === msgId)
  if (!msg) return
  const words = text.split(' ')
  for (let i = 0; i < words.length; i++) {
    msg.content += (i > 0 ? ' ' : '') + words[i]
    msg.tokenCount = Math.floor(msg.content.length / 4)
    totalTokensSession.value += 1
    if (i % 3 === 0) {
      await nextTick()
      scrollToBottom()
      await new Promise(r => setTimeout(r, 18))
    }
  }
  msg.isStreaming = false
}

function scrollToBottom() {
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
  }
}

function clearChat() {
  messages.value = []
  totalTokensSession.value = 0
  addWelcomeMessage()
}

function addWelcomeMessage() {
  messages.value.push({
    id: '0',
    role: 'system',
    content: 'Welcome to Housaky Chat! I\'m your AGI assistant. Ask me anything — code, hardware, skills, or strategy.',
    timestamp: new Date(),
  })
}

function formatTime(d: Date) {
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function formatRelative(d: Date): string {
  const diff = (Date.now() - d.getTime()) / 1000
  if (diff < 60) return 'just now'
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`
  return d.toLocaleDateString()
}

function copyMessage(msg: Message) {
  navigator.clipboard.writeText(msg.content)
  msg.copied = true
  setTimeout(() => { msg.copied = false }, 2000)
}

function retryLastMessage() {
  const last = [...messages.value].reverse().find(m => m.role === 'user')
  if (last) {
    messages.value = messages.value.filter(m => m.id !== last.id)
    inputMessage.value = last.content
  }
}

function useQuickPrompt(prompt: string) {
  inputMessage.value = prompt
}

function sanitizeDisplay(content: string): string {
  return content
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
}

function renderContent(content: string): string {
  const safe = sanitizeDisplay(content)
  return safe
    .replace(/`([^`]+)`/g, '<code class="bg-muted px-1 rounded text-xs font-mono">$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/\*([^*]+)\*/g, '<em>$1</em>')
    .replace(/\n/g, '<br>')
}

const thoughtTypeConfig: Record<string, { label: string; color: string }> = {
  thinking: { label: '💭', color: 'text-purple-600' },
  tool_call: { label: '🔧', color: 'text-blue-600' },
  tool_result: { label: '✓', color: 'text-green-600' },
  decision: { label: '⚡', color: 'text-orange-600' },
}

onMounted(async () => {
  await checkHousaky()
  await loadConversations()
  addWelcomeMessage()
})

onUnmounted(() => {
  if (thinkingInterval) clearInterval(thinkingInterval)
})
</script>

<template>
  <div class="h-[calc(100vh-57px)] flex overflow-hidden">
    <!-- Conversation Sidebar -->
    <div :class="['flex-shrink-0 border-r bg-card flex flex-col transition-all duration-300 overflow-hidden', showSidebar ? 'w-64' : 'w-0']">
      <div class="p-3 border-b flex items-center justify-between flex-shrink-0">
        <span class="text-sm font-semibold">Conversations</span>
        <Button variant="ghost" size="sm" class="h-7 w-7 p-0" @click="clearChat">
          <Plus class="w-3.5 h-3.5" />
        </Button>
      </div>
      <div class="flex-1 overflow-y-auto p-2 space-y-1">
        <button v-for="conv in conversations" :key="conv.id"
          class="w-full text-left p-2.5 rounded-lg hover:bg-muted transition-colors group"
        >
          <p class="text-xs font-medium truncate">{{ conv.title }}</p>
          <p class="text-[10px] text-muted-foreground truncate mt-0.5">{{ conv.lastMessage }}</p>
          <p class="text-[10px] text-muted-foreground mt-1">{{ formatRelative(conv.timestamp) }} · {{ conv.messageCount }}m</p>
        </button>
      </div>
    </div>

    <!-- Main Chat Area -->
    <div class="flex-1 flex flex-col min-w-0">
      <!-- Chat Header -->
      <div class="flex items-center justify-between px-4 py-3 border-b bg-card flex-shrink-0">
        <div class="flex items-center gap-3">
          <Button variant="ghost" size="sm" class="h-8 w-8 p-0" @click="showSidebar = !showSidebar">
            <MessageSquare class="w-4 h-4" />
          </Button>
          <div>
            <div class="flex items-center gap-2">
              <Brain class="w-4 h-4 text-purple-500" />
              <span class="font-semibold text-sm">Housaky Assistant</span>
              <Badge :class="housakyInstalled ? 'bg-green-500/10 text-green-600 border-green-200 text-[10px]' : 'bg-red-500/10 text-red-600 border-red-200 text-[10px]'">
                <span :class="['w-1.5 h-1.5 rounded-full mr-1 inline-block', housakyInstalled ? 'bg-green-500 animate-pulse' : 'bg-red-500']" />
                {{ housakyInstalled ? 'Live' : 'Offline' }}
              </Badge>
            </div>
            <p class="text-[10px] text-muted-foreground">{{ messageCount }} messages · {{ totalTokensSession.toLocaleString() }} tokens</p>
          </div>
        </div>
        <div class="flex items-center gap-1.5">
          <Button variant="ghost" size="sm" class="h-8 gap-1.5 text-xs" @click="showThoughtsGlobal = !showThoughtsGlobal" :title="showThoughtsGlobal ? 'Hide thoughts' : 'Show thoughts'">
            <component :is="showThoughtsGlobal ? Eye : EyeOff" class="w-3.5 h-3.5" />
            Thoughts
          </Button>
          <Button variant="ghost" size="sm" class="h-8" @click="showSettings = !showSettings">
            <Settings class="w-3.5 h-3.5" />
          </Button>
          <Button variant="ghost" size="sm" class="h-8" @click="clearChat">
            <Trash2 class="w-3.5 h-3.5" />
          </Button>
        </div>
      </div>

      <!-- Settings Drawer -->
      <div v-if="showSettings" class="border-b bg-muted/30 px-4 py-3 flex-shrink-0">
        <div class="flex items-start gap-6 flex-wrap">
          <div class="space-y-1">
            <label class="text-xs font-medium text-muted-foreground">Model</label>
            <select v-model="selectedModel" class="h-8 rounded-md border bg-background px-2 text-sm">
              <option v-for="m in availableModels" :key="m.id" :value="m.id">{{ m.label }}</option>
            </select>
          </div>
          <div class="space-y-1">
            <label class="text-xs font-medium text-muted-foreground">Temperature: {{ temperature }}</label>
            <input type="range" v-model="temperature" min="0" max="2" step="0.1" class="w-32 h-2" />
          </div>
          <div class="flex-1 space-y-1 min-w-48">
            <label class="text-xs font-medium text-muted-foreground">System Prompt</label>
            <input v-model="systemPrompt" class="w-full h-8 rounded-md border bg-background px-2 text-xs" placeholder="Optional system prompt…" />
          </div>
          <Button variant="ghost" size="sm" class="h-8 w-8 p-0 mt-4" @click="showSettings = false">
            <X class="w-3.5 h-3.5" />
          </Button>
        </div>
      </div>

      <!-- Not installed warning -->
      <div v-if="!housakyInstalled" class="mx-4 mt-3 p-3 rounded-lg border border-yellow-400/50 bg-yellow-50/80 dark:bg-yellow-900/20 flex items-center gap-2 flex-shrink-0">
        <AlertCircle class="w-4 h-4 text-yellow-600 flex-shrink-0" />
        <span class="text-sm text-yellow-700 dark:text-yellow-400">Housaky not installed — AI responses unavailable</span>
      </div>

      <!-- Messages -->
      <div class="flex-1 overflow-y-auto p-4 space-y-4" ref="messagesContainer">
        <div v-for="message in messages" :key="message.id"
          :class="['flex gap-3', message.role === 'user' ? 'justify-end' : 'justify-start']"
        >
          <div :class="['flex gap-3 max-w-[85%]', message.role === 'user' ? 'flex-row-reverse' : '']">
            <!-- Avatar -->
            <div :class="[
              'w-7 h-7 rounded-full flex items-center justify-center flex-shrink-0 mt-0.5',
              message.role === 'user' ? 'bg-primary' :
              message.role === 'system' ? 'bg-amber-500' :
              'bg-gradient-to-br from-blue-500 to-purple-600'
            ]">
              <User v-if="message.role === 'user'" class="w-3.5 h-3.5 text-primary-foreground" />
              <Brain v-else-if="message.role === 'assistant'" class="w-3.5 h-3.5 text-white" />
              <AlertCircle v-else class="w-3.5 h-3.5 text-white" />
            </div>

            <div class="flex flex-col gap-1">
              <!-- Thought steps -->
              <div v-if="message.thoughts && message.thoughts.length > 0 && showThoughtsGlobal" class="mb-1">
                <button @click="message.showThoughts = !message.showThoughts"
                  class="flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground transition-colors"
                >
                  <Eye class="w-3 h-3" />
                  {{ message.showThoughts ? 'Hide' : 'Show' }} {{ message.thoughts.length }} reasoning steps
                  <ChevronDown :class="['w-3 h-3 transition-transform', message.showThoughts ? 'rotate-180' : '']" />
                </button>
                <div v-if="message.showThoughts" class="mt-1 space-y-1 pl-2 border-l-2 border-muted">
                  <div v-for="(t, i) in message.thoughts" :key="i"
                    :class="['text-[10px] font-mono', thoughtTypeConfig[t.type]?.color ?? 'text-muted-foreground']"
                  >
                    {{ thoughtTypeConfig[t.type]?.label }} {{ t.content }}
                  </div>
                </div>
              </div>

              <!-- Bubble -->
              <div :class="[
                'rounded-2xl px-4 py-2.5 leading-relaxed',
                message.role === 'user' ? 'bg-primary text-primary-foreground rounded-tr-sm' :
                message.role === 'system' ? 'bg-amber-100 dark:bg-amber-900/30 text-amber-900 dark:text-amber-200 rounded-tl-sm text-sm' :
                'bg-muted rounded-tl-sm'
              ]">
                <span v-if="message.role === 'user'" class="text-sm whitespace-pre-wrap">{{ message.content }}</span>
                <span v-else v-html="renderContent(message.content)" class="text-sm" />
                <span v-if="message.isStreaming" class="inline-block w-1 h-4 bg-current ml-0.5 animate-pulse align-middle" />
              </div>

              <!-- Meta row -->
              <div :class="['flex items-center gap-3 px-1', message.role === 'user' ? 'justify-end' : 'justify-start']">
                <span class="text-[10px] text-muted-foreground">{{ formatTime(message.timestamp) }}</span>
                <span v-if="message.tokenCount" class="text-[10px] text-muted-foreground flex items-center gap-0.5">
                  <Hash class="w-2.5 h-2.5" />{{ message.tokenCount }}t
                </span>
                <button v-if="message.role !== 'system'" @click="copyMessage(message)"
                  class="hover:opacity-100 opacity-0 group-hover:opacity-100 transition-opacity"
                >
                  <component :is="message.copied ? Check : Copy" class="w-3 h-3 text-muted-foreground" />
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Streaming thoughts (while loading) -->
        <div v-if="isLoading && streamingThoughts.length > 0" class="flex gap-3">
          <div class="w-7 h-7 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center flex-shrink-0 mt-0.5">
            <Brain class="w-3.5 h-3.5 text-white" />
          </div>
          <div class="bg-muted rounded-2xl rounded-tl-sm px-4 py-2.5 max-w-lg">
            <div class="space-y-1 mb-2">
              <div v-for="(t, i) in streamingThoughts" :key="i"
                :class="['text-[10px] font-mono', thoughtTypeConfig[t.type]?.color ?? 'text-muted-foreground']"
              >
                {{ thoughtTypeConfig[t.type]?.label }} {{ t.content }}
              </div>
            </div>
            <div class="flex items-center gap-2 text-xs text-muted-foreground">
              <Loader2 class="w-3.5 h-3.5 animate-spin" />
              Reasoning…
            </div>
          </div>
        </div>

        <div v-else-if="isLoading" class="flex gap-3">
          <div class="w-7 h-7 rounded-full bg-gradient-to-br from-blue-500 to-purple-600 flex items-center justify-center flex-shrink-0">
            <Brain class="w-3.5 h-3.5 text-white" />
          </div>
          <div class="bg-muted rounded-2xl rounded-tl-sm px-4 py-2.5 flex items-center gap-2">
            <Loader2 class="w-3.5 h-3.5 animate-spin" />
            <span class="text-sm text-muted-foreground">Thinking…</span>
          </div>
        </div>
      </div>

      <!-- Quick Prompts -->
      <div class="px-4 py-2 border-t bg-muted/20 flex-shrink-0">
        <div class="flex gap-1.5 flex-wrap">
          <button v-for="qp in quickPrompts" :key="qp.label"
            @click="useQuickPrompt(qp.prompt)"
            class="text-[11px] px-2.5 py-1 rounded-full bg-muted hover:bg-muted/80 border hover:border-primary/30 transition-all"
          >
            {{ qp.label }}
          </button>
        </div>
      </div>

      <!-- Input -->
      <div class="px-4 pb-4 pt-2 border-t flex-shrink-0">
        <div class="flex gap-2 items-end">
          <Textarea
            v-model="inputMessage"
            placeholder="Message Housaky… (Enter to send, Shift+Enter for new line)"
            :disabled="isLoading"
            class="flex-1 min-h-[42px] max-h-36 resize-none text-sm"
            @keydown.enter.exact.prevent="sendMessage"
          />
          <Button @click="sendMessage" :disabled="isLoading || !inputMessage.trim()" class="h-10 w-10 p-0 flex-shrink-0">
            <Send class="w-4 h-4" />
          </Button>
        </div>
        <div v-if="error" class="flex items-center justify-between mt-2 text-xs text-destructive">
          <span class="truncate">{{ error }}</span>
          <Button variant="ghost" size="sm" class="h-6 text-xs flex-shrink-0 ml-2" @click="retryLastMessage">
            <RotateCcw class="w-3 h-3 mr-1" />Retry
          </Button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.group:hover button { opacity: 1; }
</style>
