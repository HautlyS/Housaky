<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardDescription from '@/components/ui/card-description.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import Button from '@/components/ui/button.vue'
import Input from '@/components/ui/input.vue'
import Badge from '@/components/ui/badge.vue'
import Textarea from '@/components/ui/textarea.vue'
import { 
  Settings, 
  Save,
  RotateCcw,
  Eye,
  EyeOff,
  Copy,
  RefreshCw,
  CheckCircle2,
  AlertCircle,
  Loader2,
  FileText,
  Folder,
  Shield,
  Cpu,
  Database,
  Radio,
  Globe
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface HousakyConfig {
  api_key?: string
  default_provider: string
  default_model?: string
  default_temperature: number
  memory: {
    backend: string
    auto_save: boolean
    embedding_provider: string
    vector_weight: number
    keyword_weight: number
  }
  autonomy: {
    level: string
    workspace_only: boolean
    allowed_commands: string[]
    forbidden_paths: string[]
    max_actions_per_hour: number
    max_cost_per_day_cents: number
  }
  runtime: {
    kind: string
  }
  heartbeat: {
    enabled: boolean
    interval_minutes: number
  }
  gateway: {
    require_pairing: boolean
    allow_public_bind: boolean
  }
  tunnel: {
    provider: string
  }
  secrets: {
    encrypt: boolean
  }
}

const loading = ref(true)
const saving = ref(false)
const error = ref('')
const success = ref('')
const warnings = ref<string[]>([])
const showSecret = ref<Record<string, boolean>>({})
const configPath = ref('')
const originalConfig = ref<string>('')
const autoSaveTimer = ref<number | null>(null)
const lastSaved = ref<Date | null>(null)

const config = ref<HousakyConfig>({
  default_provider: 'openrouter',
  default_temperature: 0.7,
  memory: {
    backend: 'sqlite',
    auto_save: true,
    embedding_provider: 'openai',
    vector_weight: 0.7,
    keyword_weight: 0.3,
  },
  autonomy: {
    level: 'supervised',
    workspace_only: true,
    allowed_commands: [],
    forbidden_paths: [],
    max_actions_per_hour: 100,
    max_cost_per_day_cents: 1000,
  },
  runtime: {
    kind: 'native',
  },
  heartbeat: {
    enabled: false,
    interval_minutes: 30,
  },
  gateway: {
    require_pairing: true,
    allow_public_bind: false,
  },
  tunnel: {
    provider: 'none',
  },
  secrets: {
    encrypt: true,
  },
})

async function loadConfig() {
  loading.value = true
  error.value = ''
  
  if (!isTauri) {
    error.value = 'Running in server mode - config not available'
    loading.value = false
    return
  }
  
  try {
    const status = await invoke<{ version: string; config: string }>('get_status')
    configPath.value = status.config
    
    const fullConfig = await invoke<HousakyConfig>('get_config')
    if (fullConfig) {
      config.value = { ...config.value, ...fullConfig }
    }
    
    originalConfig.value = JSON.stringify(config.value)
  } catch (e) {
    error.value = String(e)
    console.error('Failed to load config:', e)
  } finally {
    loading.value = false
  }
}

async function validateAndWarn() {
  if (!isTauri) return
  try {
    const result = await invoke<string[]>('validate_config', { config: config.value })
    warnings.value = Array.isArray(result) ? result : []
  } catch { warnings.value = [] }
}

async function saveConfig() {
  saving.value = true
  error.value = ''
  success.value = ''

  if (!isTauri) {
    error.value = 'Running in server mode — cannot save config'
    saving.value = false
    return
  }

  try {
    await validateAndWarn()
    await invoke<string>('save_config', { config: config.value })
    success.value = 'Configuration saved!'
    originalConfig.value = JSON.stringify(config.value)
    lastSaved.value = new Date()
    setTimeout(() => { success.value = '' }, 3000)
  } catch (e) {
    error.value = String(e)
  } finally {
    saving.value = false
  }
}

function scheduleAutoValidate() {
  if (autoSaveTimer.value) clearTimeout(autoSaveTimer.value)
  autoSaveTimer.value = window.setTimeout(validateAndWarn, 800)
}

function resetToOriginal() {
  if (originalConfig.value) {
    config.value = JSON.parse(originalConfig.value)
  }
}

function toggleSecret(key: string) {
  showSecret.value[key] = !showSecret.value[key]
}

function copyConfig() {
  navigator.clipboard.writeText(JSON.stringify(config.value, null, 2))
  success.value = 'Config copied to clipboard!'
  setTimeout(() => success.value = '', 3000)
}

function hasChanges(): boolean {
  return JSON.stringify(config.value) !== originalConfig.value
}

const sections = [
  { id: 'general', name: 'General', icon: Settings, description: 'Provider and model settings' },
  { id: 'memory', name: 'Memory', icon: Database, description: 'Memory backend configuration' },
  { id: 'autonomy', name: 'Autonomy', icon: Shield, description: 'Agent autonomy and security' },
  { id: 'runtime', name: 'Runtime', icon: Cpu, description: 'Execution runtime settings' },
  { id: 'gateway', name: 'Gateway', icon: Globe, description: 'Webhook server settings' },
  { id: 'tunnel', name: 'Tunnel', icon: Radio, description: 'Tunnel provider for public access' },
]

const activeSection = ref('general')

watch(() => JSON.stringify(config.value), scheduleAutoValidate)

const providerOptions = ['openrouter', 'anthropic', 'openai', 'ollama', 'gemini', 'groq', 'mistral', 'deepseek']
const memoryBackends = ['sqlite', 'lucid', 'markdown', 'none']
const autonomyLevels = ['readonly', 'supervised', 'full']
const runtimeKinds = ['native', 'docker']
const tunnelProviders = ['none', 'cloudflare', 'tailscale', 'ngrok']

onMounted(() => {
  loadConfig()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h1 class="text-2xl font-bold tracking-tight flex items-center gap-2">
          <Settings class="w-6 h-6 text-primary" />
          Configuration
        </h1>
        <p class="text-sm text-muted-foreground">
          Async config editor
          <span v-if="lastSaved" class="ml-2 text-green-600">· saved {{ lastSaved.toLocaleTimeString() }}</span>
        </p>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" @click="loadConfig" :disabled="loading">
          <RefreshCw :class="['w-4 h-4 mr-1.5', loading && 'animate-spin']" />
          Reload
        </Button>
        <Button variant="outline" size="sm" @click="copyConfig">
          <Copy class="w-4 h-4 mr-1.5" />
          Copy
        </Button>
        <Button size="sm" @click="saveConfig" :disabled="saving || !hasChanges()" class="gap-1.5">
          <Loader2 v-if="saving" class="w-4 h-4 animate-spin" />
          <Save v-else class="w-4 h-4" />
          {{ saving ? 'Saving…' : 'Save' }}
        </Button>
      </div>
    </div>

    <div v-if="error" class="p-3 rounded-lg bg-destructive/10 text-destructive flex items-center gap-2 text-sm">
      <AlertCircle class="w-4 h-4 flex-shrink-0" />{{ error }}
    </div>

    <div v-if="success" class="p-3 rounded-lg bg-green-500/10 text-green-700 flex items-center gap-2 text-sm">
      <CheckCircle2 class="w-4 h-4 flex-shrink-0" />{{ success }}
    </div>

    <div v-if="warnings.length > 0" class="space-y-1.5">
      <div v-for="(w, i) in warnings" :key="i"
        class="p-3 rounded-lg bg-yellow-500/10 border border-yellow-400/40 text-yellow-700 dark:text-yellow-400 flex items-start gap-2 text-sm"
      >
        <AlertCircle class="w-4 h-4 flex-shrink-0 mt-0.5" />
        <span>{{ w }}</span>
      </div>
    </div>

    <Card>
      <CardContent class="pt-6">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <FileText class="w-5 h-5 text-muted-foreground" />
            <span class="text-sm text-muted-foreground">Config file:</span>
            <code class="text-sm bg-muted px-2 py-1 rounded">{{ configPath || 'Loading...' }}</code>
          </div>
          <Badge :variant="hasChanges() ? 'warning' : 'success'">
            {{ hasChanges() ? 'Modified' : 'Saved' }}
          </Badge>
        </div>
      </CardContent>
    </Card>

    <div v-if="loading" class="flex items-center justify-center py-12">
      <Loader2 class="w-8 h-8 animate-spin text-muted-foreground" />
    </div>

    <div v-else class="grid grid-cols-1 lg:grid-cols-4 gap-6">
      <Card class="lg:col-span-1 h-fit">
        <CardHeader class="py-4">
          <CardTitle class="text-sm">Sections</CardTitle>
        </CardHeader>
        <CardContent class="p-2">
          <div class="space-y-1">
            <button
              v-for="section in sections"
              :key="section.id"
              @click="activeSection = section.id"
              :class="[
                'w-full text-left px-3 py-2.5 rounded-lg text-sm transition-colors flex items-center gap-2',
                activeSection === section.id 
                  ? 'bg-primary text-primary-foreground' 
                  : 'hover:bg-muted'
              ]"
            >
              <component :is="section.icon" class="w-4 h-4" />
              {{ section.name }}
            </button>
          </div>
        </CardContent>
      </Card>

      <Card class="lg:col-span-3">
        <template v-if="activeSection === 'general'">
          <CardHeader>
            <CardTitle class="flex items-center gap-2">
              <Settings class="w-5 h-5" />
              General Settings
            </CardTitle>
            <CardDescription>Provider and model configuration</CardDescription>
          </CardHeader>
          <CardContent class="space-y-6">
            <div class="space-y-2">
              <label class="text-sm font-medium">API Key</label>
              <div class="relative">
                <Input
                  v-model="config.api_key"
                  :type="showSecret['api_key'] ? 'text' : 'password'"
                  placeholder="Enter your API key"
                />
                <button
                  @click="toggleSecret('api_key')"
                  class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground"
                >
                  <Eye v-if="!showSecret['api_key']" class="w-4 h-4" />
                  <EyeOff v-else class="w-4 h-4" />
                </button>
              </div>
            </div>

            <div class="grid md:grid-cols-2 gap-4">
              <div class="space-y-2">
                <label class="text-sm font-medium">Default Provider</label>
                <select v-model="config.default_provider" class="w-full h-10 rounded-md border bg-background px-3">
                  <option v-for="p in providerOptions" :key="p" :value="p">{{ p }}</option>
                </select>
              </div>
              <div class="space-y-2">
                <label class="text-sm font-medium">Default Model</label>
                <Input v-model="config.default_model" placeholder="Optional, uses provider default" />
              </div>
            </div>

            <div class="space-y-2">
              <label class="text-sm font-medium">Temperature: {{ config.default_temperature }}</label>
              <input 
                type="range" 
                v-model.number="config.default_temperature" 
                min="0" 
                max="2" 
                step="0.1"
                class="w-full"
              />
              <p class="text-xs text-muted-foreground">Controls randomness (0 = deterministic, 2 = very creative)</p>
            </div>
          </CardContent>
        </template>

        <template v-else-if="activeSection === 'memory'">
          <CardHeader>
            <CardTitle class="flex items-center gap-2">
              <Database class="w-5 h-5" />
              Memory Settings
            </CardTitle>
            <CardDescription>Memory backend configuration</CardDescription>
          </CardHeader>
          <CardContent class="space-y-6">
            <div class="grid md:grid-cols-2 gap-4">
              <div class="space-y-2">
                <label class="text-sm font-medium">Backend</label>
                <select v-model="config.memory.backend" class="w-full h-10 rounded-md border bg-background px-3">
                  <option v-for="b in memoryBackends" :key="b" :value="b">{{ b }}</option>
                </select>
              </div>
              <div class="space-y-2">
                <label class="text-sm font-medium">Embedding Provider</label>
                <select v-model="config.memory.embedding_provider" class="w-full h-10 rounded-md border bg-background px-3">
                  <option value="openai">OpenAI</option>
                  <option value="noop">None</option>
                </select>
              </div>
            </div>

            <div class="flex items-center gap-3">
              <input 
                type="checkbox" 
                v-model="config.memory.auto_save"
                class="w-4 h-4 rounded"
              />
              <label class="text-sm">Auto-save memory</label>
            </div>

            <div class="grid md:grid-cols-2 gap-4">
              <div class="space-y-2">
                <label class="text-sm font-medium">Vector Weight: {{ config.memory.vector_weight }}</label>
                <input 
                  type="range" 
                  v-model.number="config.memory.vector_weight" 
                  min="0" 
                  max="1" 
                  step="0.1"
                  class="w-full"
                />
              </div>
              <div class="space-y-2">
                <label class="text-sm font-medium">Keyword Weight: {{ config.memory.keyword_weight }}</label>
                <input 
                  type="range" 
                  v-model.number="config.memory.keyword_weight" 
                  min="0" 
                  max="1" 
                  step="0.1"
                  class="w-full"
                />
              </div>
            </div>
          </CardContent>
        </template>

        <template v-else-if="activeSection === 'autonomy'">
          <CardHeader>
            <CardTitle class="flex items-center gap-2">
              <Shield class="w-5 h-5" />
              Autonomy & Security
            </CardTitle>
            <CardDescription>Agent autonomy and security settings</CardDescription>
          </CardHeader>
          <CardContent class="space-y-6">
            <div class="space-y-2">
              <label class="text-sm font-medium">Autonomy Level</label>
              <select v-model="config.autonomy.level" class="w-full h-10 rounded-md border bg-background px-3">
                <option v-for="l in autonomyLevels" :key="l" :value="l">{{ l }}</option>
              </select>
              <p class="text-xs text-muted-foreground">
                {{ config.autonomy.level === 'readonly' ? 'Agent can only read files' : 
                   config.autonomy.level === 'supervised' ? 'Agent asks permission for actions' : 
                   'Agent can act autonomously' }}
              </p>
            </div>

            <div class="flex items-center gap-3">
              <input 
                type="checkbox" 
                v-model="config.autonomy.workspace_only"
                class="w-4 h-4 rounded"
              />
              <div>
                <label class="text-sm font-medium">Workspace Only</label>
                <p class="text-xs text-muted-foreground">Restrict file access to workspace directory</p>
              </div>
            </div>

            <div class="grid md:grid-cols-2 gap-4">
              <div class="space-y-2">
                <label class="text-sm font-medium">Max Actions/Hour</label>
                <Input type="number" v-model.number="config.autonomy.max_actions_per_hour" />
              </div>
              <div class="space-y-2">
                <label class="text-sm font-medium">Max Cost/Day (cents)</label>
                <Input type="number" v-model.number="config.autonomy.max_cost_per_day_cents" />
              </div>
            </div>
          </CardContent>
        </template>

        <template v-else-if="activeSection === 'runtime'">
          <CardHeader>
            <CardTitle class="flex items-center gap-2">
              <Cpu class="w-5 h-5" />
              Runtime Settings
            </CardTitle>
            <CardDescription>Execution runtime configuration</CardDescription>
          </CardHeader>
          <CardContent class="space-y-6">
            <div class="space-y-2">
              <label class="text-sm font-medium">Runtime</label>
              <select v-model="config.runtime.kind" class="w-full h-10 rounded-md border bg-background px-3">
                <option v-for="r in runtimeKinds" :key="r" :value="r">{{ r }}</option>
              </select>
              <p class="text-xs text-muted-foreground">
                {{ config.runtime.kind === 'native' ? 'Run directly on host system' : 'Run in Docker container' }}
              </p>
            </div>

            <div class="space-y-2">
              <label class="text-sm font-medium">Heartbeat</label>
              <div class="flex items-center gap-4">
                <label class="flex items-center gap-2">
                  <input type="checkbox" v-model="config.heartbeat.enabled" class="w-4 h-4 rounded" />
                  <span class="text-sm">Enabled</span>
                </label>
                <Input 
                  v-if="config.heartbeat.enabled"
                  type="number" 
                  v-model.number="config.heartbeat.interval_minutes"
                  placeholder="Minutes"
                  class="w-32"
                />
                <span v-if="config.heartbeat.enabled" class="text-sm text-muted-foreground">minutes</span>
              </div>
            </div>
          </CardContent>
        </template>

        <template v-else-if="activeSection === 'gateway'">
          <CardHeader>
            <CardTitle class="flex items-center gap-2">
              <Globe class="w-5 h-5" />
              Gateway Settings
            </CardTitle>
            <CardDescription>Webhook server configuration</CardDescription>
          </CardHeader>
          <CardContent class="space-y-6">
            <div class="flex items-center gap-3">
              <input 
                type="checkbox" 
                v-model="config.gateway.require_pairing"
                class="w-4 h-4 rounded"
              />
              <div>
                <label class="text-sm font-medium">Require Pairing</label>
                <p class="text-xs text-muted-foreground">Require pairing code on first connection</p>
              </div>
            </div>

            <div class="flex items-center gap-3">
              <input 
                type="checkbox" 
                v-model="config.gateway.allow_public_bind"
                class="w-4 h-4 rounded"
              />
              <div>
                <label class="text-sm font-medium">Allow Public Bind</label>
                <p class="text-xs text-muted-foreground">Allow binding to 0.0.0.0 (security risk)</p>
              </div>
            </div>

            <div class="flex items-center gap-3">
              <input 
                type="checkbox" 
                v-model="config.secrets.encrypt"
                class="w-4 h-4 rounded"
              />
              <div>
                <label class="text-sm font-medium">Encrypt Secrets</label>
                <p class="text-xs text-muted-foreground">Encrypt API keys with local key</p>
              </div>
            </div>
          </CardContent>
        </template>

        <template v-else-if="activeSection === 'tunnel'">
          <CardHeader>
            <CardTitle class="flex items-center gap-2">
              <Radio class="w-5 h-5" />
              Tunnel Settings
            </CardTitle>
            <CardDescription>Tunnel provider for public access</CardDescription>
          </CardHeader>
          <CardContent class="space-y-6">
            <div class="space-y-2">
              <label class="text-sm font-medium">Tunnel Provider</label>
              <select v-model="config.tunnel.provider" class="w-full h-10 rounded-md border bg-background px-3">
                <option v-for="t in tunnelProviders" :key="t" :value="t">{{ t }}</option>
              </select>
              <p class="text-xs text-muted-foreground">
                Tunnel providers allow public access to your local Housaky instance
              </p>
            </div>
          </CardContent>
        </template>

        <div class="flex justify-between p-6 border-t">
          <Button variant="outline" @click="resetToOriginal" :disabled="!hasChanges()">
            <RotateCcw class="w-4 h-4 mr-2" />
            Reset Changes
          </Button>
          <Button @click="saveConfig" :disabled="saving || !hasChanges()">
            <Save class="w-4 h-4 mr-2" />
            {{ saving ? 'Saving...' : 'Save Changes' }}
          </Button>
        </div>
      </Card>
    </div>
  </div>
</template>
