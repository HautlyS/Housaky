<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
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
  Network, Plus, Search, MessageSquare, Hash, Mail, Phone,
  Send, Settings, RefreshCw, CheckCircle2, XCircle, Loader2,
  Play, Square, Eye, EyeOff, Users, Zap, Clock, X, Terminal,
  Shield, AlertCircle, ChevronRight,
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

type ChannelType = 'telegram' | 'discord' | 'slack' | 'whatsapp' | 'matrix' | 'imessage' | 'email' | 'webhook' | 'cli'

interface Channel {
  id: string
  name: string
  type: ChannelType
  configured: boolean
  active: boolean
  allowlist_count: number
  last_activity?: string
  token?: string
  webhook_url?: string
  error?: string
}

interface ChannelFormFields {
  token: string
  webhook_url: string
  channel_id: string
  guild_id: string
  email: string
  smtp_host: string
  smtp_port: string
  allowlist: string
}

const channels = ref<Channel[]>([])
const loading = ref(true)
const pollingActive = ref(false)
const searchQuery = ref('')
const activeTab = ref('all')
const housakyInstalled = ref(true)
const selectedChannel = ref<Channel | null>(null)
const showConfigModal = ref(false)
const modalSaving = ref(false)
const modalError = ref('')
const toggleLoading = ref<Record<string, boolean>>({})
const showTokenFields = ref<Record<string, boolean>>({})

const formFields = ref<ChannelFormFields>({
  token: '',
  webhook_url: '',
  channel_id: '',
  guild_id: '',
  email: '',
  smtp_host: '',
  smtp_port: '587',
  allowlist: '',
})

const channelIcons: Record<string, any> = {
  cli:       Terminal,
  telegram:  Send,
  discord:   Hash,
  slack:     Hash,
  whatsapp:  Phone,
  matrix:    MessageSquare,
  imessage:  MessageSquare,
  email:     Mail,
  webhook:   Network,
}

const channelColors: Record<string, string> = {
  cli:       'from-gray-500 to-gray-600',
  telegram:  'from-blue-500 to-blue-600',
  discord:   'from-indigo-500 to-indigo-600',
  slack:     'from-purple-500 to-purple-600',
  whatsapp:  'from-green-500 to-green-600',
  matrix:    'from-orange-500 to-orange-600',
  imessage:  'from-green-600 to-emerald-600',
  email:     'from-red-500 to-red-600',
  webhook:   'from-cyan-500 to-cyan-600',
}

const channelDescriptions: Record<string, string> = {
  telegram:  'Receive messages from Telegram users',
  discord:   'Bot integration for Discord servers',
  slack:     'Slack workspace bot integration',
  whatsapp:  'WhatsApp Business API messaging',
  matrix:    'Matrix decentralized chat protocol',
  imessage:  'Apple iMessage relay (macOS only)',
  email:     'SMTP/IMAP email integration',
  webhook:   'Generic HTTP webhook endpoint',
  cli:       'Command-line interface channel',
}

let pollInterval: number | null = null

async function checkHousaky() {
  if (!isTauri) { housakyInstalled.value = false; return }
  try {
    housakyInstalled.value = await invoke<boolean>('check_housaky_installed')
  } catch { housakyInstalled.value = false }
}

async function loadChannels(silent = false) {
  if (!silent) loading.value = true
  try {
    if (isTauri) {
      const result = await invoke<Channel[]>('get_channels')
      channels.value = result
    } else {
      // Demo data for web mode
      channels.value = [
        { id: 'telegram', name: 'Telegram', type: 'telegram', configured: true,  active: true,  allowlist_count: 3, last_activity: '2m ago' },
        { id: 'discord',  name: 'Discord',  type: 'discord',  configured: false, active: false, allowlist_count: 0 },
        { id: 'slack',    name: 'Slack',    type: 'slack',    configured: false, active: false, allowlist_count: 0 },
        { id: 'email',    name: 'Email',    type: 'email',    configured: false, active: false, allowlist_count: 0 },
        { id: 'webhook',  name: 'Webhook',  type: 'webhook',  configured: true,  active: true,  allowlist_count: 0, last_activity: '15m ago' },
        { id: 'cli',      name: 'CLI',      type: 'cli',      configured: true,  active: false, allowlist_count: 0 },
      ]
    }
  } catch (e) {
    console.error(e)
  } finally {
    if (!silent) loading.value = false
  }
}

function startPolling() {
  pollingActive.value = true
  pollInterval = window.setInterval(() => loadChannels(true), 10_000)
}

const filteredChannels = computed(() => {
  let result = [...channels.value]
  if (activeTab.value === 'active')       result = result.filter(c => c.active)
  else if (activeTab.value === 'configured')   result = result.filter(c => c.configured)
  else if (activeTab.value === 'unconfigured') result = result.filter(c => !c.configured)
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    result = result.filter(c => c.name.toLowerCase().includes(q) || c.type.toLowerCase().includes(q))
  }
  return result
})

const stats = computed(() => ({
  configured: channels.value.filter(c => c.configured).length,
  active:     channels.value.filter(c => c.active).length,
  total:      channels.value.length,
  allowedSenders: channels.value.reduce((acc, c) => acc + c.allowlist_count, 0),
}))

async function toggleChannel(channel: Channel) {
  if (!isTauri || !housakyInstalled.value) return
  toggleLoading.value[channel.id] = true
  try {
    if (channel.active) {
      await invoke('stop_channel', { channelType: channel.type })
      channel.active = false
    } else {
      await invoke('start_channel', { channelType: channel.type })
      channel.active = true
    }
  } catch (e) {
    channel.error = String(e)
    setTimeout(() => { channel.error = undefined }, 5000)
  } finally {
    toggleLoading.value[channel.id] = false
  }
}

function openConfigModal(channel: Channel) {
  selectedChannel.value = channel
  modalError.value = ''
  formFields.value = {
    token: channel.token ?? '',
    webhook_url: channel.webhook_url ?? '',
    channel_id: '',
    guild_id: '',
    email: '',
    smtp_host: '',
    smtp_port: '587',
    allowlist: '',
  }
  showConfigModal.value = true
}

function closeConfigModal() {
  showConfigModal.value = false
  selectedChannel.value = null
  modalError.value = ''
}

async function saveChannelConfig() {
  if (!selectedChannel.value || !isTauri) return
  modalSaving.value = true
  modalError.value = ''

  // Validate required fields per channel type
  const type = selectedChannel.value.type
  if (['telegram', 'discord', 'slack'].includes(type) && !formFields.value.token.trim()) {
    modalError.value = 'Bot token is required'
    modalSaving.value = false
    return
  }
  if (type === 'webhook' && !formFields.value.webhook_url.trim()) {
    modalError.value = 'Webhook URL is required'
    modalSaving.value = false
    return
  }
  if (type === 'email' && !formFields.value.email.trim()) {
    modalError.value = 'Email address is required'
    modalSaving.value = false
    return
  }

  const config: Record<string, string> = {}
  if (formFields.value.token)       config.token       = formFields.value.token.trim()
  if (formFields.value.webhook_url) config.webhook_url = formFields.value.webhook_url.trim()
  if (formFields.value.channel_id)  config.channel_id  = formFields.value.channel_id.trim()
  if (formFields.value.guild_id)    config.guild_id    = formFields.value.guild_id.trim()
  if (formFields.value.email)       config.email       = formFields.value.email.trim()
  if (formFields.value.smtp_host)   config.smtp_host   = formFields.value.smtp_host.trim()
  if (formFields.value.smtp_port)   config.smtp_port   = formFields.value.smtp_port.trim()
  if (formFields.value.allowlist)   config.allowlist   = formFields.value.allowlist.trim()

  try {
    await invoke('configure_channel', { channelType: selectedChannel.value.type, config })
    selectedChannel.value.configured = true
    closeConfigModal()
    await loadChannels(true)
  } catch (e) {
    modalError.value = String(e)
  } finally {
    modalSaving.value = false
  }
}

function toggleTokenVisibility(id: string) {
  showTokenFields.value[id] = !showTokenFields.value[id]
}

onMounted(async () => {
  await checkHousaky()
  await loadChannels()
  startPolling()
})

onUnmounted(() => {
  if (pollInterval) clearInterval(pollInterval)
})
</script>

<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h1 class="text-2xl font-bold tracking-tight flex items-center gap-2">
          <Network class="w-6 h-6 text-primary" />
          Channels
        </h1>
        <p class="text-sm text-muted-foreground flex items-center gap-1.5 mt-0.5">
          Manage messaging integrations
          <span class="flex items-center gap-1 text-green-600">
            <span class="w-1.5 h-1.5 rounded-full bg-green-500 animate-pulse inline-block" />
            Live polling
          </span>
        </p>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" @click="loadChannels(false)" :disabled="loading" class="gap-1.5">
          <RefreshCw :class="['w-3.5 h-3.5', loading && 'animate-spin']" />
          Refresh
        </Button>
      </div>
    </div>

    <!-- KPI Row -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <Card class="border-0 bg-gradient-to-br from-blue-500/10 to-blue-500/5">
        <CardContent class="pt-4 pb-3">
          <div class="flex items-center justify-between mb-1">
            <span class="text-xs font-medium text-muted-foreground uppercase tracking-wide">Configured</span>
            <CheckCircle2 class="w-3.5 h-3.5 text-blue-500" />
          </div>
          <div class="text-2xl font-bold">{{ stats.configured }}</div>
          <p class="text-xs text-muted-foreground">of {{ stats.total }} total</p>
        </CardContent>
      </Card>
      <Card class="border-0 bg-gradient-to-br from-green-500/10 to-green-500/5">
        <CardContent class="pt-4 pb-3">
          <div class="flex items-center justify-between mb-1">
            <span class="text-xs font-medium text-muted-foreground uppercase tracking-wide">Active</span>
            <Zap class="w-3.5 h-3.5 text-green-500" />
          </div>
          <div class="text-2xl font-bold text-green-600">{{ stats.active }}</div>
          <p class="text-xs text-muted-foreground">channels running</p>
        </CardContent>
      </Card>
      <Card class="border-0 bg-gradient-to-br from-purple-500/10 to-purple-500/5">
        <CardContent class="pt-4 pb-3">
          <div class="flex items-center justify-between mb-1">
            <span class="text-xs font-medium text-muted-foreground uppercase tracking-wide">Allowed</span>
            <Users class="w-3.5 h-3.5 text-purple-500" />
          </div>
          <div class="text-2xl font-bold">{{ stats.allowedSenders }}</div>
          <p class="text-xs text-muted-foreground">senders total</p>
        </CardContent>
      </Card>
      <Card class="border-0 bg-gradient-to-br from-amber-500/10 to-amber-500/5">
        <CardContent class="pt-4 pb-3">
          <div class="flex items-center justify-between mb-1">
            <span class="text-xs font-medium text-muted-foreground uppercase tracking-wide">Polling</span>
            <Clock class="w-3.5 h-3.5 text-amber-500" />
          </div>
          <div class="text-2xl font-bold">10s</div>
          <p class="text-xs text-muted-foreground">refresh interval</p>
        </CardContent>
      </Card>
    </div>

    <!-- Search + Filter -->
    <div class="flex gap-3 flex-wrap items-center">
      <div class="relative flex-1 min-w-48 max-w-sm">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground" />
        <Input v-model="searchQuery" placeholder="Search channels…" class="pl-9 h-9 text-sm" />
      </div>
      <div class="flex items-center gap-1 bg-muted rounded-lg p-0.5">
        <button v-for="tab in ['all', 'active', 'configured', 'unconfigured']" :key="tab"
          @click="activeTab = tab"
          :class="[
            'px-3 py-1.5 rounded-md text-xs font-medium transition-all capitalize',
            activeTab === tab ? 'bg-background shadow-sm' : 'text-muted-foreground hover:text-foreground'
          ]"
        >
          {{ tab }}
        </button>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-16">
      <div class="flex flex-col items-center gap-3">
        <Loader2 class="w-8 h-8 animate-spin text-muted-foreground" />
        <p class="text-sm text-muted-foreground">Loading channels…</p>
      </div>
    </div>

    <!-- Empty -->
    <div v-else-if="filteredChannels.length === 0" class="text-center py-16">
      <Network class="w-12 h-12 mx-auto text-muted-foreground mb-3" />
      <h3 class="font-semibold mb-1">No channels found</h3>
      <p class="text-sm text-muted-foreground">{{ searchQuery ? 'Try a different search term' : 'Configure channels to receive messages' }}</p>
    </div>

    <!-- Channel Grid -->
    <div v-else class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      <Card v-for="channel in filteredChannels" :key="channel.id"
        class="hover:shadow-lg transition-all duration-200 overflow-hidden"
        :class="channel.active ? 'ring-1 ring-green-400/30' : ''"
      >
        <CardHeader class="pb-2 pt-4">
          <div class="flex items-start justify-between">
            <div class="flex items-center gap-3">
              <div :class="['w-10 h-10 rounded-xl bg-gradient-to-br flex items-center justify-center text-white shadow-sm flex-shrink-0', channelColors[channel.type] ?? 'from-gray-500 to-gray-600']">
                <component :is="channelIcons[channel.type] ?? Network" class="w-5 h-5" />
              </div>
              <div>
                <CardTitle class="text-sm font-semibold">{{ channel.name }}</CardTitle>
                <p class="text-[10px] text-muted-foreground capitalize">{{ channel.type }}</p>
              </div>
            </div>
            <div class="flex items-center gap-1.5">
              <div :class="['w-2 h-2 rounded-full flex-shrink-0', channel.active ? 'bg-green-500 animate-pulse' : 'bg-gray-300 dark:bg-gray-600']" />
              <span :class="['text-[10px] font-medium', channel.active ? 'text-green-600' : 'text-muted-foreground']">
                {{ channel.active ? 'Live' : 'Off' }}
              </span>
            </div>
          </div>
        </CardHeader>

        <CardContent class="pt-0">
          <p class="text-xs text-muted-foreground mb-3 leading-relaxed">
            {{ channelDescriptions[channel.type] ?? '' }}
          </p>

          <!-- Status row -->
          <div class="flex items-center gap-2 mb-3 flex-wrap">
            <Badge :class="channel.configured
              ? 'bg-green-500/10 text-green-600 border-green-200 text-[10px]'
              : 'bg-yellow-500/10 text-yellow-600 border-yellow-200 text-[10px]'"
            >
              <CheckCircle2 v-if="channel.configured" class="w-2.5 h-2.5 mr-1" />
              <XCircle v-else class="w-2.5 h-2.5 mr-1" />
              {{ channel.configured ? 'Configured' : 'Needs setup' }}
            </Badge>
            <Badge v-if="channel.allowlist_count > 0" class="bg-purple-500/10 text-purple-600 border-purple-200 text-[10px]">
              <Users class="w-2.5 h-2.5 mr-1" />
              {{ channel.allowlist_count }} allowed
            </Badge>
            <span v-if="channel.last_activity" class="text-[10px] text-muted-foreground flex items-center gap-1">
              <Clock class="w-2.5 h-2.5" />
              {{ channel.last_activity }}
            </span>
          </div>

          <!-- Error -->
          <div v-if="channel.error" class="flex items-center gap-1.5 p-2 rounded-md bg-red-500/10 text-red-600 text-xs mb-3">
            <AlertCircle class="w-3 h-3 flex-shrink-0" />
            <span class="truncate">{{ channel.error }}</span>
          </div>

          <!-- Actions -->
          <div class="flex gap-2">
            <Button
              size="sm"
              :variant="channel.active ? 'destructive' : 'default'"
              class="flex-1 h-8 gap-1.5 text-xs"
              @click="toggleChannel(channel)"
              :disabled="!channel.configured || !housakyInstalled || !!toggleLoading[channel.id]"
            >
              <Loader2 v-if="toggleLoading[channel.id]" class="w-3 h-3 animate-spin" />
              <component v-else :is="channel.active ? Square : Play" class="w-3 h-3" />
              {{ channel.active ? 'Stop' : 'Start' }}
            </Button>
            <Button
              variant="outline"
              size="sm"
              class="h-8 w-8 p-0"
              @click="openConfigModal(channel)"
              title="Configure"
            >
              <Settings class="w-3.5 h-3.5" />
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>

    <!-- Config Modal -->
    <Teleport to="body">
      <div v-if="showConfigModal && selectedChannel"
        class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4"
        @click.self="closeConfigModal"
      >
        <Card class="w-full max-w-lg shadow-2xl">
          <CardHeader class="pb-3">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div :class="['w-9 h-9 rounded-xl bg-gradient-to-br flex items-center justify-center text-white flex-shrink-0', channelColors[selectedChannel.type] ?? 'from-gray-500 to-gray-600']">
                  <component :is="channelIcons[selectedChannel.type] ?? Network" class="w-4.5 h-4.5" />
                </div>
                <div>
                  <CardTitle class="text-base">{{ selectedChannel.name }}</CardTitle>
                  <CardDescription class="text-xs">Configure credentials & allowlist</CardDescription>
                </div>
              </div>
              <Button variant="ghost" size="sm" class="h-8 w-8 p-0" @click="closeConfigModal">
                <X class="w-4 h-4" />
              </Button>
            </div>
          </CardHeader>

          <CardContent>
            <div v-if="modalError" class="flex items-center gap-2 p-3 rounded-lg bg-destructive/10 text-destructive text-sm mb-4">
              <AlertCircle class="w-4 h-4 flex-shrink-0" />
              {{ modalError }}
            </div>

            <form @submit.prevent="saveChannelConfig" class="space-y-4">
              <!-- Telegram -->
              <template v-if="selectedChannel.type === 'telegram'">
                <div class="space-y-1.5">
                  <label class="text-sm font-medium">Bot Token <span class="text-destructive">*</span></label>
                  <div class="relative">
                    <Input v-model="formFields.token"
                      :type="showTokenFields['token'] ? 'text' : 'password'"
                      placeholder="123456789:AABBccddEEFFggHH..."
                      class="pr-9"
                    />
                    <button type="button" @click="toggleTokenVisibility('token')" class="absolute right-2.5 top-1/2 -translate-y-1/2">
                      <component :is="showTokenFields['token'] ? EyeOff : Eye" class="w-4 h-4 text-muted-foreground" />
                    </button>
                  </div>
                  <p class="text-[11px] text-muted-foreground">Get from <span class="font-mono">@BotFather</span> on Telegram</p>
                </div>
              </template>

              <!-- Discord -->
              <template v-else-if="selectedChannel.type === 'discord'">
                <div class="space-y-1.5">
                  <label class="text-sm font-medium">Bot Token <span class="text-destructive">*</span></label>
                  <div class="relative">
                    <Input v-model="formFields.token"
                      :type="showTokenFields['token'] ? 'text' : 'password'"
                      placeholder="MTxxxxxx.xxxxxx.xxxxxxx"
                      class="pr-9"
                    />
                    <button type="button" @click="toggleTokenVisibility('token')" class="absolute right-2.5 top-1/2 -translate-y-1/2">
                      <component :is="showTokenFields['token'] ? EyeOff : Eye" class="w-4 h-4 text-muted-foreground" />
                    </button>
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-3">
                  <div class="space-y-1.5">
                    <label class="text-sm font-medium">Guild ID</label>
                    <Input v-model="formFields.guild_id" placeholder="Server ID" />
                  </div>
                  <div class="space-y-1.5">
                    <label class="text-sm font-medium">Channel ID</label>
                    <Input v-model="formFields.channel_id" placeholder="Channel ID" />
                  </div>
                </div>
              </template>

              <!-- Slack -->
              <template v-else-if="selectedChannel.type === 'slack'">
                <div class="space-y-1.5">
                  <label class="text-sm font-medium">Bot Token <span class="text-destructive">*</span></label>
                  <div class="relative">
                    <Input v-model="formFields.token"
                      :type="showTokenFields['token'] ? 'text' : 'password'"
                      placeholder="xoxb-xxxx-xxxx-xxxx"
                      class="pr-9"
                    />
                    <button type="button" @click="toggleTokenVisibility('token')" class="absolute right-2.5 top-1/2 -translate-y-1/2">
                      <component :is="showTokenFields['token'] ? EyeOff : Eye" class="w-4 h-4 text-muted-foreground" />
                    </button>
                  </div>
                </div>
              </template>

              <!-- Webhook -->
              <template v-else-if="selectedChannel.type === 'webhook'">
                <div class="space-y-1.5">
                  <label class="text-sm font-medium">Webhook URL <span class="text-destructive">*</span></label>
                  <Input v-model="formFields.webhook_url" placeholder="https://your-server.com/webhook" />
                  <p class="text-[11px] text-muted-foreground">Housaky will POST events to this URL</p>
                </div>
                <div class="space-y-1.5">
                  <label class="text-sm font-medium">Secret Token</label>
                  <div class="relative">
                    <Input v-model="formFields.token"
                      :type="showTokenFields['token'] ? 'text' : 'password'"
                      placeholder="Optional HMAC signing secret"
                      class="pr-9"
                    />
                    <button type="button" @click="toggleTokenVisibility('token')" class="absolute right-2.5 top-1/2 -translate-y-1/2">
                      <component :is="showTokenFields['token'] ? EyeOff : Eye" class="w-4 h-4 text-muted-foreground" />
                    </button>
                  </div>
                </div>
              </template>

              <!-- Email -->
              <template v-else-if="selectedChannel.type === 'email'">
                <div class="space-y-1.5">
                  <label class="text-sm font-medium">Email Address <span class="text-destructive">*</span></label>
                  <Input v-model="formFields.email" type="email" placeholder="agent@yourdomain.com" />
                </div>
                <div class="grid grid-cols-2 gap-3">
                  <div class="space-y-1.5">
                    <label class="text-sm font-medium">SMTP Host</label>
                    <Input v-model="formFields.smtp_host" placeholder="smtp.gmail.com" />
                  </div>
                  <div class="space-y-1.5">
                    <label class="text-sm font-medium">SMTP Port</label>
                    <Input v-model="formFields.smtp_port" type="number" placeholder="587" />
                  </div>
                </div>
                <div class="space-y-1.5">
                  <label class="text-sm font-medium">App Password</label>
                  <div class="relative">
                    <Input v-model="formFields.token"
                      :type="showTokenFields['token'] ? 'text' : 'password'"
                      placeholder="App-specific password"
                      class="pr-9"
                    />
                    <button type="button" @click="toggleTokenVisibility('token')" class="absolute right-2.5 top-1/2 -translate-y-1/2">
                      <component :is="showTokenFields['token'] ? EyeOff : Eye" class="w-4 h-4 text-muted-foreground" />
                    </button>
                  </div>
                </div>
              </template>

              <!-- Generic fallback -->
              <template v-else>
                <div class="space-y-1.5">
                  <label class="text-sm font-medium">Token / Credential</label>
                  <div class="relative">
                    <Input v-model="formFields.token"
                      :type="showTokenFields['token'] ? 'text' : 'password'"
                      placeholder="Enter credential…"
                      class="pr-9"
                    />
                    <button type="button" @click="toggleTokenVisibility('token')" class="absolute right-2.5 top-1/2 -translate-y-1/2">
                      <component :is="showTokenFields['token'] ? EyeOff : Eye" class="w-4 h-4 text-muted-foreground" />
                    </button>
                  </div>
                </div>
              </template>

              <!-- Allowlist (common to all) -->
              <div class="space-y-1.5">
                <label class="text-sm font-medium flex items-center gap-1.5">
                  <Shield class="w-3.5 h-3.5 text-muted-foreground" />
                  Allowlist
                </label>
                <textarea
                  v-model="formFields.allowlist"
                  rows="3"
                  class="w-full rounded-md border bg-background px-3 py-2 text-sm resize-none focus:outline-none focus:ring-1 focus:ring-ring"
                  placeholder="One user ID / username per line (leave empty to allow all)"
                />
                <p class="text-[11px] text-muted-foreground">Only these users can interact with the agent via this channel</p>
              </div>

              <div class="flex gap-2 justify-end pt-2 border-t">
                <Button type="button" variant="outline" size="sm" @click="closeConfigModal">Cancel</Button>
                <Button type="submit" size="sm" :disabled="modalSaving" class="gap-1.5">
                  <Loader2 v-if="modalSaving" class="w-3.5 h-3.5 animate-spin" />
                  <CheckCircle2 v-else class="w-3.5 h-3.5" />
                  {{ modalSaving ? 'Saving…' : 'Save Configuration' }}
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>
      </div>
    </Teleport>
  </div>
</template>
