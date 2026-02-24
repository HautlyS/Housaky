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
  Wrench, 
  Plus, 
  Search, 
  Package, 
  BookOpen,
  Download,
  Trash2,
  RefreshCw
} from 'lucide-vue-next'

interface Skill {
  name: string
  description: string
  version: string
  author?: string
  tags: string[]
  tools_count: number
  enabled: boolean
}

const skills = ref<Skill[]>([])
const loading = ref(true)
const searchQuery = ref('')
const activeTab = ref('all')

const sampleSkills: Skill[] = [
  {
    name: 'Git Helper',
    description: 'Automates git workflows, commits, and PR creation',
    version: '1.2.0',
    author: 'Housaky Team',
    tags: ['git', 'automation', 'productivity'],
    tools_count: 5,
    enabled: true,
  },
  {
    name: 'Web Scraper',
    description: 'Extract data from websites using CSS selectors',
    version: '0.8.0',
    author: 'Community',
    tags: ['scraping', 'data', 'web'],
    tools_count: 3,
    enabled: true,
  },
  {
    name: 'Database Manager',
    description: 'Execute queries and manage databases',
    version: '2.0.0',
    author: 'Housaky Team',
    tags: ['database', 'sql', 'data'],
    tools_count: 8,
    enabled: false,
  },
  {
    name: 'API Tester',
    description: 'Test REST APIs with assertions',
    version: '1.5.0',
    author: 'Community',
    tags: ['api', 'testing', 'developer'],
    tools_count: 4,
    enabled: true,
  },
  {
    name: 'File Organizer',
    description: 'Automatically organize files by type, date, or custom rules',
    version: '0.5.0',
    author: 'Community',
    tags: ['files', 'automation', 'organization'],
    tools_count: 2,
    enabled: false,
  },
]

async function loadSkills() {
  loading.value = true
  try {
    // Use sample data for now - in real app would call Tauri command
    // const result = await invoke<Skill[]>('get_skills')
    setTimeout(() => {
      skills.value = sampleSkills
      loading.value = false
    }, 500)
  } catch (e) {
    console.error(e)
    loading.value = false
  }
}

function filteredSkills() {
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
}

function toggleSkill(skill: Skill) {
  skill.enabled = !skill.enabled
}

function installSkill() {
  // Would open a dialog or navigate to skill store
  alert('Skill store coming soon!')
}

onMounted(() => {
  loadSkills()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-bold">Skills</h1>
        <p class="text-muted-foreground">Extend Housaky with capabilities</p>
      </div>
      <Button @click="installSkill">
        <Plus class="w-4 h-4 mr-2" />
        Add Skill
      </Button>
    </div>

    <!-- Search and Filter -->
    <div class="flex gap-4">
      <div class="relative flex-1 max-w-md">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
        <Input v-model="searchQuery" placeholder="Search skills..." class="pl-10" />
      </div>
      <Tabs v-model="activeTab" defaultValue="all">
        <TabsList>
          <TabsTrigger value="all">All</TabsTrigger>
          <TabsTrigger value="enabled">Enabled</TabsTrigger>
          <TabsTrigger value="disabled">Disabled</TabsTrigger>
        </TabsList>
      </Tabs>
    </div>

    <!-- Skills Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <Card v-for="skill in filteredSkills()" :key="skill.name" class="hover:shadow-md transition-shadow">
        <CardHeader>
          <div class="flex items-start justify-between">
            <div class="flex items-center gap-2">
              <div class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                <Wrench class="w-5 h-5 text-primary" />
              </div>
              <div>
                <CardTitle class="text-lg">{{ skill.name }}</CardTitle>
                <CardDescription class="text-xs">v{{ skill.version }}</CardDescription>
              </div>
            </div>
            <Badge :variant="skill.enabled ? 'success' : 'secondary'">
              {{ skill.enabled ? 'Enabled' : 'Disabled' }}
            </Badge>
          </div>
        </CardHeader>
        <CardContent>
          <p class="text-sm text-muted-foreground mb-4">{{ skill.description }}</p>
          
          <div class="flex flex-wrap gap-1 mb-4">
            <Badge v-for="tag in skill.tags" :key="tag" variant="outline" class="text-xs">
              {{ tag }}
            </Badge>
          </div>
          
          <div class="flex items-center justify-between text-sm text-muted-foreground">
            <span class="flex items-center gap-1">
              <Package class="w-4 h-4" />
              {{ skill.tools_count }} tools
            </span>
            <span v-if="skill.author">{{ skill.author }}</span>
          </div>

          <div class="flex gap-2 mt-4">
            <Button 
              variant="outline" 
              size="sm" 
              class="flex-1"
              @click="toggleSkill(skill)"
            >
              {{ skill.enabled ? 'Disable' : 'Enable' }}
            </Button>
            <Button variant="outline" size="sm">
              <RefreshCw class="w-4 h-4" />
            </Button>
            <Button variant="outline" size="sm">
              <Trash2 class="w-4 h-4" />
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>

    <!-- Empty State -->
    <Card v-if="filteredSkills().length === 0 && !loading" class="p-8">
      <div class="text-center">
        <Wrench class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
        <h3 class="text-lg font-semibold mb-2">No skills found</h3>
        <p class="text-muted-foreground mb-4">
          {{ searchQuery ? 'Try a different search term' : 'Install skills to extend Housaky' }}
        </p>
        <Button @click="installSkill">
          <Download class="w-4 h-4 mr-2" />
          Browse Skill Store
        </Button>
      </div>
    </Card>
  </div>
</template>
