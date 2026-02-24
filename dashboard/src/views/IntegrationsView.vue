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
import ScrollArea from '@/components/ui/scroll-area.vue'
import { 
  Package, 
  Search,
  CheckCircle2,
  Clock,
  XCircle,
  ExternalLink,
  Info,
  MessageSquare,
  Cpu,
  Cloud,
  Wrench,
  Music,
  Home,
  Palette,
  Share2,
  Layers
} from 'lucide-vue-next'

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
  // Chat
  { name: 'Telegram', description: 'Bot API — long-polling', category: 'chat', status: 'active' },
  { name: 'Discord', description: 'Servers, channels & DMs', category: 'chat', status: 'active' },
  { name: 'Slack', description: 'Workspace apps via Web API', category: 'chat', status: 'active' },
  { name: 'WhatsApp', description: 'Meta Cloud API via webhook', category: 'chat', status: 'available' },
  { name: 'Signal', description: 'Privacy-focused via signal-cli', category: 'chat', status: 'coming_soon' },
  { name: 'iMessage', description: 'macOS AppleScript bridge', category: 'chat', status: 'available' },
  { name: 'Matrix', description: 'Matrix protocol (Element)', category: 'chat', status: 'available' },
  { name: 'Microsoft Teams', description: 'Enterprise chat support', category: 'chat', status: 'coming_soon' },
  // AI Models
  { name: 'OpenRouter', description: '200+ models, 1 API key', category: 'ai_model', status: 'active' },
  { name: 'Anthropic', description: 'Claude 3.5/4 Sonnet & Opus', category: 'ai_model', status: 'available' },
  { name: 'OpenAI', description: 'GPT-4o, GPT-5, o1', category: 'ai_model', status: 'available' },
  { name: 'Google', description: 'Gemini 2.5 Pro/Flash', category: 'ai_model', status: 'available' },
  { name: 'DeepSeek', description: 'DeepSeek V3 & R1', category: 'ai_model', status: 'available' },
  { name: 'Ollama', description: 'Local models', category: 'ai_model', status: 'available' },
  { name: 'Groq', description: 'Fast inference', category: 'ai_model', status: 'available' },
  { name: 'Mistral', description: 'Mistral AI models', category: 'ai_model', status: 'available' },
  // Tools & Automation
  { name: 'Composio', description: '1000+ OAuth apps', category: 'tools_automation', status: 'available' },
  { name: 'Brave Search', description: 'Web search API', category: 'tools_automation', status: 'available' },
  { name: 'GitHub', description: 'Repo management', category: 'tools_automation', status: 'available' },
  { name: 'Notion', description: 'Workspace integration', category: 'productivity', status: 'coming_soon' },
  // Smart Home
  { name: 'Home Assistant', description: 'Smart home hub', category: 'smart_home', status: 'coming_soon' },
  { name: 'Philips Hue', description: 'Smart lighting', category: 'smart_home', status: 'coming_soon' },
]

async function loadIntegrations() {
  loading.value = true
  try {
    setTimeout(() => {
      integrations.value = sampleIntegrations
      loading.value = false
    }, 500)
  } catch (e) {
    console.error(e)
    loading.value = false
  }
}

function filteredIntegrations() {
  let result = integrations.value

  if (activeTab.value === 'active') {
    result = result.filter(i => i.status === 'active')
  } else if (activeTab.value === 'available') {
    result = result.filter(i => i.status === 'available')
  } else if (activeTab.value === 'coming_soon') {
    result = result.filter(i => i.status === 'coming_soon')
  }

  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(i => 
      i.name.toLowerCase().includes(query) ||
      i.description.toLowerCase().includes(query)
    )
  }

  return result
}

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

const categories = computed(() => {
  const cats = new Set(integrations.value.map(i => i.category))
  return Array.from(cats)
})

onMounted(() => {
  loadIntegrations()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-bold">Integrations</h1>
        <p class="text-muted-foreground">Browse 75+ integrations across 9 categories</p>
      </div>
    </div>

    <!-- Stats -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ integrations.filter(i => i.status === 'active').length }}</div>
          <p class="text-sm text-muted-foreground">Active</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ integrations.filter(i => i.status === 'available').length }}</div>
          <p class="text-sm text-muted-foreground">Available</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ integrations.filter(i => i.status === 'coming_soon').length }}</div>
          <p class="text-sm text-muted-foreground">Coming Soon</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ categories.length }}</div>
          <p class="text-sm text-muted-foreground">Categories</p>
        </CardContent>
      </Card>
    </div>

    <!-- Search and Filter -->
    <div class="flex gap-4">
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
    </div>

    <!-- Category Sections -->
    <div v-for="category in categories" :key="category" class="space-y-4">
      <div class="flex items-center gap-2">
        <component :is="categoryIcons[category]" class="w-5 h-5 text-muted-foreground" />
        <h2 class="text-xl font-semibold">{{ categoryLabels[category] }}</h2>
        <Badge variant="outline">{{ filteredIntegrations().filter(i => i.category === category).length }}</Badge>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card 
          v-for="integration in filteredIntegrations().filter(i => i.category === category)" 
          :key="integration.name"
          class="hover:shadow-md transition-shadow cursor-pointer"
        >
          <CardHeader class="pb-2">
            <div class="flex items-start justify-between">
              <CardTitle class="text-base">{{ integration.name }}</CardTitle>
              <component 
                :is="getStatusIcon(integration.status)" 
                class="w-4 h-4"
                :class="{
                  'text-green-500': integration.status === 'active',
                  'text-muted-foreground': integration.status === 'available',
                  'text-yellow-500': integration.status === 'coming_soon'
                }"
              />
            </div>
          </CardHeader>
          <CardContent>
            <p class="text-sm text-muted-foreground">{{ integration.description }}</p>
            <div class="flex items-center justify-between mt-4">
              <Badge :variant="getStatusColor(integration.status)" class="text-xs">
                {{ integration.status === 'coming_soon' ? 'Coming Soon' : integration.status }}
              </Badge>
              <Button variant="ghost" size="sm">
                <Info class="w-4 h-4" />
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>

    <!-- Empty State -->
    <Card v-if="filteredIntegrations().length === 0 && !loading" class="p-8">
      <div class="text-center">
        <Package class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
        <h3 class="text-lg font-semibold mb-2">No integrations found</h3>
        <p class="text-muted-foreground">
          Try a different search term
        </p>
      </div>
    </Card>
  </div>
</template>
