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
  Wrench, 
  Plus, 
  Search, 
  Package, 
  RefreshCw,
  Trash2,
  Power,
  Settings,
  ExternalLink,
  Loader2,
  CheckCircle2,
  XCircle,
  Download,
  Folder
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface Skill {
  name: string
  description: string
  version: string
  author?: string
  tags: string[]
  tools_count: number
  enabled: boolean
  location?: string
}

const skills = ref<Skill[]>([])
const loading = ref(true)
const searchQuery = ref('')
const activeTab = ref('all')
const housakyInstalled = ref(true)

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

async function loadSkills() {
  loading.value = true
  try {
    if (isTauri) {
      const result = await invoke<Skill[]>('get_skills')
      skills.value = result
    }
  } catch (e) {
    console.error(e)
  } finally {
    loading.value = false
  }
}

const filteredSkills = computed(() => {
  let result = skills.value

  if (activeTab.value === 'enabled') {
    result = result.filter(s => s.enabled)
  } else if (activeTab.value === 'disabled') {
    result = result.filter(s => !s.enabled)
  }

  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(s => 
      s.name.toLowerCase().includes(query) ||
      s.description.toLowerCase().includes(query) ||
      s.tags.some(t => t.toLowerCase().includes(query))
    )
  }

  return result
})

async function toggleSkill(skill: Skill) {
  if (!isTauri) {
    alert('Running in server mode - skill toggle not available')
    return
  }
  try {
    await invoke('toggle_skill', { name: skill.name, enabled: !skill.enabled })
    skill.enabled = !skill.enabled
  } catch (e) {
    console.error(e)
  }
}

async function refreshSkills() {
  await loadSkills()
}

function installSkill() {
  alert('Skill store coming soon! Check GitHub for available skills.')
}

function viewSkillDetails(skill: Skill) {
  alert(`Skill: ${skill.name}\nVersion: ${skill.version}\nAuthor: ${skill.author || 'Unknown'}\nTools: ${skill.tools_count}\n\n${skill.description}`)
}

onMounted(async () => {
  await checkHousaky()
  await loadSkills()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">Skills</h1>
        <p class="text-sm text-muted-foreground">Extend Housaky with capabilities</p>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" @click="refreshSkills" :disabled="loading">
          <RefreshCw :class="['w-4 h-4 mr-2', loading && 'animate-spin']" />
          Refresh
        </Button>
        <Button size="sm" @click="installSkill" :disabled="!housakyInstalled">
          <Plus class="w-4 h-4 mr-2" />
          Add Skill
        </Button>
      </div>
    </div>

    <div v-if="!housakyInstalled" class="p-4 rounded-lg border border-yellow-500 bg-yellow-50 dark:bg-yellow-900/20">
      <div class="flex items-center gap-2 text-yellow-600 dark:text-yellow-400">
        <XCircle class="w-5 h-5" />
        <span>Housaky is not installed. Skills require Housaky to function.</span>
      </div>
    </div>

    <div class="flex gap-4 flex-wrap">
      <div class="relative flex-1 max-w-md">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
        <Input v-model="searchQuery" placeholder="Search skills..." class="pl-10" />
      </div>
      <Tabs v-model="activeTab" defaultValue="all">
        <TabsList>
          <TabsTrigger value="all">All ({{ skills.length }})</TabsTrigger>
          <TabsTrigger value="enabled">Enabled ({{ skills.filter(s => s.enabled).length }})</TabsTrigger>
          <TabsTrigger value="disabled">Disabled</TabsTrigger>
        </TabsList>
      </Tabs>
    </div>

    <div v-if="loading" class="flex items-center justify-center py-12">
      <Loader2 class="w-8 h-8 animate-spin text-muted-foreground" />
    </div>

    <div v-else-if="filteredSkills.length === 0" class="text-center py-12">
      <Wrench class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
      <h3 class="text-lg font-semibold mb-2">No skills found</h3>
      <p class="text-muted-foreground mb-4">
        {{ searchQuery ? 'Try a different search term' : 'Install skills to extend Housaky' }}
      </p>
      <Button @click="installSkill">
        <Download class="w-4 h-4 mr-2" />
        Browse Skills
      </Button>
    </div>

    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <Card 
        v-for="skill in filteredSkills" 
        :key="skill.name" 
        class="hover:shadow-md transition-all hover:-translate-y-0.5"
      >
        <CardHeader class="pb-3">
          <div class="flex items-start justify-between">
            <div class="flex items-center gap-3">
              <div :class="[
                'w-10 h-10 rounded-lg flex items-center justify-center',
                skill.enabled ? 'bg-primary/10' : 'bg-muted'
              ]">
                <Wrench :class="['w-5 h-5', skill.enabled ? 'text-primary' : 'text-muted-foreground']" />
              </div>
              <div>
                <CardTitle class="text-base">{{ skill.name }}</CardTitle>
                <CardDescription class="text-xs">v{{ skill.version }}</CardDescription>
              </div>
            </div>
            <Badge :variant="skill.enabled ? 'success' : 'secondary'">
              {{ skill.enabled ? 'On' : 'Off' }}
            </Badge>
          </div>
        </CardHeader>
        <CardContent>
          <p class="text-sm text-muted-foreground mb-3 line-clamp-2">{{ skill.description }}</p>
          
          <div class="flex flex-wrap gap-1 mb-3">
            <Badge v-for="tag in (skill.tags ?? []).slice(0, 3)" :key="tag" variant="outline" class="text-xs">
              {{ tag }}
            </Badge>
            <Badge v-if="(skill.tags ?? []).length > 3" variant="outline" class="text-xs">
              +{{ (skill.tags ?? []).length - 3 }}
            </Badge>
          </div>
          
          <div class="flex items-center justify-between text-xs text-muted-foreground mb-4">
            <span class="flex items-center gap-1">
              <Package class="w-3 h-3" />
              {{ skill.tools_count }} tools
            </span>
            <span v-if="skill.author">{{ skill.author }}</span>
          </div>

          <div class="flex gap-2">
            <Button 
              variant="outline" 
              size="sm" 
              class="flex-1"
              @click="toggleSkill(skill)"
              :disabled="!housakyInstalled"
            >
              <Power class="w-3 h-3 mr-1" />
              {{ skill.enabled ? 'Disable' : 'Enable' }}
            </Button>
            <Button variant="outline" size="sm" @click="viewSkillDetails(skill)">
              <Settings class="w-3 h-3" />
            </Button>
            <Button variant="outline" size="sm" class="text-destructive hover:text-destructive">
              <Trash2 class="w-3 h-3" />
            </Button>
          </div>

          <div v-if="skill.location" class="mt-3 pt-3 border-t">
            <div class="flex items-center gap-1 text-xs text-muted-foreground">
              <Folder class="w-3 h-3" />
              <span class="truncate">{{ skill.location }}</span>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  </div>
</template>
