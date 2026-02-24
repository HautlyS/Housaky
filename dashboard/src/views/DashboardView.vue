<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useRouter } from 'vue-router'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardDescription from '@/components/ui/card-description.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import Button from '@/components/ui/button.vue'
import Badge from '@/components/ui/badge.vue'
import { 
  Activity,
  Cpu,
  HardDrive,
  MessageSquare,
  Network,
  Wrench,
  Zap,
  CheckCircle2,
  AlertCircle,
  RefreshCw,
  Settings,
  Shield,
  Heart
} from 'lucide-vue-next'

interface HousakyStatus {
  version: string
  workspace: string
  config: string
  provider: string
  model: string
  temperature: number
  memory_backend: string
  memory_auto_save: boolean
  embedding_provider: string
  autonomy_level: string
  workspace_only: boolean
  runtime: string
  heartbeat_enabled: boolean
  heartbeat_interval: number
  channels: Record<string, { configured: boolean; active: boolean; allowlist_count: number }>
  secrets_encrypted: boolean
}

const router = useRouter()
const status = ref<HousakyStatus | null>(null)
const loading = ref(true)
const error = ref('')
const housakyInstalled = ref(false)

async function loadStatus() {
  loading.value = true
  error.value = ''
  
  try {
    housakyInstalled.value = await invoke<boolean>('check_housaky_installed')
    status.value = await invoke<HousakyStatus>('get_status')
  } catch (e) {
    error.value = String(e)
    console.error('Failed to load status:', e)
  } finally {
    loading.value = false
  }
}

async function runDoctor() {
  try {
    const result = await invoke<string>('run_doctor')
    alert(result || 'Doctor check completed')
  } catch (e) {
    alert(`Error: ${e}`)
  }
}

const channelList = computed(() => {
  if (!status.value?.channels) return []
  return Object.entries(status.value.channels).map(([name, data]) => ({
    name: name.charAt(0).toUpperCase() + name.slice(1),
    ...data
  }))
})

function navigate(path: string) {
  router.push(path)
}

onMounted(() => {
  loadStatus()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-bold">Dashboard</h1>
        <p class="text-muted-foreground">Monitor your Housaky AI Assistant</p>
      </div>
      <Button @click="loadStatus" :disabled="loading">
        <RefreshCw v-if="!loading" class="w-4 h-4 mr-2" :class="{ 'animate-spin': loading }" />
        <span v-if="loading">Loading...</span>
        <span v-else>Refresh</span>
      </Button>
    </div>

    <!-- Not Installed Warning -->
    <Card v-if="!housakyInstalled && !loading" class="border-yellow-500 bg-yellow-50 dark:bg-yellow-900/20">
      <CardContent class="pt-6">
        <div class="flex items-center gap-2 text-yellow-600 dark:text-yellow-400">
          <AlertCircle class="w-5 h-5" />
          <span>Housaky is not installed. Install it to enable full functionality.</span>
        </div>
      </CardContent>
    </Card>

    <!-- Error State -->
    <Card v-if="error" class="border-destructive bg-destructive/10">
      <CardContent class="pt-6">
        <div class="flex items-center gap-2 text-destructive">
          <AlertCircle class="w-5 h-5" />
          <span>{{ error }}</span>
        </div>
      </CardContent>
    </Card>

    <!-- Loading State -->
    <div v-if="loading" class="flex items-center justify-center py-12">
      <RefreshCw class="w-8 h-8 animate-spin text-muted-foreground" />
    </div>

    <template v-else-if="status">
      <!-- Status Overview Cards -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <CardHeader class="flex flex-row items-center justify-between pb-2">
            <CardTitle class="text-sm font-medium">Version</CardTitle>
            <Zap class="w-4 h-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold">{{ status.version }}</div>
            <p class="text-xs text-muted-foreground">Housaky AI</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="flex flex-row items-center justify-between pb-2">
            <CardTitle class="text-sm font-medium">Provider</CardTitle>
            <Cpu class="w-4 h-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold">{{ status.provider }}</div>
            <p class="text-xs text-muted-foreground">{{ status.model }}</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="flex flex-row items-center justify-between pb-2">
            <CardTitle class="text-sm font-medium">Memory</CardTitle>
            <HardDrive class="w-4 h-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold">{{ status.memory_backend }}</div>
            <p class="text-xs text-muted-foreground">
              {{ status.embedding_provider }} | Auto: {{ status.memory_auto_save ? 'on' : 'off' }}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader class="flex flex-row items-center justify-between pb-2">
            <CardTitle class="text-sm font-medium">Runtime</CardTitle>
            <Activity class="w-4 h-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div class="text-2xl font-bold">{{ status.runtime }}</div>
            <p class="text-xs text-muted-foreground">
              <Heart class="w-3 h-3 inline mr-1" />
              {{ status.heartbeat_enabled ? `Every ${status.heartbeat_interval}min` : 'disabled' }}
            </p>
          </CardContent>
        </Card>
      </div>

      <!-- Two Column Layout -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- Channels Status -->
        <Card>
          <CardHeader>
            <CardTitle class="flex items-center gap-2">
              <Network class="w-5 h-5" />
              Channels
            </CardTitle>
            <CardDescription>Messaging channels configured</CardDescription>
          </CardHeader>
          <CardContent>
            <div class="space-y-2">
              <div
                v-for="channel in channelList"
                :key="channel.name"
                class="flex items-center justify-between p-3 rounded-lg border"
              >
                <div class="flex items-center gap-2">
                  <CheckCircle2 v-if="channel.configured" class="w-5 h-5 text-green-500" />
                  <AlertCircle v-else class="w-5 h-5 text-muted-foreground" />
                  <span class="font-medium">{{ channel.name }}</span>
                </div>
                <Badge :variant="channel.configured ? 'success' : 'secondary'">
                  {{ channel.configured ? 'Configured' : 'Not configured' }}
                </Badge>
              </div>
            </div>
          </CardContent>
        </Card>

        <!-- Security & Autonomy -->
        <Card>
          <CardHeader>
            <CardTitle class="flex items-center gap-2">
              <Shield class="w-5 h-5" />
              Security & Autonomy
            </CardTitle>
            <CardDescription>Current security settings</CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <div class="flex items-center justify-between">
              <span class="text-muted-foreground">Autonomy Level</span>
              <Badge :variant="status.autonomy_level === 'full' ? 'success' : 'secondary'">
                {{ status.autonomy_level }}
              </Badge>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-muted-foreground">Workspace Only</span>
              <Badge :variant="status.workspace_only ? 'success' : 'warning'">
                {{ status.workspace_only ? 'Enabled' : 'Disabled' }}
              </Badge>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-muted-foreground">Secrets Encrypted</span>
              <Badge :variant="status.secrets_encrypted ? 'success' : 'warning'">
                {{ status.secrets_encrypted ? 'Yes' : 'No' }}
              </Badge>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-muted-foreground">Temperature</span>
              <span class="font-mono">{{ status.temperature }}</span>
            </div>
          </CardContent>
        </Card>
      </div>

      <!-- Quick Actions -->
      <Card>
        <CardHeader>
          <CardTitle>Quick Actions</CardTitle>
          <CardDescription>Common tasks and operations</CardDescription>
        </CardHeader>
        <CardContent>
          <div class="flex flex-wrap gap-2">
            <Button variant="outline" @click="navigate('/chat')">
              <MessageSquare class="w-4 h-4 mr-2" />
              Start Chat
            </Button>
            <Button variant="outline" @click="navigate('/channels')">
              <Network class="w-4 h-4 mr-2" />
              Configure Channels
            </Button>
            <Button variant="outline" @click="navigate('/config')">
              <Settings class="w-4 h-4 mr-2" />
              Edit Config
            </Button>
            <Button variant="outline" @click="runDoctor">
              <Activity class="w-4 h-4 mr-2" />
              Run Diagnostics
            </Button>
          </div>
        </CardContent>
      </Card>

      <!-- Workspace Info -->
      <Card>
        <CardHeader>
          <CardTitle>Paths</CardTitle>
          <CardDescription>Configuration locations</CardDescription>
        </CardHeader>
        <CardContent class="space-y-2">
          <div class="flex justify-between">
            <span class="text-muted-foreground">Workspace:</span>
            <code class="text-sm">{{ status.workspace }}</code>
          </div>
          <div class="flex justify-between">
            <span class="text-muted-foreground">Config:</span>
            <code class="text-sm">{{ status.config }}</code>
          </div>
        </CardContent>
      </Card>
    </template>
  </div>
</template>
