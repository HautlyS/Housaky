<script setup lang="ts">
import { ref, onMounted } from 'vue'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import CardDescription from '@/components/ui/card-description.vue'
import Button from '@/components/ui/button.vue'
import Input from '@/components/ui/input.vue'
import Badge from '@/components/ui/badge.vue'
import { 
  Server, 
  Plus, 
  Trash2, 
  RefreshCw, 
  Play, 
  Square,
  Terminal,
  Loader2,
  CheckCircle2,
  AlertCircle,
  Settings
} from 'lucide-vue-next'

const GATEWAY_URL = import.meta.env.VITE_GATEWAY_URL || 'http://127.0.0.1:8080'

interface McpServer {
  name: string
  command: string
  args: string[]
  env: Record<string, string>
  status: string
  tools_count: number
}

const servers = ref<McpServer[]>([])
const loading = ref(false)
const error = ref('')
const success = ref('')
const showAddForm = ref(false)
const newServer = ref<McpServer>({
  name: '',
  command: '',
  args: [],
  env: {},
  status: 'new',
  tools_count: 0
})
const newArg = ref('')
const newEnvKey = ref('')
const newEnvValue = ref('')

async function loadServers() {
  loading.value = true
  error.value = ''
  
  try {
    const response = await fetch(`${GATEWAY_URL}/api/mcp`)
    if (response.ok) {
      const data = await response.json()
      servers.value = data.servers || []
    } else {
      error.value = 'Failed to load MCP servers'
    }
  } catch (e) {
    error.value = `Cannot connect to gateway: ${e}`
  } finally {
    loading.value = false
  }
}

async function addServer() {
  if (!newServer.value.name || !newServer.value.command) {
    error.value = 'Name and command are required'
    return
  }
  
  loading.value = true
  error.value = ''
  
  try {
    const response = await fetch(`${GATEWAY_URL}/api/mcp`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(newServer.value)
    })
    
    if (response.ok) {
      success.value = `MCP server '${newServer.value.name}' added!`
      await loadServers()
      showAddForm.value = false
      newServer.value = { name: '', command: '', args: [], env: {}, status: 'new', tools_count: 0 }
      setTimeout(() => { success.value = '' }, 3000)
    } else {
      const err = await response.json()
      error.value = err.error || 'Failed to add server'
    }
  } catch (e) {
    error.value = `Error: ${e}`
  } finally {
    loading.value = false
  }
}

async function removeServer(name: string) {
  if (!confirm(`Remove MCP server '${name}'?`)) return
  
  loading.value = true
  error.value = ''
  
  try {
    const response = await fetch(`${GATEWAY_URL}/api/mcp/${encodeURIComponent(name)}`, {
      method: 'DELETE'
    })
    
    if (response.ok) {
      success.value = `MCP server '${name}' removed!`
      await loadServers()
      setTimeout(() => { success.value = '' }, 3000)
    } else {
      const err = await response.json()
      error.value = err.error || 'Failed to remove server'
    }
  } catch (e) {
    error.value = `Error: ${e}`
  } finally {
    loading.value = false
  }
}

function addArg() {
  if (newArg.value.trim()) {
    newServer.value.args.push(newArg.value.trim())
    newArg.value = ''
  }
}

function removeArg(index: number) {
  newServer.value.args.splice(index, 1)
}

function addEnvVar() {
  if (newEnvKey.value.trim() && newEnvValue.value.trim()) {
    newServer.value.env[newEnvKey.value.trim()] = newEnvValue.value.trim()
    newEnvKey.value = ''
    newEnvValue.value = ''
  }
}

function removeEnvVar(key: string) {
  delete newServer.value.env[key]
}

onMounted(() => {
  loadServers()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h1 class="text-2xl font-bold tracking-tight flex items-center gap-2">
          <Server class="w-6 h-6 text-primary" />
          MCP Servers
        </h1>
        <p class="text-sm text-muted-foreground">
          Model Context Protocol server management
        </p>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" @click="loadServers" :disabled="loading">
          <RefreshCw :class="['w-4 h-4 mr-1.5', loading && 'animate-spin']" />
          Refresh
        </Button>
        <Button size="sm" @click="showAddForm = !showAddForm">
          <Plus class="w-4 h-4 mr-1.5" />
          Add Server
        </Button>
      </div>
    </div>

    <div v-if="error" class="p-3 rounded-lg bg-destructive/10 text-destructive flex items-center gap-2 text-sm">
      <AlertCircle class="w-4 h-4 flex-shrink-0" />{{ error }}
    </div>

    <div v-if="success" class="p-3 rounded-lg bg-green-500/10 text-green-700 flex items-center gap-2 text-sm">
      <CheckCircle2 class="w-4 h-4 flex-shrink-0" />{{ success }}
    </div>

    <!-- Add Server Form -->
    <Card v-if="showAddForm">
      <CardHeader>
        <CardTitle class="text-lg">Add MCP Server</CardTitle>
        <CardDescription>Configure a new Model Context Protocol server</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="grid md:grid-cols-2 gap-4">
          <div class="space-y-2">
            <label class="text-sm font-medium">Name *</label>
            <Input v-model="newServer.name" placeholder="my-mcp-server" />
          </div>
          <div class="space-y-2">
            <label class="text-sm font-medium">Command *</label>
            <Input v-model="newServer.command" placeholder="npx -y @modelcontextprotocol/server-filesystem" />
          </div>
        </div>
        
        <div class="space-y-2">
          <label class="text-sm font-medium">Arguments</label>
          <div class="flex gap-2">
            <Input v-model="newArg" placeholder="Add argument..." @keydown.enter.prevent="addArg" class="flex-1" />
            <Button variant="outline" size="sm" @click="addArg">Add</Button>
          </div>
          <div v-if="newServer.args.length > 0" class="flex flex-wrap gap-2 mt-2">
            <Badge v-for="(arg, i) in newServer.args" :key="i" variant="secondary" class="gap-1">
              {{ arg }}
              <button @click="removeArg(i)" class="ml-1 hover:text-destructive">×</button>
            </Badge>
          </div>
        </div>
        
        <div class="space-y-2">
          <label class="text-sm font-medium">Environment Variables</label>
          <div class="flex gap-2">
            <Input v-model="newEnvKey" placeholder="KEY" class="w-32" />
            <Input v-model="newEnvValue" placeholder="value" class="flex-1" />
            <Button variant="outline" size="sm" @click="addEnvVar">Add</Button>
          </div>
          <div v-if="Object.keys(newServer.env).length > 0" class="space-y-1 mt-2">
            <div v-for="(value, key) in newServer.env" :key="key" class="flex items-center justify-between text-sm bg-muted px-2 py-1 rounded">
              <code>{{ key }}={{ value }}</code>
              <button @click="removeEnvVar(key)" class="hover:text-destructive">×</button>
            </div>
          </div>
        </div>
        
        <div class="flex justify-end gap-2">
          <Button variant="outline" @click="showAddForm = false">Cancel</Button>
          <Button @click="addServer" :disabled="loading || !newServer.name || !newServer.command">
            <Loader2 v-if="loading" class="w-4 h-4 mr-1.5 animate-spin" />
            <Plus v-else class="w-4 h-4 mr-1.5" />
            Add Server
          </Button>
        </div>
      </CardContent>
    </Card>

    <!-- Server List -->
    <div v-if="loading && servers.length === 0" class="flex items-center justify-center py-12">
      <Loader2 class="w-8 h-8 animate-spin text-muted-foreground" />
    </div>

    <div v-else-if="servers.length === 0" class="text-center py-12 text-muted-foreground">
      <Server class="w-12 h-12 mx-auto mb-3 opacity-50" />
      <p>No MCP servers configured</p>
      <p class="text-sm mt-1">Click "Add Server" to configure one</p>
    </div>

    <div v-else class="grid gap-4">
      <Card v-for="server in servers" :key="server.name">
        <CardContent class="pt-6">
          <div class="flex items-start justify-between">
            <div class="space-y-1">
              <div class="flex items-center gap-2">
                <Terminal class="w-4 h-4 text-muted-foreground" />
                <span class="font-semibold">{{ server.name }}</span>
                <Badge :variant="server.status === 'running' ? 'success' : 'secondary'" class="text-xs">
                  {{ server.status }}
                </Badge>
              </div>
              <code class="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded">{{ server.command }}</code>
              <div v-if="server.args.length > 0" class="flex flex-wrap gap-1 mt-1">
                <span v-for="arg in server.args" :key="arg" class="text-xs bg-muted/50 px-1.5 py-0.5 rounded">{{ arg }}</span>
              </div>
              <p v-if="server.tools_count > 0" class="text-xs text-muted-foreground mt-1">
                {{ server.tools_count }} tools available
              </p>
            </div>
            <div class="flex items-center gap-1">
              <Button variant="ghost" size="sm" class="h-8 w-8 p-0 text-destructive hover:text-destructive" @click="removeServer(server.name)">
                <Trash2 class="w-4 h-4" />
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>

    <!-- Help Card -->
    <Card>
      <CardHeader>
        <CardTitle class="text-sm flex items-center gap-2">
          <Settings class="w-4 h-4" />
          Quick Start
        </CardTitle>
      </CardHeader>
      <CardContent class="text-sm text-muted-foreground space-y-2">
        <p>MCP (Model Context Protocol) servers extend Housaky with additional tools and capabilities.</p>
        <p class="font-mono text-xs bg-muted p-2 rounded">
          # Example: Filesystem server<br>
          npx -y @modelcontextprotocol/server-filesystem /path/to/dir
        </p>
        <p class="font-mono text-xs bg-muted p-2 rounded">
          # Example: GitHub server<br>
          npx -y @modelcontextprotocol/server-github
        </p>
      </CardContent>
    </Card>
  </div>
</template>
