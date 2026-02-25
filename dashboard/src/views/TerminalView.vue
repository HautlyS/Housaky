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
import { 
  Terminal, 
  Play,
  Trash2,
  Download,
  Command,
  AlertCircle,
  CheckCircle2,
  Loader2,
  Copy,
  History,
  RotateCcw
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface TerminalLine {
  id: string
  type: 'input' | 'output' | 'error' | 'system' | 'success'
  content: string
  timestamp: Date
}

const inputCommand = ref('')
const lines = ref<TerminalLine[]>([])
const terminalContainer = ref<HTMLElement | null>(null)
const isRunning = ref(false)
const history = ref<string[]>([])
const historyIndex = ref(-1)
const housakyInstalled = ref(true)

const quickCommands = [
  { cmd: 'housaky status', desc: 'Show system status' },
  { cmd: 'housaky doctor', desc: 'Run diagnostics' },
  { cmd: 'housaky channel list', desc: 'List channels' },
  { cmd: 'housaky skills list', desc: 'List skills' },
  { cmd: 'housaky config show', desc: 'Show configuration' },
  { cmd: 'housaky --version', desc: 'Show version' },
]

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

async function runCommand() {
  if (!inputCommand.value.trim() || isRunning.value) return

  if (!isTauri) {
    lines.value.push({
      id: Date.now().toString(),
      type: 'error',
      content: 'Running in server mode - commands not available',
      timestamp: new Date(),
    })
    return
  }

  const cmd = inputCommand.value.trim()
  lines.value.push({
    id: Date.now().toString(),
    type: 'input',
    content: `$ ${cmd}`,
    timestamp: new Date(),
  })

  history.value.push(cmd)
  historyIndex.value = history.value.length
  inputCommand.value = ''
  isRunning.value = true

  await nextTick()
  scrollToBottom()

  try {
    const parts = cmd.split(' ')
    const command = parts[0] === 'housaky' ? parts[1] || '' : parts[0]
    const args = parts[0] === 'housaky' ? parts.slice(2) : parts.slice(1)
    
    let output = ''
    
    if (cmd.startsWith('housaky ')) {
      output = await invoke<string>('run_housaky_command_cmd', { command, args })
      lines.value.push({
        id: (Date.now() + 1).toString(),
        type: 'success',
        content: output || 'Command completed successfully.',
        timestamp: new Date(),
      })
    } else {
      lines.value.push({
        id: (Date.now() + 1).toString(),
        type: 'error',
        content: `Unknown command: ${cmd}. Use 'housaky' commands.`,
        timestamp: new Date(),
      })
    }
  } catch (e) {
    lines.value.push({
      id: (Date.now() + 1).toString(),
      type: 'error',
      content: `Error: ${e}`,
      timestamp: new Date(),
    })
  } finally {
    isRunning.value = false
    await nextTick()
    scrollToBottom()
  }
}

function scrollToBottom() {
  if (terminalContainer.value) {
    terminalContainer.value.scrollTop = terminalContainer.value.scrollHeight
  }
}

function clearTerminal() {
  lines.value = []
  addWelcomeMessage()
}

function addWelcomeMessage() {
  lines.value.push({
    id: '0',
    type: 'system',
    content: 'Welcome to Housaky Terminal!\nType "housaky --help" for available commands.',
    timestamp: new Date(),
  })
}

function downloadHistory() {
  const content = lines.value
    .map(l => `[${l.timestamp.toISOString()}] [${l.type.toUpperCase()}] ${l.content}`)
    .join('\n')
  
  const blob = new Blob([content], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `housaky-terminal-${Date.now()}.log`
  a.click()
  URL.revokeObjectURL(url)
}

function insertCommand(cmd: string) {
  inputCommand.value = cmd
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowUp') {
    e.preventDefault()
    if (historyIndex.value > 0) {
      historyIndex.value--
      inputCommand.value = history.value[historyIndex.value]
    }
  } else if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (historyIndex.value < history.value.length - 1) {
      historyIndex.value++
      inputCommand.value = history.value[historyIndex.value]
    } else {
      historyIndex.value = history.value.length
      inputCommand.value = ''
    }
  }
}

function copyOutput() {
  const output = lines.value
    .filter(l => l.type === 'output' || l.type === 'success')
    .map(l => l.content)
    .join('\n')
  navigator.clipboard.writeText(output)
}

onMounted(async () => {
  await checkHousaky()
  addWelcomeMessage()
})
</script>

<template>
  <div class="h-full flex flex-col p-6 gap-4">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">Terminal</h1>
        <p class="text-sm text-muted-foreground">Execute Housaky commands</p>
      </div>
      <div class="flex items-center gap-2">
        <Badge :variant="housakyInstalled ? 'success' : 'secondary'" class="flex items-center gap-1">
          <CheckCircle2 v-if="housakyInstalled" class="w-3 h-3" />
          <AlertCircle v-else class="w-3 h-3" />
          {{ housakyInstalled ? 'Ready' : 'Not Installed' }}
        </Badge>
        <Button variant="outline" size="sm" @click="clearTerminal">
          <Trash2 class="w-4 h-4 mr-2" />
          Clear
        </Button>
        <Button variant="outline" size="sm" @click="downloadHistory">
          <Download class="w-4 h-4 mr-2" />
          Export
        </Button>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-4 gap-4 flex-1 min-h-0">
      <Card class="lg:col-span-3 flex flex-col min-h-0">
        <CardHeader class="pb-2 border-b py-3">
          <div class="flex items-center justify-between">
            <CardTitle class="flex items-center gap-2 text-base">
              <Terminal class="w-4 h-4" />
              Terminal
            </CardTitle>
            <Badge :variant="isRunning ? 'warning' : 'secondary'" class="text-xs">
              {{ isRunning ? 'Running' : 'Ready' }}
            </Badge>
          </div>
        </CardHeader>
        
        <ScrollArea class="flex-1 p-4 font-mono text-sm bg-gray-950 text-gray-100" ref="terminalContainer">
          <div class="space-y-1">
            <div
              v-for="line in lines"
              :key="line.id"
              class="whitespace-pre-wrap"
              :class="{
                'text-green-400': line.type === 'input',
                'text-gray-400': line.type === 'output',
                'text-red-400': line.type === 'error',
                'text-blue-400': line.type === 'system',
                'text-green-300': line.type === 'success',
              }"
            >
              {{ line.content }}
            </div>

            <div v-if="isRunning" class="flex items-center gap-2 text-yellow-400">
              <Loader2 class="w-4 h-4 animate-spin" />
              <span>Executing...</span>
            </div>
          </div>
        </ScrollArea>

        <div class="p-4 border-t bg-gray-900">
          <form @submit.prevent="runCommand" class="flex gap-2">
            <div class="flex items-center px-3 bg-gray-800 rounded-l-md text-green-400 font-mono">
              $
            </div>
            <Input
              v-model="inputCommand"
              placeholder="Type a command..."
              :disabled="isRunning || !housakyInstalled"
              class="font-mono rounded-none bg-gray-800 border-gray-700 text-gray-100 placeholder:text-gray-500"
              @keydown="handleKeydown"
            />
            <Button type="submit" :disabled="isRunning || !inputCommand.trim() || !housakyInstalled" class="rounded-l-none">
              <Play class="w-4 h-4" />
            </Button>
          </form>
        </div>
      </Card>

      <Card class="lg:col-span-1 h-fit">
        <CardHeader class="py-3">
          <CardTitle class="flex items-center gap-2 text-base">
            <Command class="w-4 h-4" />
            Quick Commands
          </CardTitle>
        </CardHeader>
        <CardContent class="p-3">
          <div class="space-y-1">
            <button
              v-for="example in quickCommands"
              :key="example.cmd"
              @click="insertCommand(example.cmd)"
              class="w-full text-left p-2 rounded-lg border hover:bg-muted transition-colors"
            >
              <code class="text-xs font-mono text-primary">{{ example.cmd }}</code>
              <p class="text-xs text-muted-foreground mt-0.5">{{ example.desc }}</p>
            </button>
          </div>
          
          <div class="mt-4 pt-4 border-t">
            <div class="flex items-center gap-2 text-xs text-muted-foreground mb-2">
              <History class="w-3 h-3" />
              Recent Commands
            </div>
            <div v-if="history.length === 0" class="text-xs text-muted-foreground">
              No commands yet
            </div>
            <div v-else class="space-y-1">
              <button
                v-for="(cmd, i) in history.slice(-5).reverse()"
                :key="i"
                @click="insertCommand(cmd)"
                class="w-full text-left p-1.5 rounded text-xs font-mono bg-muted/50 hover:bg-muted"
              >
                {{ cmd }}
              </button>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  </div>
</template>
