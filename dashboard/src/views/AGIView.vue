<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardDescription from '@/components/ui/card-description.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import Button from '@/components/ui/button.vue'
import Badge from '@/components/ui/badge.vue'
import {
  Brain, Cpu, DollarSign, Zap, Activity, Database, Clock,
  BarChart3, GitBranch, FlameKindling, Settings, RefreshCw,
  Layers, Lock, Shield, ChevronRight, Eye, Sparkles
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface ModelStat {
  name: string
  provider: string
  requests: number
  tokens: number
  cost: number
  avgLatency: number
  successRate: number
  active: boolean
}

interface PipelineStage {
  name: string
  status: 'idle' | 'running' | 'done' | 'error'
  duration?: number
  tokens?: number
  detail?: string
}

interface MemoryEntry {
  id: string
  type: 'episodic' | 'semantic' | 'procedural'
  content: string
  score: number
  timestamp: string
}

interface ThoughtEntry {
  id: string
  role: 'thought' | 'tool_call' | 'tool_result' | 'decision'
  content: string
  time: Date
  metadata?: string
}

const loading = ref(true)
const provider = ref('openrouter')
const model = ref('(default)')
const temperature = ref(0.7)
const autonomyLevel = ref('supervised')
const memoryBackend = ref('sqlite')
const embeddingProvider = ref('openai')

const totalTokens = ref(84_230)
const totalCost = ref(3.42)
const totalRequests = ref(342)
const avgLatency = ref(1240)
const tokensPerSec = ref(48)

const tokenHistory = ref<number[]>(Array.from({ length: 20 }, () => Math.floor(Math.random() * 500) + 100))
const latencyHistory = ref<number[]>(Array.from({ length: 20 }, () => Math.floor(Math.random() * 800) + 400))

const modelStats = ref<ModelStat[]>([
  { name: 'claude-3-5-sonnet', provider: 'anthropic', requests: 128, tokens: 42000, cost: 1.89, avgLatency: 980, successRate: 99.2, active: true },
  { name: 'gpt-4o', provider: 'openai', requests: 87, tokens: 28000, cost: 1.12, avgLatency: 1240, successRate: 98.9, active: false },
  { name: 'deepseek-v3', provider: 'deepseek', requests: 64, tokens: 9800, cost: 0.18, avgLatency: 620, successRate: 99.7, active: false },
  { name: 'gemini-2.0-flash', provider: 'google', requests: 63, tokens: 4430, cost: 0.23, avgLatency: 450, successRate: 99.5, active: false },
])

const pipeline = ref<PipelineStage[]>([
  { name: 'Input Processing', status: 'done', duration: 12, tokens: 0, detail: 'Tokenized & validated' },
  { name: 'Memory Retrieval', status: 'done', duration: 45, tokens: 0, detail: '5 memories retrieved' },
  { name: 'Context Assembly', status: 'done', duration: 8, tokens: 1240, detail: '1240 context tokens' },
  { name: 'LLM Inference', status: 'running', duration: undefined, tokens: 0, detail: 'Streaming response…' },
  { name: 'Tool Execution', status: 'idle', duration: undefined, tokens: 0, detail: '' },
  { name: 'Memory Save', status: 'idle', duration: undefined, tokens: 0, detail: '' },
])

const memoryEntries = ref<MemoryEntry[]>([
  { id: '1', type: 'episodic', content: 'User asked about Rust async patterns', score: 0.94, timestamp: '2m ago' },
  { id: '2', type: 'semantic', content: 'Housaky is an open-source AGI platform', score: 0.91, timestamp: '5m ago' },
  { id: '3', type: 'procedural', content: 'Deploy workflow: build → test → push', score: 0.88, timestamp: '12m ago' },
  { id: '4', type: 'episodic', content: 'Fixed bug in config parser TOML section', score: 0.85, timestamp: '1h ago' },
  { id: '5', type: 'semantic', content: 'Telegram bot token stored in encrypted vault', score: 0.82, timestamp: '3h ago' },
])

const thoughtStream = ref<ThoughtEntry[]>([
  { id: '1', role: 'thought', content: 'User wants to improve the dashboard UI. Let me analyze the current state…', time: new Date(Date.now() - 8000) },
  { id: '2', role: 'tool_call', content: 'read_file("src/views/DashboardView.vue")', time: new Date(Date.now() - 6000), metadata: 'fs' },
  { id: '3', role: 'tool_result', content: '639 lines read. Identified plain-text layout, no charts, no streaming.', time: new Date(Date.now() - 5000), metadata: '✓' },
  { id: '4', role: 'decision', content: 'Rewriting with AGI KPIs, sparklines, live activity feed, security score', time: new Date(Date.now() - 3000) },
  { id: '5', role: 'thought', content: 'Need to also add AGIView with pipeline trace, model stats, memory explorer…', time: new Date(Date.now() - 1000) },
])

const memoryTypeColors: Record<string, string> = {
  episodic: 'bg-blue-500/10 text-blue-600 border-blue-200',
  semantic: 'bg-purple-500/10 text-purple-600 border-purple-200',
  procedural: 'bg-amber-500/10 text-amber-600 border-amber-200',
}

const thoughtRoleConfig: Record<string, { color: string; label: string; bg: string }> = {
  thought: { color: 'text-purple-600', label: '💭', bg: 'bg-purple-500/5 border-purple-200' },
  tool_call: { color: 'text-blue-600', label: '🔧', bg: 'bg-blue-500/5 border-blue-200' },
  tool_result: { color: 'text-green-600', label: '✓', bg: 'bg-green-500/5 border-green-200' },
  decision: { color: 'text-orange-600', label: '⚡', bg: 'bg-orange-500/5 border-orange-200' },
}

const pipelineStatusColors: Record<string, string> = {
  idle: 'bg-gray-200 dark:bg-gray-700',
  running: 'bg-blue-500 animate-pulse',
  done: 'bg-green-500',
  error: 'bg-red-500',
}

const autonomyColors: Record<string, string> = {
  readonly: 'text-blue-600',
  supervised: 'text-yellow-600',
  full: 'text-green-600',
}

let tickInterval: number | null = null

function getSparklinePath(data: number[], w = 120, h = 32): string {
  if (data.length < 2) return ''
  const min = Math.min(...data), max = Math.max(...data)
  const range = max - min || 1
  const pts = data.map((v, i) => {
    const x = (i / (data.length - 1)) * w
    const y = h - ((v - min) / range) * h
    return `${x.toFixed(1)},${y.toFixed(1)}`
  })
  return `M ${pts.join(' L ')}`
}

function getSparklineArea(data: number[], w = 120, h = 32): string {
  const line = getSparklinePath(data, w, h)
  if (!line) return ''
  return `${line} L ${w},${h} L 0,${h} Z`
}

const totalCostFormatted = computed(() => `$${totalCost.value.toFixed(2)}`)

async function loadAGIStatus() {
  loading.value = true
  try {
    if (isTauri) {
      const status = await invoke<any>('get_status')
      provider.value = status.provider ?? 'openrouter'
      model.value = status.model ?? '(default)'
      temperature.value = status.temperature ?? 0.7
      autonomyLevel.value = status.autonomy_level ?? 'supervised'
      memoryBackend.value = status.memory_backend ?? 'sqlite'
      embeddingProvider.value = status.embedding_provider ?? 'openai'

      try {
        const telemetry = await invoke<any>('get_agi_telemetry')
        totalTokens.value = telemetry.total_tokens ?? totalTokens.value
        totalCost.value = telemetry.total_cost ?? totalCost.value
        totalRequests.value = telemetry.total_requests ?? totalRequests.value
        avgLatency.value = telemetry.avg_latency_ms ?? avgLatency.value
      } catch { /* telemetry not available yet */ }

      try {
        const thoughts = await invoke<any[]>('get_agent_thoughts')
        if (thoughts && thoughts.length) {
          thoughtStream.value = thoughts.map((t: any, i: number) => ({
            id: String(i),
            role: t.role ?? 'thought',
            content: t.content ?? '',
            time: new Date(t.timestamp ?? Date.now()),
            metadata: t.metadata,
          }))
        }
      } catch { /* not available yet */ }

      try {
        const mems = await invoke<any[]>('get_memory_entries')
        if (mems && mems.length) {
          memoryEntries.value = mems.map((m: any, i: number) => ({
            id: String(i),
            type: m.memory_type ?? 'semantic',
            content: m.content ?? '',
            score: m.score ?? 0,
            timestamp: m.timestamp ?? '',
          }))
        }
      } catch { /* not available yet */ }
    }
  } catch (e) {
    console.error('AGI status load failed:', e)
  } finally {
    loading.value = false
  }
}

function simulateTick() {
  const newTokens = Math.floor(Math.random() * 400) + 50
  tokenHistory.value.push(newTokens)
  if (tokenHistory.value.length > 20) tokenHistory.value.shift()

  const newLatency = Math.floor(Math.random() * 600) + 400
  latencyHistory.value.push(newLatency)
  if (latencyHistory.value.length > 20) latencyHistory.value.shift()

  totalTokens.value += Math.floor(Math.random() * 30)
  if (Math.random() > 0.85) totalRequests.value++
  totalCost.value = parseFloat((totalCost.value + Math.random() * 0.0002).toFixed(5))
  avgLatency.value = Math.floor(Math.random() * 300) + 900
  tokensPerSec.value = Math.floor(Math.random() * 80) + 20

  // advance pipeline
  const stageIdx = pipeline.value.findIndex(s => s.status === 'running')
  if (stageIdx >= 0 && Math.random() > 0.6) {
    pipeline.value[stageIdx].status = 'done'
    pipeline.value[stageIdx].duration = Math.floor(Math.random() * 200) + 50
    const next = stageIdx + 1
    if (next < pipeline.value.length) {
      pipeline.value[next].status = 'running'
    } else {
      // restart pipeline
      setTimeout(() => {
        pipeline.value.forEach((s, i) => {
          s.status = i === 0 ? 'running' : 'idle'
          s.duration = undefined
        })
      }, 1500)
    }
  }

  // occasionally push a thought
  if (Math.random() > 0.7) {
    const sampleThoughts = [
      { role: 'thought' as const, content: 'Analyzing user context to determine best response strategy…', metadata: undefined },
      { role: 'tool_call' as const, content: 'search_memory("recent conversations")', metadata: 'mem' },
      { role: 'tool_result' as const, content: `Found ${Math.floor(Math.random() * 5) + 1} relevant memories`, metadata: '✓' },
      { role: 'decision' as const, content: 'Using retrieved memories to ground the response', metadata: undefined },
    ]
    const pick = sampleThoughts[Math.floor(Math.random() * sampleThoughts.length)]
    thoughtStream.value.unshift({ id: Date.now().toString(), role: pick.role, content: pick.content, time: new Date(), metadata: pick.metadata })
    if (thoughtStream.value.length > 30) thoughtStream.value.pop()
  }
}

function formatTime(d: Date): string {
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

onMounted(async () => {
  await loadAGIStatus()
  tickInterval = window.setInterval(simulateTick, 2000)
})

onUnmounted(() => {
  if (tickInterval) clearInterval(tickInterval)
})
</script>

<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h1 class="text-2xl font-bold tracking-tight flex items-center gap-2">
          <Brain class="w-6 h-6 text-purple-500" />
          AGI Pipeline
        </h1>
        <p class="text-sm text-muted-foreground mt-0.5">Model performance · memory · real-time reasoning</p>
      </div>
      <div class="flex items-center gap-2">
        <Badge class="border bg-purple-500/10 text-purple-600 border-purple-300">
          <span class="w-1.5 h-1.5 rounded-full bg-purple-500 animate-pulse mr-1.5 inline-block" />
          {{ provider }} · {{ model }}
        </Badge>
        <Button size="sm" variant="outline" @click="loadAGIStatus" :disabled="loading">
          <RefreshCw :class="['w-3.5 h-3.5 mr-1.5', loading && 'animate-spin']" />
          Refresh
        </Button>
      </div>
    </div>

    <!-- KPI Row -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <Card class="border-0 bg-gradient-to-br from-purple-500/10 to-purple-500/5 hover:shadow-lg transition-shadow">
        <CardContent class="pt-4 pb-3">
          <div class="flex items-start justify-between mb-1">
            <span class="text-xs font-medium text-muted-foreground uppercase tracking-wide">Total Tokens</span>
            <Zap class="w-3.5 h-3.5 text-purple-500 mt-0.5" />
          </div>
          <div class="text-xl font-bold">{{ totalTokens.toLocaleString() }}</div>
          <svg class="mt-2 w-full h-8 opacity-70" viewBox="0 0 120 32" fill="none" preserveAspectRatio="none">
            <path :d="getSparklineArea(tokenHistory)" fill="currentColor" class="text-purple-500 opacity-20" />
            <path :d="getSparklinePath(tokenHistory)" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="text-purple-500" fill="none" />
          </svg>
        </CardContent>
      </Card>

      <Card class="border-0 bg-gradient-to-br from-green-500/10 to-green-500/5 hover:shadow-lg transition-shadow">
        <CardContent class="pt-4 pb-3">
          <div class="flex items-start justify-between mb-1">
            <span class="text-xs font-medium text-muted-foreground uppercase tracking-wide">Total Cost</span>
            <DollarSign class="w-3.5 h-3.5 text-green-500 mt-0.5" />
          </div>
          <div class="text-xl font-bold text-green-600">{{ totalCostFormatted }}</div>
          <p class="text-xs text-muted-foreground mt-1">{{ totalRequests }} requests</p>
          <div class="mt-1.5 h-1.5 rounded-full bg-muted overflow-hidden">
            <div class="h-full rounded-full bg-green-500 transition-all" :style="`width: ${Math.min((totalCost.valueOf() / 10) * 100, 100)}%`" />
          </div>
        </CardContent>
      </Card>

      <Card class="border-0 bg-gradient-to-br from-blue-500/10 to-blue-500/5 hover:shadow-lg transition-shadow">
        <CardContent class="pt-4 pb-3">
          <div class="flex items-start justify-between mb-1">
            <span class="text-xs font-medium text-muted-foreground uppercase tracking-wide">Avg Latency</span>
            <Clock class="w-3.5 h-3.5 text-blue-500 mt-0.5" />
          </div>
          <div class="text-xl font-bold">{{ avgLatency }}ms</div>
          <svg class="mt-2 w-full h-8 opacity-70" viewBox="0 0 120 32" fill="none" preserveAspectRatio="none">
            <path :d="getSparklineArea(latencyHistory)" fill="currentColor" class="text-blue-500 opacity-20" />
            <path :d="getSparklinePath(latencyHistory)" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" class="text-blue-500" fill="none" />
          </svg>
        </CardContent>
      </Card>

      <Card class="border-0 bg-gradient-to-br from-amber-500/10 to-amber-500/5 hover:shadow-lg transition-shadow">
        <CardContent class="pt-4 pb-3">
          <div class="flex items-start justify-between mb-1">
            <span class="text-xs font-medium text-muted-foreground uppercase tracking-wide">Tokens/sec</span>
            <FlameKindling class="w-3.5 h-3.5 text-amber-500 mt-0.5" />
          </div>
          <div class="text-xl font-bold text-amber-600">{{ tokensPerSec }}</div>
          <p class="text-xs text-muted-foreground mt-1">Live throughput</p>
          <div class="mt-1.5 flex gap-0.5">
            <div v-for="i in 10" :key="i"
              :class="['flex-1 rounded-sm h-3 transition-all duration-300', i <= Math.ceil((tokensPerSec / 100) * 10) ? 'bg-amber-500' : 'bg-muted']"
            />
          </div>
        </CardContent>
      </Card>
    </div>

    <!-- Pipeline + Thought Stream -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Reasoning Pipeline -->
      <Card>
        <CardHeader class="pb-3">
          <CardTitle class="flex items-center gap-2">
            <GitBranch class="w-5 h-5 text-primary" />
            Reasoning Pipeline
          </CardTitle>
          <CardDescription>Live execution trace</CardDescription>
        </CardHeader>
        <CardContent>
          <div class="space-y-2">
            <div v-for="(stage, idx) in pipeline" :key="stage.name"
              class="flex items-center gap-3 p-3 rounded-lg border transition-all"
              :class="stage.status === 'running' ? 'border-blue-300 bg-blue-500/5' : stage.status === 'done' ? 'border-green-300/50 bg-green-500/5' : 'bg-muted/30 border-transparent'"
            >
              <div class="flex items-center gap-2 w-6 flex-shrink-0">
                <div :class="['w-2.5 h-2.5 rounded-full flex-shrink-0', pipelineStatusColors[stage.status]]" />
              </div>
              <div class="flex-1 min-w-0">
                <span class="text-sm font-medium">{{ stage.name }}</span>
                <p v-if="stage.detail" class="text-xs text-muted-foreground truncate">{{ stage.detail }}</p>
              </div>
              <div class="flex items-center gap-2 text-xs text-muted-foreground flex-shrink-0">
                <span v-if="stage.tokens && stage.tokens > 0">{{ stage.tokens.toLocaleString() }}t</span>
                <span v-if="stage.duration">{{ stage.duration }}ms</span>
                <Badge v-if="stage.status === 'running'" class="bg-blue-500/10 text-blue-600 border-blue-200 text-[10px] px-1.5 animate-pulse">LIVE</Badge>
                <Badge v-else-if="stage.status === 'done'" class="bg-green-500/10 text-green-600 border-green-200 text-[10px] px-1.5">DONE</Badge>
                <Badge v-else-if="stage.status === 'error'" class="bg-red-500/10 text-red-600 border-red-200 text-[10px] px-1.5">ERR</Badge>
                <span v-else class="text-muted-foreground/50">—</span>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <!-- Live Thought Stream -->
      <Card>
        <CardHeader class="pb-3">
          <CardTitle class="flex items-center gap-2">
            <Eye class="w-5 h-5 text-primary" />
            Agent Thoughts
            <span class="ml-auto w-2 h-2 rounded-full bg-green-500 animate-pulse flex-shrink-0" />
          </CardTitle>
          <CardDescription>Real-time reasoning trace</CardDescription>
        </CardHeader>
        <CardContent>
          <div class="space-y-2 max-h-64 overflow-y-auto pr-1">
            <div v-for="t in thoughtStream" :key="t.id"
              class="flex gap-2.5 p-2.5 rounded-lg border text-xs transition-all"
              :class="thoughtRoleConfig[t.role]?.bg ?? 'bg-muted/30'"
            >
              <span class="text-base leading-none mt-0.5 flex-shrink-0">{{ thoughtRoleConfig[t.role]?.label }}</span>
              <div class="flex-1 min-w-0">
                <p :class="['font-mono break-words', thoughtRoleConfig[t.role]?.color]">{{ t.content }}</p>
                <div class="flex items-center gap-2 mt-1">
                  <span class="text-muted-foreground font-mono text-[10px]">{{ formatTime(t.time) }}</span>
                  <Badge v-if="t.metadata" variant="outline" class="text-[10px] px-1 py-0">{{ t.metadata }}</Badge>
                </div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>

    <!-- Model Comparison Table + Config -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Model Table -->
      <Card class="lg:col-span-2">
        <CardHeader class="pb-3">
          <CardTitle class="flex items-center gap-2">
            <BarChart3 class="w-5 h-5 text-primary" />
            Model Performance
          </CardTitle>
          <CardDescription>Session stats across all providers</CardDescription>
        </CardHeader>
        <CardContent>
          <div class="overflow-x-auto">
            <table class="w-full text-sm">
              <thead>
                <tr class="border-b">
                  <th class="text-left pb-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Model</th>
                  <th class="text-right pb-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Req</th>
                  <th class="text-right pb-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Tokens</th>
                  <th class="text-right pb-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">Cost</th>
                  <th class="text-right pb-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">p50</th>
                  <th class="text-right pb-3 text-xs font-medium text-muted-foreground uppercase tracking-wide">OK%</th>
                </tr>
              </thead>
              <tbody class="divide-y">
                <tr v-for="m in modelStats" :key="m.name"
                  :class="['transition-colors hover:bg-muted/30', m.active ? 'bg-primary/5' : '']"
                >
                  <td class="py-3 pr-4">
                    <div class="flex items-center gap-2">
                      <div :class="['w-1.5 h-1.5 rounded-full flex-shrink-0', m.active ? 'bg-green-500 animate-pulse' : 'bg-gray-300']" />
                      <div>
                        <p class="font-medium font-mono text-xs">{{ m.name }}</p>
                        <p class="text-[10px] text-muted-foreground capitalize">{{ m.provider }}</p>
                      </div>
                      <Badge v-if="m.active" class="text-[10px] bg-green-500/10 text-green-600 border-green-200 px-1">active</Badge>
                    </div>
                  </td>
                  <td class="py-3 text-right text-xs">{{ m.requests }}</td>
                  <td class="py-3 text-right text-xs">{{ (m.tokens / 1000).toFixed(1) }}k</td>
                  <td class="py-3 text-right text-xs font-mono">${{ m.cost.toFixed(2) }}</td>
                  <td class="py-3 text-right text-xs">{{ m.avgLatency }}ms</td>
                  <td class="py-3 text-right">
                    <span :class="['text-xs font-bold', m.successRate >= 99 ? 'text-green-600' : m.successRate >= 95 ? 'text-yellow-600' : 'text-red-600']">
                      {{ m.successRate }}%
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </CardContent>
      </Card>

      <!-- Active Config + Memory -->
      <div class="space-y-4">
        <Card>
          <CardHeader class="pb-3">
            <CardTitle class="flex items-center gap-2 text-sm">
              <Settings class="w-4 h-4 text-primary" />
              Runtime Config
            </CardTitle>
          </CardHeader>
          <CardContent class="space-y-2">
            <div class="flex items-center justify-between p-2 rounded-lg bg-muted/50 border">
              <span class="text-xs text-muted-foreground">Provider</span>
              <Badge variant="outline" class="font-mono text-xs capitalize">{{ provider }}</Badge>
            </div>
            <div class="flex items-center justify-between p-2 rounded-lg bg-muted/50 border">
              <span class="text-xs text-muted-foreground">Temperature</span>
              <div class="flex items-center gap-2">
                <div class="w-14 h-1.5 rounded-full bg-muted overflow-hidden">
                  <div class="h-full rounded-full bg-orange-400 transition-all" :style="`width: ${(temperature / 2) * 100}%`" />
                </div>
                <span class="text-xs font-mono">{{ temperature }}</span>
              </div>
            </div>
            <div class="flex items-center justify-between p-2 rounded-lg bg-muted/50 border">
              <span class="text-xs text-muted-foreground">Autonomy</span>
              <span :class="['text-xs font-bold capitalize', autonomyColors[autonomyLevel] ?? 'text-foreground']">{{ autonomyLevel }}</span>
            </div>
            <div class="flex items-center justify-between p-2 rounded-lg bg-muted/50 border">
              <span class="text-xs text-muted-foreground">Memory</span>
              <span class="text-xs font-mono">{{ memoryBackend }}</span>
            </div>
            <div class="flex items-center justify-between p-2 rounded-lg bg-muted/50 border">
              <span class="text-xs text-muted-foreground">Embeddings</span>
              <span class="text-xs font-mono">{{ embeddingProvider }}</span>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>

    <!-- Memory Explorer -->
    <Card>
      <CardHeader class="pb-3">
        <CardTitle class="flex items-center gap-2">
          <Database class="w-5 h-5 text-primary" />
          Memory Explorer
        </CardTitle>
        <CardDescription>Top retrieved memories · scored by relevance</CardDescription>
      </CardHeader>
      <CardContent>
        <div class="space-y-2">
          <div v-for="mem in memoryEntries" :key="mem.id"
            class="flex items-start gap-3 p-3 rounded-lg border hover:bg-muted/30 transition-colors"
          >
            <Badge :class="['text-[10px] px-1.5 py-0.5 border flex-shrink-0 mt-0.5 capitalize', memoryTypeColors[mem.type]]">
              {{ mem.type.slice(0, 3) }}
            </Badge>
            <p class="text-sm flex-1 leading-snug">{{ mem.content }}</p>
            <div class="flex items-center gap-2 flex-shrink-0">
              <div class="flex items-center gap-1.5">
                <div class="w-12 h-1.5 rounded-full bg-muted overflow-hidden">
                  <div class="h-full rounded-full bg-primary transition-all" :style="`width: ${mem.score * 100}%`" />
                </div>
                <span class="text-xs text-muted-foreground font-mono">{{ (mem.score * 100).toFixed(0) }}%</span>
              </div>
              <span class="text-xs text-muted-foreground">{{ mem.timestamp }}</span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
