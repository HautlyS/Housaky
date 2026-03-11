<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
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
import { gateway, type Skill as GatewaySkill } from '@/lib/gateway'
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
  Folder,
  Zap,
  Bot,
  Server
} from 'lucide-vue-next'

interface Skill {
  name: string
  description: string
  version: string
  author?: string
  tags: string[]
  tools_count: number
  enabled: boolean
  location?: string
  source?: string
  installed?: boolean
}

interface McpServer {
  name: string
  description: string
  command: string
  args: string[]
  enabled: boolean
  status: string
  connected_count: number
}

const skills = ref<Skill[]>([])
const mcpServers = ref<McpServer[]>([])
const loading = ref(true)
const installing = ref(false)
const searchQuery = ref('')
const activeTab = ref('skills')
const housakyInstalled = ref(true)
const showInstallDialog = ref(false)
const selectedSkill = ref<Skill | null>(null)

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
      const result = await invoke<Skill[]>('get_marketplace_skills')
      skills.value = result
    } else {
      // Web mode sample data
      skills.value = [
        { name: 'Git Helper', description: 'Automates git workflows', version: '1.2.0', author: 'Housaky Team', tags: ['git', 'automation'], tools_count: 5, enabled: true, source: 'marketplace', installed: true },
        { name: 'Web Scraper', description: 'Extract data from websites', version: '0.8.0', author: 'Community', tags: ['scraping', 'data'], tools_count: 3, enabled: true, source: 'marketplace', installed: true },
      ]
    }
  } catch (e) {
    console.error(e)
  } finally {
    loading.value = false
  }
}

async function loadMcpServers() {
  try {
    if (isTauri) {
      const result = await invoke<McpServer[]>('get_mcp_servers')
      mcpServers.value = result
    } else {
      mcpServers.value = [
        { name: 'Filesystem', description: 'File operations', command: 'npx', args: ['-y', '@modelcontextprotocol/server-filesystem', '/'], enabled: true, status: 'stopped', connected_count: 0 },
        { name: 'Brave Search', description: 'Web search', command: 'npx', args: ['-y', '@modelcontextprotocol/server-brave-search'], enabled: false, status: 'stopped', connected_count: 0 },
      ]
    }
  } catch (e) {
    console.error('Failed to load MCP servers:', e)
  }
}

const filteredSkills = computed(() => {
  let result = skills.value

  if (activeTab.value === 'installed') {
    result = result.filter(s => s.installed)
  } else if (activeTab.value === 'available') {
    result = result.filter(s => !s.installed)
  }

  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(s => 
      s.name.toLowerCase().includes(query) ||
      s.description.toLowerCase().includes(query) ||
      s.tags?.some(t => t.toLowerCase().includes(query))
    )
  }

  return result
})

const filteredMcpServers = computed(() => {
  let result = mcpServers.value

  if (activeTab.value === 'mcp-active') {
    result = result.filter(s => s.enabled)
  } else if (activeTab.value === 'mcp-inactive') {
    result = result.filter(s => !s.enabled)
  }

  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    result = result.filter(s => 
      s.name.toLowerCase().includes(query) ||
      s.description.toLowerCase().includes(query)
    )
  }

  return result
})

async function installSkill(skill: Skill) {
  if (!isTauri) {
    alert('Running in web mode - install not available')
    return
  }
  installing.value = true
  try {
    await invoke('install_market_skill', { skillName: skill.name })
    skill.installed = true
    showInstallDialog.value = false
  } catch (e) {
    console.error(e)
    alert(`Failed to install: ${e}`)
  } finally {
    installing.value = false
  }
}

async function uninstallSkill(skill: Skill) {
  if (!isTauri) return
  try {
    await invoke('uninstall_skill', { skillName: skill.name })
    skill.installed = false
  } catch (e) {
    console.error(e)
  }
}

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

async function toggleMcpServer(server: McpServer) {
  if (!isTauri) return
  try {
    await invoke('configure_mcp_server', { 
      name: server.name, 
      enabled: !server.enabled,
      command: server.command,
      args: server.args
    })
    server.enabled = !server.enabled
  } catch (e) {
    console.error(e)
  }
}

async function refreshSkills() {
  await loadSkills()
  await loadMcpServers()
}

function openInstallDialog(skill: Skill) {
  selectedSkill.value = skill
  showInstallDialog.value = true
}

function viewSkillDetails(skill: Skill) {
  alert(`Skill: ${skill.name}\nVersion: ${skill.version}\nAuthor: ${skill.author || 'Unknown'}\nTools: ${skill.tools_count}\nSource: ${skill.source}\n\n${skill.description}`)
}

onMounted(async () => {
  await checkHousaky()
  await loadSkills()
  await loadMcpServers()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">Skills & MCPs</h1>
        <p class="text-sm text-muted-foreground">Extend Housaky with skills and MCP servers</p>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" @click="refreshSkills" :disabled="loading">
          <RefreshCw :class="['w-4 h-4 mr-2', loading && 'animate-spin']" />
          Refresh
        </Button>
      </div>
    </div>

    <div v-if="!housakyInstalled" class="p-4 rounded-lg border border-yellow-500 bg-yellow-50 dark:bg-yellow-900/20">
      <div class="flex items-center gap-2 text-yellow-600 dark:text-yellow-400">
        <XCircle class="w-5 h-5" />
        <span>Housaky is not installed. Skills and MCPs require Housaky to function.</span>
      </div>
    </div>

    <div class="flex gap-4 flex-wrap">
      <div class="relative flex-1 max-w-md">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
        <Input v-model="searchQuery" :placeholder="activeTab.startsWith('mcp') ? 'Search MCP servers...' : 'Search skills...'" class="pl-10" />
      </div>
      <Tabs v-model="activeTab" defaultValue="skills">
        <TabsList>
          <TabsTrigger value="skills">
            <Wrench class="w-4 h-4 mr-2" />
            Skills ({{ skills.filter(s => s.installed).length }})
          </TabsTrigger>
          <TabsTrigger value="available">
            <Download class="w-4 h-4 mr-2" />
            Available ({{ skills.filter(s => !s.installed).length }})
          </TabsTrigger>
          <TabsTrigger value="mcp-active">
            <Server class="w-4 h-4 mr-2" />
            MCPs ({{ mcpServers.filter(s => s.enabled).length }})
          </TabsTrigger>
        </TabsList>
      </Tabs>
    </div>

    <div v-if="loading" class="flex items-center justify-center py-12">
      <Loader2 class="w-8 h-8 animate-spin text-muted-foreground" />
    </div>

    <!-- Skills: Installed -->
    <div v-else-if="activeTab === 'skills'" class="space-y-4">
      <div v-if="filteredSkills.length === 0" class="text-center py-12">
        <Package class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
        <h3 class="text-lg font-semibold mb-2">No skills installed</h3>
        <p class="text-muted-foreground mb-4">Browse available skills to extend Housaky</p>
        <Button @click="activeTab = 'available'">
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
                <div :class="['w-10 h-10 rounded-lg flex items-center justify-center', skill.enabled ? 'bg-primary/10' : 'bg-muted']">
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
              <Button variant="outline" size="sm" class="text-destructive hover:text-destructive" @click="uninstallSkill(skill)">
                <Trash2 class="w-3 h-3" />
              </Button>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>

    <!-- Skills: Available -->
    <div v-else-if="activeTab === 'available'" class="space-y-4">
      <div v-if="filteredSkills.length === 0" class="text-center py-12">
        <Download class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
        <h3 class="text-lg font-semibold mb-2">No skills available</h3>
        <p class="text-muted-foreground">Check back later for more skills</p>
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
                <div class="w-10 h-10 rounded-lg flex items-center justify-center bg-blue-50 dark:bg-blue-900/20">
                  <Zap class="w-5 h-5 text-blue-500" />
                </div>
                <div>
                  <CardTitle class="text-base">{{ skill.name }}</CardTitle>
                  <CardDescription class="text-xs">v{{ skill.version }}</CardDescription>
                </div>
              </div>
              <Badge variant="outline">{{ skill.source }}</Badge>
            </div>
          </CardHeader>
          <CardContent>
            <p class="text-sm text-muted-foreground mb-3 line-clamp-2">{{ skill.description }}</p>
            
            <div class="flex flex-wrap gap-1 mb-3">
              <Badge v-for="tag in (skill.tags ?? []).slice(0, 3)" :key="tag" variant="outline" class="text-xs">
                {{ tag }}
              </Badge>
            </div>
            
            <div class="flex items-center justify-between text-xs text-muted-foreground mb-4">
              <span class="flex items-center gap-1">
                <Package class="w-3 h-3" />
                {{ skill.tools_count }} tools
              </span>
              <span>{{ skill.author }}</span>
            </div>

            <Button 
              class="w-full" 
              @click="installSkill(skill)"
              :disabled="installing || !housakyInstalled"
            >
              <Download class="w-4 h-4 mr-2" />
              {{ installing ? 'Installing...' : 'Install' }}
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>

    <!-- MCP Servers -->
    <div v-else-if="activeTab.startsWith('mcp')" class="space-y-4">
      <div v-if="filteredMcpServers.length === 0" class="text-center py-12">
        <Server class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
        <h3 class="text-lg font-semibold mb-2">No MCP servers</h3>
        <p class="text-muted-foreground">Configure MCP servers to extend capabilities</p>
      </div>
      <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <Card 
          v-for="server in filteredMcpServers" 
          :key="server.name" 
          class="hover:shadow-md transition-all hover:-translate-y-0.5"
        >
          <CardHeader class="pb-3">
            <div class="flex items-start justify-between">
              <div class="flex items-center gap-3">
                <div :class="['w-10 h-10 rounded-lg flex items-center justify-center', server.enabled ? 'bg-green-500/10' : 'bg-muted']">
                  <Server :class="['w-5 h-5', server.enabled ? 'text-green-500' : 'text-muted-foreground']" />
                </div>
                <div>
                  <CardTitle class="text-base">{{ server.name }}</CardTitle>
                  <CardDescription class="text-xs">MCP Server</CardDescription>
                </div>
              </div>
              <Badge :variant="server.enabled ? 'success' : 'secondary'">
                {{ server.enabled ? 'Active' : 'Inactive' }}
              </Badge>
            </div>
          </CardHeader>
          <CardContent>
            <p class="text-sm text-muted-foreground mb-3 line-clamp-2">{{ server.description }}</p>
            
            <div class="text-xs text-muted-foreground mb-3 font-mono bg-muted p-2 rounded">
              {{ server.command }} {{ server.args.join(' ') }}
            </div>
            
            <div class="flex items-center justify-between text-xs text-muted-foreground mb-4">
              <span class="flex items-center gap-1">
                <Bot class="w-3 h-3" />
                {{ server.connected_count }} connected
              </span>
              <span>{{ server.status }}</span>
            </div>

            <Button 
              variant="outline" 
              class="w-full"
              @click="toggleMcpServer(server)"
              :disabled="!housakyInstalled"
            >
              <Power class="w-3 h-3 mr-1" />
              {{ server.enabled ? 'Disable' : 'Enable' }}
            </Button>
          </CardContent>
        </Card>
      </div>
    </div>
  </div>
</template>
