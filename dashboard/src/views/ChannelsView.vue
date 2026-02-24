<script setup lang="ts">
import { ref, onMounted } from 'vue'
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
  XCircle
} from 'lucide-vue-next'

interface Channel {
  id: string
  name: string
  type: 'telegram' | 'discord' | 'slack' | 'whatsapp' | 'matrix' | 'imessage' | 'email' | 'webhook'
  configured: boolean
  active: boolean
  allowlist_count: number
  last_activity?: string
}

const channels = ref<Channel[]>([])
const loading = ref(true)
const searchQuery = ref('')
const activeTab = ref('all')
const channelTypeFilter = ref('all')

const channelIcons: Record<string, any> = {
  telegram: MessageSquare,
  discord: Hash,
  slack: Hash,
  whatsapp: Phone,
  matrix: MessageSquare,
  imessage: MessageSquare,
  email: Mail,
  webhook: Send,
}

const sampleChannels: Channel[] = [
  {
    id: '1',
    name: 'Telegram Bot',
    type: 'telegram',
    configured: false,
    active: false,
    allowlist_count: 0,
  },
  {
    id: '2',
    name: 'Discord Server',
    type: 'discord',
    configured: true,
    active: true,
    allowlist_count: 2,
    last_activity: '2 minutes ago',
  },
  {
    id: '3',
    name: 'Slack Workspace',
    type: 'slack',
    configured: true,
    active: false,
    allowlist_count: 1,
  },
  {
    id: '4',
    name: 'WhatsApp Business',
    type: 'whatsapp',
    configured: false,
    active: false,
    allowlist_count: 0,
  },
  {
    id: '5',
    name: 'Matrix Room',
    type: 'matrix',
    configured: true,
    active: true,
    allowlist_count: 3,
    last_activity: '5 minutes ago',
  },
  {
    id: '6',
    name: 'Webhook Endpoint',
    type: 'webhook',
    configured: true,
    active: true,
    allowlist_count: 0,
    last_activity: '1 hour ago',
  },
]

async function loadChannels() {
  loading.value = true
  try {
    setTimeout(() => {
      channels.value = sampleChannels
      loading.value = false
    }, 500)
  } catch (e) {
    console.error(e)
    loading.value = false
  }
}

function filteredChannels() {
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

  if (channelTypeFilter.value !== 'all') {
    result = result.filter(c => c.type === channelTypeFilter.value)
  }

  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(c => c.name.toLowerCase().includes(query))
  }

  return result
}

function toggleChannel(channel: Channel) {
  channel.active = !channel.active
}

function configureChannel(channel: Channel) {
  // Would open config modal
  alert(`Configure ${channel.name}`)
}

onMounted(() => {
  loadChannels()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-bold">Channels</h1>
        <p class="text-muted-foreground">Manage messaging integrations</p>
      </div>
      <Button>
        <Plus class="w-4 h-4 mr-2" />
        Add Channel
      </Button>
    </div>

    <!-- Stats -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ channels.filter(c => c.configured).length }}</div>
          <p class="text-sm text-muted-foreground">Configured</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ channels.filter(c => c.active).length }}</div>
          <p class="text-sm text-muted-foreground">Active</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ channels.length }}</div>
          <p class="text-sm text-muted-foreground">Total Channels</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">
            {{ channels.reduce((acc, c) => acc + c.allowlist_count, 0) }}
          </div>
          <p class="text-sm text-muted-foreground">Allowed Senders</p>
        </CardContent>
      </Card>
    </div>

    <!-- Search and Filter -->
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
        </TabsList>
      </Tabs>
    </div>

    <!-- Channels Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <Card v-for="channel in filteredChannels()" :key="channel.id" class="hover:shadow-md transition-shadow">
        <CardHeader>
          <div class="flex items-start justify-between">
            <div class="flex items-center gap-3">
              <div class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                <component :is="channelIcons[channel.type]" class="w-5 h-5 text-primary" />
              </div>
              <div>
                <CardTitle class="text-lg">{{ channel.name }}</CardTitle>
                <CardDescription class="text-xs capitalize">{{ channel.type }}</CardDescription>
              </div>
            </div>
            <div class="flex gap-1">
              <CheckCircle2 v-if="channel.configured" class="w-5 h-5 text-green-500" />
              <XCircle v-else class="w-5 h-5 text-muted-foreground" />
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div class="flex items-center gap-4 text-sm text-muted-foreground mb-4">
            <span class="flex items-center gap-1">
              <Power class="w-4 h-4" />
              {{ channel.active ? 'Running' : 'Stopped' }}
            </span>
            <span v-if="channel.allowlist_count > 0">
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
              :disabled="!channel.configured"
            >
              <component :is="channel.active ? PowerOff : Power" class="w-4 h-4 mr-1" />
              {{ channel.active ? 'Stop' : 'Start' }}
            </Button>
            <Button variant="outline" size="sm" @click="configureChannel(channel)">
              <Settings class="w-4 h-4" />
            </Button>
            <Button variant="outline" size="sm">
              <RefreshCw class="w-4 h-4" />
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>

    <!-- Empty State -->
    <Card v-if="filteredChannels().length === 0 && !loading" class="p-8">
      <div class="text-center">
        <Network class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
        <h3 class="text-lg font-semibold mb-2">No channels found</h3>
        <p class="text-muted-foreground mb-4">
          {{ searchQuery ? 'Try a different search term' : 'Add a channel to start receiving messages' }}
        </p>
        <Button>
          <Plus class="w-4 h-4 mr-2" />
          Add Channel
        </Button>
      </div>
    </Card>
  </div>
</template>
