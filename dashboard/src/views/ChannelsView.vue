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
  Network, 
  Plus, 
  Search,
  MessageSquare,
  Hash,
  Mail,
  Phone,
  Send,
  Settings,
  Power,
  PowerOff,
  RefreshCw,
  CheckCircle2,
  XCircle,
  Loader2,
  Play,
  Square,
  ExternalLink
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface Channel {
  id: string
  name: string
  type: 'telegram' | 'discord' | 'slack' | 'whatsapp' | 'matrix' | 'imessage' | 'email' | 'webhook' | 'cli'
  configured: boolean
  active: boolean
  allowlist_count: number
  last_activity?: string
}

const channels = ref<Channel[]>([])
const loading = ref(true)
const searchQuery = ref('')
const activeTab = ref('all')
const housakyInstalled = ref(true)
const selectedChannel = ref<Channel | null>(null)
const showConfigModal = ref(false)

const channelIcons: Record<string, any> = {
  cli: Terminal,
  telegram: Send,
  discord: Hash,
  slack: Hash,
  whatsapp: Phone,
  matrix: MessageSquare,
  imessage: MessageSquare,
  email: Mail,
  webhook: Network,
}

const channelColors: Record<string, string> = {
  cli: 'bg-gray-500',
  telegram: 'bg-blue-500',
  discord: 'bg-indigo-500',
  slack: 'bg-purple-500',
  whatsapp: 'bg-green-500',
  matrix: 'bg-orange-500',
  imessage: 'bg-green-600',
  email: 'bg-red-500',
  webhook: 'bg-cyan-500',
}

const Terminal = { template: '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"></polyline><line x1="12" y1="19" x2="20" y2="19"></line></svg>' }

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

async function loadChannels() {
  loading.value = true
  try {
    if (isTauri) {
      const result = await invoke<Channel[]>('get_channels')
      channels.value = result
    }
  } catch (e) {
    console.error(e)
  } finally {
    loading.value = false
  }
}

const filteredChannels = computed(() => {
  let result = channels.value

  if (activeTab.value === 'active') {
    result = result.filter(c => c.active)
  } else if (activeTab.value === 'inactive') {
    result = result.filter(c => !c.active)
  } else if (activeTab.value === 'configured') {
    result = result.filter(c => c.configured)
  } else if (activeTab.value === 'unconfigured') {
    result = result.filter(c => !c.configured)
  }

  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(c => c.name.toLowerCase().includes(query))
  }

  return result
})

const stats = computed(() => ({
  configured: channels.value.filter(c => c.configured).length,
  active: channels.value.filter(c => c.active).length,
  total: channels.value.length,
  allowedSenders: channels.value.reduce((acc, c) => acc + c.allowlist_count, 0)
}))

async function toggleChannel(channel: Channel) {
  if (!isTauri) {
    alert('Running in server mode - channel control not available')
    return
  }
  if (channel.active) {
    try {
      await invoke('stop_channel', { channelType: channel.type })
      channel.active = false
    } catch (e) {
      console.error(e)
    }
  } else {
    try {
      await invoke('start_channel', { channelType: channel.type })
      channel.active = true
    } catch (e) {
      console.error(e)
    }
  }
}

function configureChannel(channel: Channel) {
  selectedChannel.value = channel
  showConfigModal.value = true
}

function closeConfigModal() {
  showConfigModal.value = false
  selectedChannel.value = null
}

async function saveChannelConfig(config: Record<string, any>) {
  if (!selectedChannel.value) return
  
  if (!isTauri) {
    alert('Running in server mode - channel config not available')
    return
  }
  
  try {
    await invoke('configure_channel', { 
      channelType: selectedChannel.value.type, 
      config 
    })
    selectedChannel.value.configured = true
    closeConfigModal()
    await loadChannels()
  } catch (e) {
    console.error(e)
  }
}

onMounted(async () => {
  await checkHousaky()
  await loadChannels()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">Channels</h1>
        <p class="text-sm text-muted-foreground">Manage messaging integrations</p>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" @click="loadChannels" :disabled="loading">
          <RefreshCw :class="['w-4 h-4 mr-2', loading && 'animate-spin']" />
          Refresh
        </Button>
        <Button size="sm">
          <Plus class="w-4 h-4 mr-2" />
          Add Channel
        </Button>
      </div>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ stats.configured }}</div>
          <p class="text-sm text-muted-foreground">Configured</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold text-green-600">{{ stats.active }}</div>
          <p class="text-sm text-muted-foreground">Active</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ stats.total }}</div>
          <p class="text-sm text-muted-foreground">Total Channels</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ stats.allowedSenders }}</div>
          <p class="text-sm text-muted-foreground">Allowed Senders</p>
        </CardContent>
      </Card>
    </div>

    <div class="flex gap-4 flex-wrap">
      <div class="relative flex-1 max-w-md">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
        <Input v-model="searchQuery" placeholder="Search channels..." class="pl-10" />
      </div>
      <Tabs v-model="activeTab" defaultValue="all">
        <TabsList>
          <TabsTrigger value="all">All</TabsTrigger>
          <TabsTrigger value="active">Active</TabsTrigger>
          <TabsTrigger value="configured">Configured</TabsTrigger>
          <TabsTrigger value="unconfigured">Setup</TabsTrigger>
        </TabsList>
      </Tabs>
    </div>

    <div v-if="loading" class="flex items-center justify-center py-12">
      <Loader2 class="w-8 h-8 animate-spin text-muted-foreground" />
    </div>

    <div v-else-if="filteredChannels.length === 0" class="text-center py-12">
      <Network class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
      <h3 class="text-lg font-semibold mb-2">No channels found</h3>
      <p class="text-muted-foreground mb-4">
        {{ searchQuery ? 'Try a different search term' : 'Configure channels to receive messages' }}
      </p>
    </div>

    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <Card 
        v-for="channel in filteredChannels" 
        :key="channel.id" 
        class="hover:shadow-md transition-all"
      >
        <CardHeader class="pb-3">
          <div class="flex items-start justify-between">
            <div class="flex items-center gap-3">
              <div :class="[
                'w-10 h-10 rounded-lg flex items-center justify-center text-white',
                channelColors[channel.type] || 'bg-gray-500'
              ]">
                <component :is="channelIcons[channel.type]" class="w-5 h-5" />
              </div>
              <div>
                <CardTitle class="text-base">{{ channel.name }}</CardTitle>
                <CardDescription class="text-xs capitalize">{{ channel.type }}</CardDescription>
              </div>
            </div>
            <div class="flex items-center gap-1">
              <CheckCircle2 v-if="channel.configured" class="w-5 h-5 text-green-500" />
              <XCircle v-else class="w-5 h-5 text-muted-foreground" />
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div class="flex items-center gap-4 text-sm text-muted-foreground mb-4">
            <span class="flex items-center gap-1">
              <div :class="['w-2 h-2 rounded-full', channel.active ? 'bg-green-500' : 'bg-gray-300']"></div>
              {{ channel.active ? 'Running' : 'Stopped' }}
            </span>
            <span v-if="channel.allowlist_count > 0" class="flex items-center gap-1">
              {{ channel.allowlist_count }} allowed
            </span>
          </div>
          
          <p v-if="channel.last_activity" class="text-xs text-muted-foreground mb-4">
            Last activity: {{ channel.last_activity }}
          </p>

          <div class="flex gap-2">
            <Button 
              variant="outline" 
              size="sm" 
              class="flex-1"
              @click="toggleChannel(channel)"
              :disabled="!channel.configured || !housakyInstalled"
            >
              <component :is="channel.active ? Square : Play" class="w-3 h-3 mr-1" />
              {{ channel.active ? 'Stop' : 'Start' }}
            </Button>
            <Button 
              variant="outline" 
              size="sm" 
              @click="configureChannel(channel)"
            >
              <Settings class="w-3 h-3" />
            </Button>
            <Button variant="outline" size="sm">
              <ExternalLink class="w-3 h-3" />
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>

    <!-- Simple Config Modal -->
    <div v-if="showConfigModal && selectedChannel" class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <Card class="w-full max-w-md mx-4">
        <CardHeader>
          <CardTitle>Configure {{ selectedChannel.name }}</CardTitle>
          <CardDescription>Enter your {{ selectedChannel.type }} credentials</CardDescription>
        </CardHeader>
        <CardContent>
          <form @submit.prevent="saveChannelConfig({})" class="space-y-4">
            <div v-if="selectedChannel.type === 'telegram'">
              <label class="text-sm font-medium mb-2 block">Bot Token</label>
              <Input placeholder="Enter Telegram bot token" />
            </div>
            <div v-else-if="selectedChannel.type === 'discord'">
              <label class="text-sm font-medium mb-2 block">Bot Token</label>
              <Input placeholder="Enter Discord bot token" />
            </div>
            <div v-else-if="selectedChannel.type === 'slack'">
              <label class="text-sm font-medium mb-2 block">Bot Token</label>
              <Input placeholder="Enter Slack bot token" />
            </div>
            <div v-else>
              <p class="text-sm text-muted-foreground">
                Configuration for {{ selectedChannel.type }} coming soon.
              </p>
            </div>
            
            <div class="flex gap-2 justify-end pt-4">
              <Button type="button" variant="outline" @click="closeConfigModal">
                Cancel
              </Button>
              <Button type="submit">
                Save Configuration
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  </div>
</template>
