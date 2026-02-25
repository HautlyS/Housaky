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
  Package, 
  Search,
  CheckCircle2,
  Clock,
  XCircle,
  ExternalLink,
  MessageSquare,
  Cpu,
  Cloud,
  Wrench,
  Music,
  Home,
  Palette,
  Share2,
  Layers,
  Loader2
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface Integration {
  name: string
  description: string
  category: 'chat' | 'ai_model' | 'productivity' | 'music_audio' | 'smart_home' | 'tools_automation' | 'media_creative' | 'social' | 'platform'
  status: 'available' | 'active' | 'coming_soon'
}

const integrations = ref<Integration[]>([])
const loading = ref(true)
const searchQuery = ref('')
const activeTab = ref('all')
const selectedCategory = ref('all')

const categoryIcons: Record<string, any> = {
  chat: MessageSquare,
  ai_model: Cpu,
  productivity: Layers,
  music_audio: Music,
  smart_home: Home,
  tools_automation: Wrench,
  media_creative: Palette,
  social: Share2,
  platform: Cloud,
}

const categoryLabels: Record<string, string> = {
  chat: 'Chat Providers',
  ai_model: 'AI Models',
  productivity: 'Productivity',
  music_audio: 'Music & Audio',
  smart_home: 'Smart Home',
  tools_automation: 'Tools & Automation',
  media_creative: 'Media & Creative',
  social: 'Social',
  platform: 'Platforms',
}

const sampleIntegrations: Integration[] = [
  { name: 'Telegram', description: 'Bot API — long-polling', category: 'chat', status: 'available' },
  { name: 'Discord', description: 'Servers, channels & DMs', category: 'chat', status: 'available' },
  { name: 'Slack', description: 'Workspace apps via Web API', category: 'chat', status: 'available' },
  { name: 'WhatsApp', description: 'Meta Cloud API via webhook', category: 'chat', status: 'available' },
  { name: 'Signal', description: 'Privacy-focused via signal-cli', category: 'chat', status: 'coming_soon' },
  { name: 'iMessage', description: 'macOS AppleScript bridge', category: 'chat', status: 'available' },
  { name: 'Matrix', description: 'Matrix protocol (Element)', category: 'chat', status: 'available' },
  { name: 'Microsoft Teams', description: 'Enterprise chat support', category: 'chat', status: 'coming_soon' },
  { name: 'OpenRouter', description: '200+ models, 1 API key', category: 'ai_model', status: 'active' },
  { name: 'Anthropic', description: 'Claude 3.5/4 Sonnet & Opus', category: 'ai_model', status: 'available' },
  { name: 'OpenAI', description: 'GPT-4o, GPT-5, o1', category: 'ai_model', status: 'available' },
  { name: 'Google', description: 'Gemini 2.5 Pro/Flash', category: 'ai_model', status: 'available' },
  { name: 'DeepSeek', description: 'DeepSeek V3 & R1', category: 'ai_model', status: 'available' },
  { name: 'Ollama', description: 'Local models', category: 'ai_model', status: 'available' },
  { name: 'Groq', description: 'Fast inference', category: 'ai_model', status: 'available' },
  { name: 'Mistral', description: 'Mistral AI models', category: 'ai_model', status: 'available' },
  { name: 'Composio', description: '1000+ OAuth apps', category: 'tools_automation', status: 'available' },
  { name: 'Brave Search', description: 'Web search API', category: 'tools_automation', status: 'available' },
  { name: 'GitHub', description: 'Repo management', category: 'tools_automation', status: 'available' },
  { name: 'Notion', description: 'Workspace integration', category: 'productivity', status: 'coming_soon' },
  { name: 'Home Assistant', description: 'Smart home hub', category: 'smart_home', status: 'coming_soon' },
  { name: 'Philips Hue', description: 'Smart lighting', category: 'smart_home', status: 'coming_soon' },
]

async function loadIntegrations() {
  loading.value = true
  try {
    if (isTauri) {
      const result = await invoke<Integration[]>('get_integrations')
      integrations.value = result.length > 0 ? result : sampleIntegrations
    } else {
      integrations.value = sampleIntegrations
    }
  } catch (e) {
    console.error(e)
    integrations.value = sampleIntegrations
  } finally {
    loading.value = false
  }
}

const categories = computed(() => {
  const cats = new Set(integrations.value.map(i => i.category))
  return Array.from(cats)
})

const filteredIntegrations = computed(() => {
  let result = integrations.value

  if (activeTab.value === 'active') {
    result = result.filter(i => i.status === 'active')
  } else if (activeTab.value === 'available') {
    result = result.filter(i => i.status === 'available')
  } else if (activeTab.value === 'coming_soon') {
    result = result.filter(i => i.status === 'coming_soon')
  }

  if (selectedCategory.value !== 'all') {
    result = result.filter(i => i.category === selectedCategory.value)
  }

  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(i => 
      i.name.toLowerCase().includes(query) ||
      i.description.toLowerCase().includes(query)
    )
  }

  return result
})

const stats = computed(() => ({
  active: integrations.value.filter(i => i.status === 'active').length,
  available: integrations.value.filter(i => i.status === 'available').length,
  comingSoon: integrations.value.filter(i => i.status === 'coming_soon').length,
  categories: categories.value.length
}))

function getStatusIcon(status: string) {
  switch (status) {
    case 'active': return CheckCircle2
    case 'available': return CheckCircle2
    case 'coming_soon': return Clock
    default: return XCircle
  }
}

function getStatusColor(status: string) {
  switch (status) {
    case 'active': return 'success'
    case 'available': return 'secondary'
    case 'coming_soon': return 'warning'
    default: return 'secondary'
  }
}

function viewIntegration(integration: Integration) {
  alert(`${integration.name}\n\n${integration.description}\n\nStatus: ${integration.status}`)
}

onMounted(() => {
  loadIntegrations()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">Integrations</h1>
        <p class="text-sm text-muted-foreground">Browse 75+ integrations across 9 categories</p>
      </div>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold text-green-600">{{ stats.active }}</div>
          <p class="text-sm text-muted-foreground">Active</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ stats.available }}</div>
          <p class="text-sm text-muted-foreground">Available</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold text-yellow-600">{{ stats.comingSoon }}</div>
          <p class="text-sm text-muted-foreground">Coming Soon</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ stats.categories }}</div>
          <p class="text-sm text-muted-foreground">Categories</p>
        </CardContent>
      </Card>
    </div>

    <div class="flex gap-4 flex-wrap">
      <div class="relative flex-1 max-w-md">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
        <Input v-model="searchQuery" placeholder="Search integrations..." class="pl-10" />
      </div>
      <Tabs v-model="activeTab" defaultValue="all">
        <TabsList>
          <TabsTrigger value="all">All</TabsTrigger>
          <TabsTrigger value="active">Active</TabsTrigger>
          <TabsTrigger value="available">Available</TabsTrigger>
          <TabsTrigger value="coming_soon">Coming Soon</TabsTrigger>
        </TabsList>
      </Tabs>
      <select 
        v-model="selectedCategory"
        class="h-10 rounded-md border bg-background px-3 text-sm"
      >
        <option value="all">All Categories</option>
        <option v-for="cat in categories" :key="cat" :value="cat">
          {{ categoryLabels[cat] || cat }}
        </option>
      </select>
    </div>

    <div v-if="loading" class="flex items-center justify-center py-12">
      <Loader2 class="w-8 h-8 animate-spin text-muted-foreground" />
    </div>

    <div v-else-if="filteredIntegrations.length === 0" class="text-center py-12">
      <Package class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
      <h3 class="text-lg font-semibold mb-2">No integrations found</h3>
      <p class="text-muted-foreground">Try a different search term or filter</p>
    </div>

    <div v-else class="space-y-8">
      <div v-for="category in categories" :key="category">
        <div v-if="filteredIntegrations.filter(i => i.category === category).length > 0" class="space-y-4">
          <div class="flex items-center gap-2">
            <component :is="categoryIcons[category]" class="w-5 h-5 text-muted-foreground" />
            <h2 class="text-lg font-semibold">{{ categoryLabels[category] }}</h2>
            <Badge variant="outline">{{ filteredIntegrations.filter(i => i.category === category).length }}</Badge>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            <Card 
              v-for="integration in filteredIntegrations.filter(i => i.category === category)" 
              :key="integration.name"
              class="hover:shadow-md transition-all cursor-pointer hover:-translate-y-0.5"
              @click="viewIntegration(integration)"
            >
              <CardHeader class="pb-2">
                <div class="flex items-start justify-between">
                  <CardTitle class="text-base">{{ integration.name }}</CardTitle>
                  <component 
                    :is="getStatusIcon(integration.status)" 
                    :class="[
                      'w-4 h-4',
                      integration.status === 'active' ? 'text-green-500' :
                      integration.status === 'coming_soon' ? 'text-yellow-500' : 'text-muted-foreground'
                    ]"
                  />
                </div>
              </CardHeader>
              <CardContent>
                <p class="text-sm text-muted-foreground mb-3">{{ integration.description }}</p>
                <div class="flex items-center justify-between">
                  <Badge :variant="getStatusColor(integration.status)" class="text-xs">
                    {{ integration.status === 'coming_soon' ? 'Coming Soon' : integration.status }}
                  </Badge>
                  <Button variant="ghost" size="sm">
                    <ExternalLink class="w-3 h-3" />
                  </Button>
                </div>
              </CardContent>
            </Card>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
