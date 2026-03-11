<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardDescription from '@/components/ui/card-description.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import Button from '@/components/ui/button.vue'
import Input from '@/components/ui/input.vue'
import Badge from '@/components/ui/badge.vue'
import { 
  Bot, 
  RefreshCw,
  CheckCircle2,
  XCircle,
  AlertCircle,
  Loader2,
  Settings,
  Power,
  Folder
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface KowalskiAgent {
  name: string
  agent_type: string
  enabled: boolean
  status: string
  role: string
  awareness: string[]
  max_concurrent: number
}

interface KowalskiStatus {
  installed: boolean
  agents: KowalskiAgent[]
  path: string
}

const status = ref<KowalskiStatus | null>(null)
const loading = ref(true)

const agentDescriptions: Record<string, string> = {
  code: 'Code analysis, refactoring, and documentation',
  web: 'Web research and information retrieval',
  academic: 'Academic research and paper analysis',
  data: 'Data analysis and processing',
  creative: 'Creative synthesis and idea generation',
  reasoning: 'Logical reasoning and deduction',
  federation: 'Multi-agent coordination and federation',
}

const agentIcons: Record<string, string> = {
  code: 'from-blue-500 to-cyan-500',
  web: 'from-green-500 to-emerald-500',
  academic: 'from-purple-500 to-pink-500',
  data: 'from-orange-500 to-amber-500',
  creative: 'from-pink-500 to-rose-500',
  reasoning: 'from-indigo-500 to-violet-500',
  federation: 'from-cyan-500 to-blue-500',
}

async function loadStatus() {
  loading.value = true
  try {
    if (isTauri) {
      status.value = await invoke<KowalskiStatus>('get_kowalski_status')
    }
  } catch (e) {
    console.error('Failed to load Kowalski status:', e)
  } finally {
    loading.value = false
  }
}

async function toggleAgent(name: string, enabled: boolean) {
  try {
    await invoke('toggle_subagent', { name, enabled })
    await loadStatus()
  } catch (e) {
    console.error('Failed to toggle agent:', e)
  }
}

const installed = computed(() => status.value?.installed ?? false)
const path = computed(() => status.value?.path ?? '')

onMounted(() => {
  loadStatus()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold tracking-tight flex items-center gap-2">
          <Bot class="w-6 h-6 text-purple-500" />
          Kowalski Subagents
        </h1>
        <p class="text-sm text-muted-foreground">
          Manage AI subagents for specialized tasks
        </p>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" @click="loadStatus" :disabled="loading">
          <RefreshCw :class="['w-4 h-4 mr-2', loading && 'animate-spin']" />
          Refresh
        </Button>
      </div>
    </div>

    <div v-if="!installed" class="rounded-lg border border-yellow-500 bg-yellow-50 dark:bg-yellow-900/20 p-4">
      <div class="flex items-start gap-3">
        <AlertCircle class="w-5 h-5 text-yellow-600 flex-shrink-0 mt-0.5" />
        <div>
          <h3 class="font-semibold text-yellow-800 dark:text-yellow-300">Kowalski Not Installed</h3>
          <p class="text-sm text-yellow-700 dark:text-yellow-400 mt-1">
            To use subagents, install Kowalski at: <code class="text-xs bg-yellow-100 dark:bg-yellow-900 px-1 rounded">{{ path }}</code>
          </p>
          <p class="text-xs text-yellow-600 dark:text-yellow-500 mt-2">
            Run: <code class="bg-yellow-100 dark:bg-yellow-900 px-1 rounded">git clone https://github.com/openclaw/kowalski vendor/kowalski</code>
          </p>
        </div>
      </div>
    </div>

    <div v-if="loading" class="flex items-center justify-center py-12">
      <Loader2 class="w-8 h-8 animate-spin text-muted-foreground" />
    </div>

    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <Card v-for="agent in status?.agents" :key="agent.name"
        :class="[
          'transition-all hover:shadow-md',
          agent.enabled ? 'border-green-200 dark:border-green-800' : 'opacity-70'
        ]"
      >
        <CardHeader class="pb-3">
          <div class="flex items-start justify-between">
            <div class="flex items-center gap-3">
              <div :class="['w-10 h-10 rounded-lg bg-gradient-to-br flex items-center justify-center', agentIcons[agent.agent_type] || 'from-gray-500 to-slate-500']">
                <Bot class="w-5 h-5 text-white" />
              </div>
              <div>
                <CardTitle class="text-base">{{ agent.name }}</CardTitle>
                <CardDescription class="capitalize">{{ agent.role || agent.agent_type }}</CardDescription>
              </div>
            </div>
            <Button 
              variant="ghost" 
              size="icon"
              @click="toggleAgent(agent.name, !agent.enabled)"
              :class="agent.enabled ? 'text-green-500' : 'text-muted-foreground'"
            >
              <Power class="w-5 h-5" />
            </Button>
          </div>
        </CardHeader>
        <CardContent class="space-y-3">
          <p class="text-sm text-muted-foreground">
            {{ agentDescriptions[agent.agent_type] || 'Specialized AI agent' }}
          </p>
          
          <div class="flex items-center gap-2">
            <Badge :variant="agent.status === 'available' ? 'success' : agent.status === 'not_installed' ? 'warning' : 'secondary'">
              <span :class="['w-1.5 h-1.5 rounded-full mr-1.5 inline-block', agent.status === 'available' ? 'bg-green-500' : 'bg-gray-400']" />
              {{ agent.status === 'available' ? 'Ready' : agent.status === 'not_installed' ? 'Not Installed' : 'Disabled' }}
            </Badge>
          </div>

          <div v-if="agent.enabled" class="space-y-2 pt-2 border-t">
            <div class="flex items-center justify-between text-xs">
              <span class="text-muted-foreground">Max Concurrent</span>
              <span class="font-mono">{{ agent.max_concurrent }}</span>
            </div>
            <div class="flex flex-wrap gap-1">
              <Badge v-for="aw in agent.awareness" :key="aw" variant="outline" class="text-[10px]">
                {{ aw }}
              </Badge>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>

    <Card>
      <CardHeader class="pb-3">
        <CardTitle class="text-base">About Kowalski</CardTitle>
      </CardHeader>
      <CardContent>
        <p class="text-sm text-muted-foreground mb-4">
          Kowalski is a multi-agent framework that extends Housaky with specialized AI agents. 
          Each subagent is optimized for specific task types and can work in parallel.
        </p>
        <div class="grid grid-cols-2 md:grid-cols-4 gap-2 text-xs">
          <div class="p-2 rounded bg-muted">
            <span class="font-semibold">Code</span>
            <p class="text-muted-foreground">Programming tasks</p>
          </div>
          <div class="p-2 rounded bg-muted">
            <span class="font-semibold">Web</span>
            <p class="text-muted-foreground">Research & fetching</p>
          </div>
          <div class="p-2 rounded bg-muted">
            <span class="font-semibold">Data</span>
            <p class="text-muted-foreground">Analysis & stats</p>
          </div>
          <div class="p-2 rounded bg-muted">
            <span class="font-semibold">Federation</span>
            <p class="text-muted-foreground">Multi-agent coord</p>
          </div>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
