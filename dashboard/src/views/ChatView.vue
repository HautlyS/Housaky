<script setup lang="ts">
import { ref, onMounted, nextTick, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import Button from '@/components/ui/button.vue'
import Input from '@/components/ui/input.vue'
import Badge from '@/components/ui/badge.vue'
import ScrollArea from '@/components/ui/scroll-area.vue'
import Textarea from '@/components/ui/textarea.vue'
import { 
  Send, Bot, User, Loader2, Trash2, AlertCircle, 
  Copy, Check, RotateCcw, Settings, Zap, Sparkles
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface Message {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  timestamp: Date
  copied?: boolean
}

const messages = ref<Message[]>([])
const inputMessage = ref('')
const isLoading = ref(false)
const messagesContainer = ref<HTMLElement | null>(null)
const housakyInstalled = ref(true)
const error = ref('')
const temperature = ref(0.7)
const systemPrompt = ref('')
const showSettings = ref(false)

const messageCount = computed(() => 
  messages.value.filter(m => m.role !== 'system').length
)

async function checkHousaky() {
  if (!isTauri) {
    housakyInstalled.value = false
    return
  }
  try {
    housakyInstalled.value = await invoke<boolean>('check_housaky_installed')
  } catch {
    housakyInstalled.value = false
  }
}

async function sendMessage() {
  if (!inputMessage.value.trim() || isLoading.value || !housakyInstalled.value) return

  const userMessage: Message = {
    id: Date.now().toString(),
    role: 'user',
    content: inputMessage.value.trim(),
    timestamp: new Date(),
  }

  messages.value.push(userMessage)
  const messageText = inputMessage.value
  inputMessage.value = ''
  isLoading.value = true
  error.value = ''

  await nextTick()
  scrollToBottom()

  if (!isTauri) {
    error.value = 'Running in server mode - chat not available'
    isLoading.value = false
    return
  }

  try {
    const response = await invoke<string>('send_message', { 
      message: messageText 
    })

    const assistantMessage: Message = {
      id: (Date.now() + 1).toString(),
      role: 'assistant',
      content: response,
      timestamp: new Date(),
    }

    messages.value.push(assistantMessage)
  } catch (e) {
    error.value = String(e)
    const errorMessage: Message = {
      id: (Date.now() + 1).toString(),
      role: 'system',
      content: `Error: ${e}`,
      timestamp: new Date(),
    }
    messages.value.push(errorMessage)
  } finally {
    isLoading.value = false
    await nextTick()
    scrollToBottom()
  }
}

function scrollToBottom() {
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
  }
}

function clearChat() {
  messages.value = []
  addWelcomeMessage()
}

function addWelcomeMessage() {
  messages.value.push({
    id: '0',
    role: 'system',
    content: 'Welcome to Housaky! I\'m your AI assistant. I can help you with code, answer questions, control hardware, and more. How can I help you today?',
    timestamp: new Date(),
  })
}

function formatTime(date: Date) {
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function copyMessage(message: Message) {
  navigator.clipboard.writeText(message.content)
  message.copied = true
  setTimeout(() => {
    message.copied = false
  }, 2000)
}

function retryLastMessage() {
  const lastUserMessage = [...messages.value].reverse().find(m => m.role === 'user')
  if (lastUserMessage) {
    messages.value = messages.value.filter(m => m.id !== lastUserMessage.id)
    inputMessage.value = lastUserMessage.content
  }
}

const quickPrompts = [
  { label: 'Explain code', prompt: 'Explain how this code works:' },
  { label: 'Debug', prompt: 'Help me debug this issue:' },
  { label: 'Write tests', prompt: 'Write unit tests for:' },
  { label: 'Hardware', prompt: 'List connected hardware devices' },
]

function useQuickPrompt(prompt: string) {
  inputMessage.value = prompt + ' '
}

onMounted(async () => {
  await checkHousaky()
  addWelcomeMessage()
})
</script>

<template>
  <div class="h-full flex flex-col p-6">
    <div class="flex items-center justify-between mb-4">
      <div>
        <h1 class="text-2xl font-bold">Chat</h1>
        <p class="text-sm text-muted-foreground">Talk with your AI assistant</p>
      </div>
      <div class="flex items-center gap-3">
        <Badge :variant="housakyInstalled ? 'success' : 'destructive'" class="flex items-center gap-1">
          <div :class="['w-2 h-2 rounded-full', housakyInstalled ? 'bg-green-500' : 'bg-red-500']"></div>
          {{ housakyInstalled ? 'Ready' : 'Offline' }}
        </Badge>
        <Button variant="outline" size="icon" @click="showSettings = !showSettings">
          <Settings class="w-4 h-4" />
        </Button>
        <Button variant="outline" size="icon" @click="clearChat">
          <Trash2 class="w-4 h-4" />
        </Button>
      </div>
    </div>

    <!-- Settings Panel -->
    <Card v-if="showSettings" class="mb-4">
      <CardHeader class="py-3">
        <CardTitle class="text-sm">Chat Settings</CardTitle>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="grid md:grid-cols-2 gap-4">
          <div>
            <label class="text-sm font-medium mb-2 block">Temperature: {{ temperature }}</label>
            <input 
              type="range" 
              v-model="temperature" 
              min="0" 
              max="2" 
              step="0.1"
              class="w-full"
            />
            <p class="text-xs text-muted-foreground mt-1">Higher = more creative, Lower = more focused</p>
          </div>
          <div>
            <label class="text-sm font-medium mb-2 block">System Prompt</label>
            <Textarea 
              v-model="systemPrompt" 
              placeholder="Optional system prompt..."
              class="h-20"
            />
          </div>
        </div>
      </CardContent>
    </Card>

    <!-- Not installed warning -->
    <Card v-if="!housakyInstalled" class="mb-4 border-yellow-500 bg-yellow-50 dark:bg-yellow-900/20">
      <CardContent class="pt-4">
        <div class="flex items-center gap-2 text-yellow-600 dark:text-yellow-400">
          <AlertCircle class="w-5 h-5" />
          <span>Housaky is not installed. Install it to enable chat.</span>
        </div>
      </CardContent>
    </Card>

    <Card class="flex-1 flex flex-col min-h-0">
      <CardHeader class="border-b py-3">
        <div class="flex items-center justify-between">
          <CardTitle class="flex items-center gap-2 text-base">
            <Bot class="w-5 h-5" />
            Housaky Assistant
          </CardTitle>
          <div class="flex items-center gap-2 text-sm text-muted-foreground">
            <Sparkles class="w-4 h-4" />
            {{ messageCount }} messages
          </div>
        </div>
      </CardHeader>
      
      <ScrollArea class="flex-1 p-4" ref="messagesContainer">
        <div class="space-y-4">
          <div
            v-for="message in messages"
            :key="message.id"
            class="flex gap-3"
            :class="message.role === 'user' ? 'justify-end' : 'justify-start'"
          >
            <div
              class="flex gap-3 max-w-[80%]"
              :class="message.role === 'user' ? 'flex-row-reverse' : ''"
            >
              <div
                class="w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0"
                :class="message.role === 'user' ? 'bg-primary' : message.role === 'system' ? 'bg-yellow-500' : 'bg-gradient-to-br from-blue-500 to-purple-500'"
              >
                <User v-if="message.role === 'user'" class="w-4 h-4 text-primary-foreground" />
                <Bot v-else-if="message.role === 'assistant'" class="w-4 h-4 text-white" />
                <AlertCircle v-else class="w-4 h-4 text-white" />
              </div>
              
              <div class="flex flex-col gap-1">
                <div
                  class="rounded-lg p-3"
                  :class="message.role === 'user' ? 'bg-primary text-primary-foreground' : message.role === 'system' ? 'bg-yellow-100 dark:bg-yellow-900' : 'bg-muted'"
                >
                  <p class="whitespace-pre-wrap">{{ message.content }}</p>
                </div>
                <div class="flex items-center gap-2 px-1">
                  <span class="text-xs text-muted-foreground">{{ formatTime(message.timestamp) }}</span>
                  <button 
                    v-if="message.role !== 'system'"
                    @click="copyMessage(message)"
                    class="opacity-0 group-hover:opacity-100 hover:opacity-100 transition-opacity"
                  >
                    <component :is="message.copied ? Check : Copy" class="w-3 h-3 text-muted-foreground" />
                  </button>
                </div>
              </div>
            </div>
          </div>

          <div v-if="isLoading" class="flex gap-3">
            <div class="w-8 h-8 rounded-full bg-gradient-to-br from-blue-500 to-purple-500 flex items-center justify-center">
              <Bot class="w-4 h-4 text-white" />
            </div>
            <div class="rounded-lg p-3 bg-muted">
              <div class="flex items-center gap-2">
                <Loader2 class="w-4 h-4 animate-spin" />
                <span class="text-muted-foreground">Thinking...</span>
              </div>
            </div>
          </div>
        </div>
      </ScrollArea>

      <!-- Quick Prompts -->
      <div class="px-4 py-2 border-t bg-muted/30">
        <div class="flex flex-wrap gap-2">
          <button
            v-for="qp in quickPrompts"
            :key="qp.label"
            @click="useQuickPrompt(qp.prompt)"
            class="text-xs px-3 py-1 rounded-full bg-muted hover:bg-muted/80 transition-colors"
          >
            {{ qp.label }}
          </button>
        </div>
      </div>

      <!-- Input -->
      <div class="p-4 border-t">
        <form @submit.prevent="sendMessage" class="flex gap-2">
          <Textarea
            v-model="inputMessage"
            placeholder="Type your message... (Shift+Enter for new line)"
            :disabled="isLoading || !housakyInstalled"
            class="min-h-[44px] max-h-32 resize-none"
            @keydown.enter.exact.prevent="sendMessage"
          />
          <Button type="submit" :disabled="isLoading || !inputMessage.trim() || !housakyInstalled" class="h-auto">
            <Send class="w-4 h-4" />
          </Button>
        </form>
        <div v-if="error" class="flex items-center justify-between mt-2 text-sm text-destructive">
          <span>{{ error }}</span>
          <Button variant="ghost" size="sm" @click="retryLastMessage">
            <RotateCcw class="w-3 h-3 mr-1" />
            Retry
          </Button>
        </div>
      </div>
    </Card>
  </div>
</template>

<style scoped>
.group:hover .opacity-0 {
  opacity: 1;
}
</style>
