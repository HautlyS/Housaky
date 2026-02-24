<script setup lang="ts">
import { ref, onMounted, computed, reactive } from 'vue'
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
  Settings, 
  Save,
  RotateCcw,
  Eye,
  EyeOff,
  Copy,
  Download,
  Upload,
  RefreshCw,
  CheckCircle2,
  AlertCircle
} from 'lucide-vue-next'

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
const showSecret = ref<Record<string, boolean>>({})
const configPath = ref('')
const originalConfig = ref<string>('')

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
  
  try {
    const result = await invoke<{ version: string; config: string }>('get_status')
    configPath.value = result.config
    
    // Get full config
    const fullConfig = await invoke<HousakyConfig>('get_config')
    if (fullConfig) {
      config.value = { ...config.value, ...fullConfig }
    }
    
    // Store original for comparison
    originalConfig.value = JSON.stringify(config.value)
  } catch (e) {
    error.value = String(e)
    console.error('Failed to load config:', e)
  } finally {
    loading.value = false
  }
}

async function saveConfig() {
  saving.value = true
  error.value = ''
  success.value = ''
  
  try {
    await invoke<string>('save_config', { config: config.value })
    success.value = 'Configuration saved successfully!'
    originalConfig.value = JSON.stringify(config.value)
    
    // Clear success message after 3 seconds
    setTimeout(() => {
      success.value = ''
    }, 3000)
  } catch (e) {
    error.value = String(e)
  } finally {
    saving.value = false
  }
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

const sections = computed(() => [
  {
    id: 'general',
    name: 'General',
    description: 'Provider and model settings',
    fields: [
      { key: 'api_key', path: 'api_key', label: 'API Key', type: 'password' as const, description: 'Your API key for the default provider' },
      { key: 'default_provider', path: 'default_provider', label: 'Default Provider', type: 'select' as const, options: ['openrouter', 'anthropic', 'openai', 'ollama', 'gemini', 'groq', 'mistral', 'deepseek'] },
      { key: 'default_model', path: 'default_model', label: 'Default Model', type: 'text' as const, description: 'Model ID (optional, uses provider default if empty)' },
      { key: 'default_temperature', path: 'default_temperature', label: 'Temperature', type: 'number' as const, description: 'Sampling temperature (0-2)', min: 0, max: 2, step: 0.1 },
    ]
  },
  {
    id: 'memory',
    name: 'Memory',
    description: 'Memory backend configuration',
    fields: [
      { key: 'backend', path: 'memory.backend', label: 'Backend', type: 'select' as const, options: ['sqlite', 'lucid', 'markdown', 'none'] },
      { key: 'auto_save', path: 'memory.auto_save', label: 'Auto Save', type: 'boolean' as const, description: 'Automatically save memory' },
      { key: 'embedding_provider', path: 'memory.embedding_provider', label: 'Embedding Provider', type: 'select' as const, options: ['openai', 'noop'] },
      { key: 'vector_weight', path: 'memory.vector_weight', label: 'Vector Weight', type: 'number' as const, description: 'Weight for vector search (0-1)', min: 0, max: 1, step: 0.1 },
      { key: 'keyword_weight', path: 'memory.keyword_weight', label: 'Keyword Weight', type: 'number' as const, description: 'Weight for keyword search (0-1)', min: 0, max: 1, step: 0.1 },
    ]
  },
  {
    id: 'autonomy',
    name: 'Autonomy',
    description: 'Agent autonomy and security settings',
    fields: [
      { key: 'level', path: 'autonomy.level', label: 'Autonomy Level', type: 'select' as const, options: ['readonly', 'supervised', 'full'] },
      { key: 'workspace_only', path: 'autonomy.workspace_only', label: 'Workspace Only', type: 'boolean' as const, description: 'Restrict file access to workspace' },
      { key: 'max_actions_per_hour', path: 'autonomy.max_actions_per_hour', label: 'Max Actions/Hour', type: 'number' as const, min: 1, max: 10000 },
      { key: 'max_cost_per_day_cents', path: 'autonomy.max_cost_per_day_cents', label: 'Max Cost/Day (cents)', type: 'number' as const, min: 0, max: 1000000 },
    ]
  },
  {
    id: 'runtime',
    name: 'Runtime',
    description: 'Execution runtime settings',
    fields: [
      { key: 'kind', path: 'runtime.kind', label: 'Runtime', type: 'select' as const, options: ['native', 'docker'] },
    ]
  },
  {
    id: 'heartbeat',
    name: 'Heartbeat',
    description: 'Periodic task configuration',
    fields: [
      { key: 'enabled', path: 'heartbeat.enabled', label: 'Enabled', type: 'boolean' as const },
      { key: 'interval_minutes', path: 'heartbeat.interval_minutes', label: 'Interval (minutes)', type: 'number' as const, min: 1, max: 1440 },
    ]
  },
  {
    id: 'gateway',
    name: 'Gateway',
    description: 'Webhook server settings',
    fields: [
      { key: 'require_pairing', path: 'gateway.require_pairing', label: 'Require Pairing', type: 'boolean' as const, description: 'Require pairing code on first connect' },
      { key: 'allow_public_bind', path: 'gateway.allow_public_bind', label: 'Allow Public Bind', type: 'boolean' as const, description: 'Allow binding to 0.0.0.0' },
    ]
  },
  {
    id: 'tunnel',
    name: 'Tunnel',
    description: 'Tunnel provider for public access',
    fields: [
      { key: 'provider', path: 'tunnel.provider', label: 'Provider', type: 'select' as const, options: ['none', 'cloudflare', 'tailscale', 'ngrok'] },
    ]
  },
  {
    id: 'secrets',
    name: 'Secrets',
    description: 'Secret management',
    fields: [
      { key: 'encrypt', path: 'secrets.encrypt', label: 'Encrypt Secrets', type: 'boolean' as const, description: 'Encrypt API keys with local key' },
    ]
  },
])

const activeSection = ref('general')

const currentSection = computed(() => 
  sections.value.find(s => s.id === activeSection.value) || sections.value[0]
)

function getValue(path: string): any {
  return path.split('.').reduce((obj: any, key) => obj?.[key], config.value)
}

function setValue(path: string, value: any) {
  const keys = path.split('.')
  let obj: any = config.value
  for (let i = 0; i < keys.length - 1; i++) {
    obj = obj[keys[i]]
  }
  obj[keys[keys.length - 1]] = value
}

onMounted(() => {
  loadConfig()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-bold">Configuration</h1>
        <p class="text-muted-foreground">Manage Housaky settings</p>
      </div>
      <div class="flex gap-2">
        <Button variant="outline" @click="loadConfig" :disabled="loading">
          <RefreshCw class="w-4 h-4 mr-2" :class="{ 'animate-spin': loading }" />
          Reload
        </Button>
        <Button variant="outline" @click="copyConfig">
          <Copy class="w-4 h-4 mr-2" />
          Copy
        </Button>
      </div>
    </div>

    <!-- Messages -->
    <div v-if="error" class="p-4 rounded-lg bg-destructive/10 text-destructive flex items-center gap-2">
      <AlertCircle class="w-5 h-5" />
      {{ error }}
    </div>
    
    <div v-if="success" class="p-4 rounded-lg bg-green-500/10 text-green-600 flex items-center gap-2">
      <CheckCircle2 class="w-5 h-5" />
      {{ success }}
    </div>

    <!-- Config Path -->
    <Card>
      <CardContent class="pt-6">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Settings class="w-5 h-5 text-muted-foreground" />
            <span class="text-sm text-muted-foreground">Config file:</span>
            <code class="text-sm">{{ configPath || 'Loading...' }}</code>
          </div>
          <Badge :variant="hasChanges() ? 'warning' : 'success'">
            {{ hasChanges() ? 'Modified' : 'Saved' }}
          </Badge>
        </div>
      </CardContent>
    </Card>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-12">
      <RefreshCw class="w-8 h-8 animate-spin text-muted-foreground" />
    </div>

    <template v-else>
      <div class="grid grid-cols-1 lg:grid-cols-4 gap-6">
        <!-- Sidebar -->
        <Card class="lg:col-span-1 h-fit">
          <CardHeader>
            <CardTitle>Sections</CardTitle>
          </CardHeader>
          <CardContent class="p-2">
            <div class="space-y-1">
              <button
                v-for="section in sections"
                :key="section.id"
                @click="activeSection = section.id"
                class="w-full text-left px-3 py-2 rounded-lg text-sm transition-colors"
                :class="activeSection === section.id ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'"
              >
                {{ section.name }}
              </button>
            </div>
          </CardContent>
        </Card>

        <!-- Content -->
        <Card class="lg:col-span-3">
          <CardHeader>
            <CardTitle>{{ currentSection.name }}</CardTitle>
            <CardDescription>{{ currentSection.description }}</CardDescription>
          </CardHeader>
          <CardContent class="space-y-6">
            <div 
              v-for="field in currentSection.fields" 
              :key="field.path"
              class="space-y-2"
            >
              <label class="text-sm font-medium">{{ field.label }}</label>
              
              <!-- Text Input -->
              <Input
                v-if="field.type === 'text'"
                :model-value="getValue(field.path)"
                @update:model-value="setValue(field.path, $event)"
                :placeholder="field.label"
              />
              
              <!-- Password Input -->
              <div v-else-if="field.type === 'password'" class="relative">
                <Input
                  :model-value="getValue(field.path) || ''"
                  @update:model-value="setValue(field.path, $event)"
                  :type="showSecret[field.path] ? 'text' : 'password'"
                  :placeholder="field.label"
                />
                <button
                  @click="toggleSecret(field.path)"
                  class="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground"
                >
                  <Eye v-if="!showSecret[field.path]" class="w-4 h-4" />
                  <EyeOff v-else class="w-4 h-4" />
                </button>
              </div>
              
              <!-- Number Input -->
              <Input
                v-else-if="field.type === 'number'"
                :model-value="getValue(field.path)"
                @update:model-value="setValue(field.path, Number($event))"
                type="number"
                :step="field.step || 1"
                :min="field.min"
                :max="field.max"
              />
              
              <!-- Select -->
              <select
                v-else-if="field.type === 'select'"
                :model-value="getValue(field.path)"
                @update:model-value="setValue(field.path, $event)"
                class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
              >
                <option v-for="opt in field.options" :key="opt" :value="opt">{{ opt }}</option>
              </select>
              
              <!-- Boolean -->
              <label v-else-if="field.type === 'boolean'" class="flex items-center gap-2">
                <input
                  type="checkbox"
                  :checked="getValue(field.path)"
                  @change="setValue(field.path, ($event.target as HTMLInputElement).checked)"
                  class="w-4 h-4 rounded border-gray-300"
                />
                <span class="text-sm text-muted-foreground">{{ field.description }}</span>
              </label>
              
              <p v-if="field.description && field.type !== 'boolean'" class="text-xs text-muted-foreground">
                {{ field.description }}
              </p>
            </div>

            <!-- Actions -->
            <div class="flex justify-between pt-4 border-t">
              <Button variant="outline" @click="resetToOriginal" :disabled="!hasChanges()">
                <RotateCcw class="w-4 h-4 mr-2" />
                Reset Changes
              </Button>
              <Button @click="saveConfig" :disabled="saving || !hasChanges()">
                <Save class="w-4 h-4 mr-2" />
                {{ saving ? 'Saving...' : 'Save Changes' }}
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </template>
  </div>
</template>
