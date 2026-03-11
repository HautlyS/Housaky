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
import Tabs from '@/components/ui/tabs.vue'
import TabsList from '@/components/ui/tabs-list.vue'
import TabsTrigger from '@/components/ui/tabs-trigger.vue'
import { 
  Key, 
  Plus, 
  Trash2, 
  RefreshCw,
  Eye,
  EyeOff,
  CheckCircle2,
  XCircle,
  AlertCircle,
  Loader2,
  Server,
  Bot,
  Copy,
  ToggleLeft,
  ToggleRight
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface KeyUsage {
  total_requests: number
  successful_requests: number
  failed_requests: number
  rate_limited_count: number
  tokens_used: number
}

interface ApiKey {
  id: string
  name: string
  key: string
  description: string
  enabled: boolean
  priority: number
  tags: string[]
  usage: KeyUsage
}

interface ProviderKeys {
  name: string
  keys: ApiKey[]
  enabled: boolean
  state: {
    is_healthy: boolean
    consecutive_failures: number
    consecutive_successes: number
    is_rate_limited: boolean
  }
}

interface SubAgentConfig {
  provider: string
  model: string
  key_name: string
  max_concurrent: number
  role: string
  awareness: string[]
}

interface KeysData {
  providers: Record<string, ProviderKeys>
  subagents: Record<string, SubAgentConfig>
}

const keysData = ref<KeysData | null>(null)
const loading = ref(true)
const activeTab = ref('providers')
const showKey = ref<Record<string, boolean>>({})
const showAddKey = ref(false)
const newKey = ref({ provider: 'modal', name: '', key: '', description: '' })

async function loadKeys() {
  loading.value = true
  try {
    if (isTauri) {
      keysData.value = await invoke<KeysData>('get_keys')
    }
  } catch (e) {
    console.error('Failed to load keys:', e)
  } finally {
    loading.value = false
  }
}

async function toggleKey(provider: string, keyId: string, enabled: boolean) {
  try {
    await invoke('toggle_key', { provider, keyId, enabled })
    await loadKeys()
  } catch (e) {
    console.error('Failed to toggle key:', e)
  }
}

async function deleteKey(provider: string, keyId: string) {
  if (!confirm('Are you sure you want to delete this key?')) return
  try {
    await invoke('delete_key', { provider, keyId })
    await loadKeys()
  } catch (e) {
    console.error('Failed to delete key:', e)
  }
}

async function addKey() {
  if (!newKey.value.name || !newKey.value.key) return
  try {
    await invoke('save_key', {
      provider: newKey.value.provider,
      key: {
        id: `${newKey.value.name.toLowerCase().replace(/\s+/g, '-')}-${Date.now()}`,
        name: newKey.value.name,
        key: newKey.value.key,
        description: newKey.value.description,
        enabled: true,
        priority: 1,
        tags: [],
        usage: {
          total_requests: 0,
          successful_requests: 0,
          failed_requests: 0,
          rate_limited_count: 0,
          tokens_used: 0
        }
      }
    })
    showAddKey.value = false
    newKey.value = { provider: 'modal', name: '', key: '', description: '' }
    await loadKeys()
  } catch (e) {
    console.error('Failed to add key:', e)
  }
}

function toggleShowKey(id: string) {
  showKey.value[id] = !showKey.value[id]
}

function maskKey(key: string): string {
  if (!key) return ''
  if (key.length <= 8) return '****'
  return key.slice(0, 4) + '****' + key.slice(-4)
}

const providers = computed(() => {
  if (!keysData.value) return []
  return Object.entries(keysData.value.providers).map(([name, data]) => ({
    name,
    ...data
  }))
})

const subagents = computed(() => {
  if (!keysData.value) return []
  return Object.entries(keysData.value.subagents).map(([name, config]) => ({
    name,
    ...config
  }))
})

onMounted(() => {
  loadKeys()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold tracking-tight flex items-center gap-2">
          <Key class="w-6 h-6 text-primary" />
          Keys & Subagents
        </h1>
        <p class="text-sm text-muted-foreground">
          Manage API keys and Kowalski subagents
        </p>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" @click="loadKeys" :disabled="loading">
          <RefreshCw :class="['w-4 h-4 mr-2', loading && 'animate-spin']" />
          Refresh
        </Button>
      </div>
    </div>

    <Tabs v-model="activeTab" defaultValue="providers">
      <TabsList>
        <TabsTrigger value="providers">
          <Server class="w-4 h-4 mr-2" />
          Providers
        </TabsTrigger>
        <TabsTrigger value="subagents">
          <Bot class="w-4 h-4 mr-2" />
          Subagents
        </TabsTrigger>
      </TabsList>
    </Tabs>

    <div v-if="loading" class="flex items-center justify-center py-12">
      <Loader2 class="w-8 h-8 animate-spin text-muted-foreground" />
    </div>

    <template v-else-if="activeTab === 'providers'">
      <div class="space-y-4">
        <div v-for="prov in providers" :key="prov.name" class="space-y-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="w-10 h-10 rounded-lg bg-gradient-to-br from-blue-500 to-cyan-500 flex items-center justify-center">
                <Server class="w-5 h-5 text-white" />
              </div>
              <div>
                <h3 class="font-semibold capitalize">{{ prov.name }}</h3>
                <p class="text-xs text-muted-foreground">{{ prov.keys.length }} key(s)</p>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <Badge :variant="prov.state.is_healthy ? 'success' : 'destructive'">
                {{ prov.state.is_healthy ? 'Healthy' : 'Unhealthy' }}
              </Badge>
              <Badge v-if="prov.state.is_rate_limited" variant="warning">
                Rate Limited
              </Badge>
            </div>
          </div>

          <div class="pl-13 space-y-2">
            <div v-for="key in prov.keys" :key="key.id"
              class="p-4 rounded-lg border bg-card hover:bg-accent/50 transition-colors"
            >
              <div class="flex items-start justify-between gap-4">
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2 mb-1">
                    <span class="font-medium">{{ key.name }}</span>
                    <Badge :variant="key.enabled ? 'success' : 'secondary'" class="text-xs">
                      {{ key.enabled ? 'Active' : 'Disabled' }}
                    </Badge>
                  </div>
                  <p class="text-xs text-muted-foreground mb-2">{{ key.description }}</p>
                  <div class="flex items-center gap-4 text-xs text-muted-foreground">
                    <span>{{ key.usage.total_requests }} requests</span>
                    <span>{{ key.usage.successful_requests }} success</span>
                    <span>{{ key.usage.failed_requests }} failed</span>
                    <span v-if="key.usage.tokens_used">{{ key.usage.tokens_used.toLocaleString() }} tokens</span>
                  </div>
                </div>
                <div class="flex items-center gap-2">
                  <code class="text-xs bg-muted px-2 py-1 rounded font-mono">
                    {{ showKey[key.id] ? key.key : maskKey(key.key) }}
                  </code>
                  <Button variant="ghost" size="icon" @click="toggleShowKey(key.id)">
                    <Eye v-if="!showKey[key.id]" class="w-4 h-4" />
                    <EyeOff v-else class="w-4 h-4" />
                  </Button>
                  <Button 
                    variant="ghost" 
                    size="icon" 
                    @click="toggleKey(prov.name, key.id, !key.enabled)"
                  >
                    <ToggleRight v-if="key.enabled" class="w-4 h-4 text-green-500" />
                    <ToggleLeft v-else class="w-4 h-4" />
                  </Button>
                  <Button variant="ghost" size="icon" @click="deleteKey(prov.name, key.id)">
                    <Trash2 class="w-4 h-4 text-destructive" />
                  </Button>
                </div>
              </div>
            </div>

            <Button v-if="showAddKey" variant="outline" size="sm" @click="showAddKey = false" class="w-full">
              Cancel
            </Button>
          </div>
        </div>

        <Card v-if="!providers.length && !loading">
          <CardContent class="py-8 text-center">
            <Key class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
            <h3 class="font-semibold mb-2">No API Keys Configured</h3>
            <p class="text-sm text-muted-foreground mb-4">
              Add API keys to start using Housaky with different providers
            </p>
            <Button @click="showAddKey = true">
              <Plus class="w-4 h-4 mr-2" />
              Add Key
            </Button>
          </CardContent>
        </Card>
      </div>

      <div v-if="showAddKey" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
        <Card class="w-full max-w-md">
          <CardHeader>
            <CardTitle>Add API Key</CardTitle>
            <CardDescription>Add a new API key to a provider</CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="space-y-2">
              <label class="text-sm font-medium">Provider</label>
              <select v-model="newKey.provider" class="w-full h-10 rounded-md border bg-background px-3">
                <option value="modal">Modal</option>
                <option value="openrouter">OpenRouter</option>
              </select>
            </div>
            <div class="space-y-2">
              <label class="text-sm font-medium">Name</label>
              <Input v-model="newKey.name" placeholder="e.g., Main Key" />
            </div>
            <div class="space-y-2">
              <label class="text-sm font-medium">API Key</label>
              <Input v-model="newKey.key" type="password" placeholder="sk-..." />
            </div>
            <div class="space-y-2">
              <label class="text-sm font-medium">Description (optional)</label>
              <Input v-model="newKey.description" placeholder="e.g., Production key" />
            </div>
          </CardContent>
          <CardContent class="flex justify-end gap-2">
            <Button variant="outline" @click="showAddKey = false">Cancel</Button>
            <Button @click="addKey" :disabled="!newKey.name || !newKey.key">
              <Plus class="w-4 h-4 mr-2" />
              Add Key
            </Button>
          </CardContent>
        </Card>
      </div>
    </template>

    <template v-else-if="activeTab === 'subagents'">
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <Card v-for="subagent in subagents" :key="subagent.name">
          <CardHeader class="pb-3">
            <div class="flex items-start justify-between">
              <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-lg bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center">
                  <Bot class="w-5 h-5 text-white" />
                </div>
                <div>
                  <CardTitle class="text-base">{{ subagent.name }}</CardTitle>
                  <CardDescription class="capitalize">{{ subagent.role }}</CardDescription>
                </div>
              </div>
              <Badge variant="success">Enabled</Badge>
            </div>
          </CardHeader>
          <CardContent class="space-y-3">
            <div class="space-y-1">
              <p class="text-xs text-muted-foreground">Provider</p>
              <p class="text-sm font-mono">{{ subagent.provider }}</p>
            </div>
            <div class="space-y-1">
              <p class="text-xs text-muted-foreground">Model</p>
              <p class="text-sm font-mono">{{ subagent.model }}</p>
            </div>
            <div class="space-y-1">
              <p class="text-xs text-muted-foreground">Max Concurrent</p>
              <p class="text-sm">{{ subagent.max_concurrent }}</p>
            </div>
            <div class="flex flex-wrap gap-1">
              <Badge v-for="aw in subagent.awareness" :key="aw" variant="outline" class="text-xs">
                {{ aw }}
              </Badge>
            </div>
          </CardContent>
        </Card>

        <Card v-if="!subagents.length && !loading">
          <CardContent class="py-8 text-center">
            <Bot class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
            <h3 class="font-semibold mb-2">No Subagents Configured</h3>
            <p class="text-sm text-muted-foreground">
              Configure Kowalski subagents to delegate tasks
            </p>
          </CardContent>
        </Card>
      </div>
    </template>
  </div>
</template>
