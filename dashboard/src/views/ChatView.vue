<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import Button from '@/components/ui/button.vue'
import Input from '@/components/ui/input.vue'
import Badge from '@/components/ui/badge.vue'
import ScrollArea from '@/components/ui/scroll-area.vue'
import { Send, Bot, User, Loader2, Trash2, AlertCircle } from 'lucide-vue-next'

interface Message {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  timestamp: Date
}

const messages = ref<Message[]>([])
const inputMessage = ref('')
const isLoading = ref(false)
const messagesContainer = ref<HTMLElement | null>(null)
const housakyInstalled = ref(true)
const error = ref('')

async function checkHousaky() {
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
  inputMessage.value = ''
  isLoading.value = true
  error.value = ''

  await nextTick()
  scrollToBottom()

  try {
    const response = await invoke<string>('send_message', { 
      message: userMessage.content 
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
  // Re-add welcome message
  messages.value.push({
    id: '0',
    role: 'system',
    content: 'Welcome to Housaky! I\'m your AI assistant. How can I help you today?',
    timestamp: new Date(),
  })
}

function formatTime(date: Date) {
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

onMounted(async () => {
  await checkHousaky()
  
  if (housakyInstalled.value) {
    messages.value.push({
      id: '0',
      role: 'system',
      content: 'Welcome to Housaky! I\'m your AI assistant. How can I help you today?',
      timestamp: new Date(),
    })
  } else {
    messages.value.push({
      id: '0',
      role: 'system',
      content: 'Housaky is not installed. Please install Housaky to start chatting.',
      timestamp: new Date(),
    })
  }
})
</script>

<template>
  <div class="h-full flex flex-col p-6">
    <div class="flex items-center justify-between mb-4">
      <div>
        <h1 class="text-3xl font-bold">Chat</h1>
        <p class="text-muted-foreground">Talk with your AI assistant</p>
      </div>
      <div class="flex items-center gap-2">
        <Badge :variant="housakyInstalled ? 'success' : 'destructive'">
          {{ housakyInstalled ? 'Ready' : 'Not Installed' }}
        </Badge>
        <Button variant="outline" size="icon" @click="clearChat">
          <Trash2 class="w-4 h-4" />
        </Button>
      </div>
    </div>

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
      <CardHeader class="border-b pb-4">
        <div class="flex items-center justify-between">
          <CardTitle class="flex items-center gap-2">
            <Bot class="w-5 h-5" />
            Housaky Assistant
          </CardTitle>
          <Badge :variant="housakyInstalled ? 'success' : 'secondary'">
            {{ housakyInstalled ? 'Online' : 'Offline' }}
          </Badge>
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
                :class="message.role === 'user' ? 'bg-primary' : message.role === 'system' ? 'bg-yellow-500' : 'bg-secondary'"
              >
                <User v-if="message.role === 'user'" class="w-4 h-4 text-primary-foreground" />
                <Bot v-else-if="message.role === 'assistant'" class="w-4 h-4" />
                <AlertCircle v-else class="w-4 h-4 text-white" />
              </div>
              
              <div
                class="rounded-lg p-3"
                :class="message.role === 'user' ? 'bg-primary text-primary-foreground' : message.role === 'system' ? 'bg-yellow-100 dark:bg-yellow-900' : 'bg-muted'"
              >
                <p class="whitespace-pre-wrap">{{ message.content }}</p>
                <p class="text-xs mt-1 opacity-60">{{ formatTime(message.timestamp) }}</p>
              </div>
            </div>
          </div>

          <div v-if="isLoading" class="flex gap-3">
            <div class="w-8 h-8 rounded-full bg-secondary flex items-center justify-center">
              <Loader2 class="w-4 h-4 animate-spin" />
            </div>
            <div class="rounded-lg p-3 bg-muted">
              <p class="text-muted-foreground">Thinking...</p>
            </div>
          </div>
        </div>
      </ScrollArea>

      <div class="p-4 border-t">
        <form @submit.prevent="sendMessage" class="flex gap-2">
          <Input
            v-model="inputMessage"
            placeholder="Type your message..."
            :disabled="isLoading || !housakyInstalled"
            @keydown.enter.exact.prevent="sendMessage"
          />
          <Button type="submit" :disabled="isLoading || !inputMessage.trim() || !housakyInstalled">
            <Send class="w-4 h-4" />
          </Button>
        </form>
      </div>
    </Card>
  </div>
</template>
