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
  Square,
  Trash2,
  Download,
  Command,
  AlertCircle,
  CheckCircle2
} from 'lucide-vue-next'

interface TerminalLine {
  id: string
  type: 'input' | 'output' | 'error' | 'system'
  content: string
  timestamp: Date
}

const inputCommand = ref('')
const lines = ref<TerminalLine[]>([])
const terminalContainer = ref<HTMLElement | null>(null)
const isRunning = ref(false)

const history = ref<string[]>([])
const historyIndex = ref(-1)

const sampleCommands = [
  { cmd: 'housaky status', desc: 'Show system status' },
  { cmd: 'housaky doctor', desc: 'Run diagnostics' },
  { cmd: 'housaky channel list', desc: 'List channels' },
  { cmd: 'housaky skills list', desc: 'List skills' },
  { cmd: 'housaky integrations info Telegram', desc: 'Show Telegram setup' },
]

async function runCommand() {
  if (!inputCommand.value.trim() || isRunning.value) return

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
    // Simulate command execution - in real app would use Tauri shell
    await new Promise(r => setTimeout(r, 300))
    
    let output = ''
    if (cmd === 'housaky status') {
      output = `🦀 Housaky Status

Version:     0.1.0
Workspace:   ~/.housaky/workspace
Config:      ~/.housaky/config.toml

🤖 Provider:      openrouter
   Model:         (default)
🛡️  Autonomy:      supervised
⚙️  Runtime:       native
💓 Heartbeat:      disabled
🧠 Memory:         sqlite (auto-save: on)

Channels:
  CLI:      ✅ always
  Telegram: ❌ not configured
  Discord:  ❌ not configured
  Slack:    ❌ not configured`
    } else if (cmd === 'housaky doctor') {
      output = `✅ All checks passed

- Config file: OK
- Workspace: OK
- Providers: OK
- Memory: OK
- Security: OK`
    } else if (cmd.startsWith('housaky ')) {
      output = `Command executed successfully.`
    } else {
      output = `Command not found: ${cmd}`
    }

    lines.value.push({
      id: (Date.now() + 1).toString(),
      type: 'output',
      content: output,
      timestamp: new Date(),
    })
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
}

function downloadHistory() {
  const content = lines.value
    .map(l => `[${l.timestamp.toISOString()}] ${l.content}`)
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

onMounted(() => {
  lines.value.push({
    id: '0',
    type: 'system',
    content: 'Welcome to Housaky Terminal! Type a command or select from examples below.',
    timestamp: new Date(),
  })
})
</script>

<template>
  <div class="h-full flex flex-col p-6 gap-4">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-bold">Terminal</h1>
        <p class="text-muted-foreground">Execute Housaky commands</p>
      </div>
      <div class="flex gap-2">
        <Button variant="outline" @click="clearTerminal">
          <Trash2 class="w-4 h-4 mr-2" />
          Clear
        </Button>
        <Button variant="outline" @click="downloadHistory">
          <Download class="w-4 h-4 mr-2" />
          Export
        </Button>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-4 gap-4 h-full">
      <!-- Terminal -->
      <Card class="lg:col-span-3 flex flex-col min-h-0">
        <CardHeader class="pb-2 border-b">
          <CardTitle class="flex items-center gap-2 text-base">
            <Terminal class="w-4 h-4" />
            Terminal
            <Badge :variant="isRunning ? 'warning' : 'success'" class="ml-2">
              {{ isRunning ? 'Running' : 'Ready' }}
            </Badge>
          </CardTitle>
        </CardHeader>
        
        <ScrollArea class="flex-1 p-4" ref="terminalContainer">
          <div class="font-mono text-sm space-y-1">
            <div
              v-for="line in lines"
              :key="line.id"
              class="whitespace-pre-wrap"
              :class="{
                'text-foreground': line.type === 'input',
                'text-muted-foreground': line.type === 'output',
                'text-red-500': line.type === 'error',
                'text-blue-500': line.type === 'system',
              }"
            >
              {{ line.content }}
            </div>
          </div>
        </ScrollArea>

        <div class="p-4 border-t">
          <form @submit.prevent="runCommand" class="flex gap-2">
            <code class="px-3 py-2 bg-muted rounded-l-md text-muted-foreground">$</code>
            <Input
              v-model="inputCommand"
              placeholder="Type a command..."
              :disabled="isRunning"
              class="font-mono rounded-l-none"
              @keydown="handleKeydown"
            />
            <Button type="submit" :disabled="isRunning || !inputCommand.trim()">
              <Play class="w-4 h-4" />
            </Button>
          </form>
        </div>
      </Card>

      <!-- Quick Commands -->
      <Card class="lg:col-span-1">
        <CardHeader class="pb-2">
          <CardTitle class="flex items-center gap-2 text-base">
            <Command class="w-4 h-4" />
            Quick Commands
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div class="space-y-2">
            <button
              v-for="example in sampleCommands"
              :key="example.cmd"
              @click="insertCommand(example.cmd)"
              class="w-full text-left p-2 rounded-lg border hover:bg-muted transition-colors"
            >
              <code class="text-sm font-mono text-primary">{{ example.cmd }}</code>
              <p class="text-xs text-muted-foreground mt-1">{{ example.desc }}</p>
            </button>
          </div>
        </CardContent>
      </Card>
    </div>
  </div>
</template>
