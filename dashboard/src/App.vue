<script setup lang="ts">
import { RouterView, useRoute, useRouter } from 'vue-router'
import { navItems } from '@/config/nav'
import { cn } from '@/lib/utils'
import Button from '@/components/ui/button.vue'
import ModeToggle from '@/components/ui/theme-toggle.vue'
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { 
  Menu, 
  X, 
  Bell, 
  Search,
  Activity,
  CheckCircle2,
  AlertCircle,
  Loader2
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const route = useRoute()
const router = useRouter()
const sidebarOpen = ref(true)
const searchOpen = ref(false)
const housakyStatus = ref<'online' | 'offline' | 'loading'>('loading')
const housakyVersion = ref('')

async function checkStatus() {
  housakyStatus.value = 'loading'
  try {
    if (!isTauri) {
      housakyStatus.value = 'offline'
      return
    }
    const installed = await invoke<boolean>('check_housaky_installed')
    if (installed) {
      const status = await invoke<{ version: string }>('get_status')
      housakyVersion.value = status.version
      housakyStatus.value = 'online'
    } else {
      housakyStatus.value = 'offline'
    }
  } catch {
    housakyStatus.value = 'offline'
  }
}

function toggleSidebar() {
  sidebarOpen.value = !sidebarOpen.value
}

onMounted(() => {
  checkStatus()
  setInterval(checkStatus, 30000)
})
</script>

<template>
  <div class="min-h-screen bg-background">
    <div class="flex h-screen overflow-hidden">
      <!-- Sidebar -->
      <aside 
        :class="cn(
          'flex flex-col bg-card border-r transition-all duration-300',
          sidebarOpen ? 'w-64' : 'w-16'
        )"
      >
        <!-- Logo -->
        <div class="p-4 border-b">
          <div class="flex items-center gap-3">
            <button 
              @click="toggleSidebar"
              class="w-8 h-8 rounded-lg bg-primary flex items-center justify-center hover:bg-primary/90 transition-colors"
            >
              <span class="text-primary-foreground font-bold">H</span>
            </button>
            <div v-if="sidebarOpen">
              <h1 class="font-bold text-lg">Housaky</h1>
              <div class="flex items-center gap-1.5">
                <div :class="cn(
                  'w-2 h-2 rounded-full',
                  housakyStatus === 'online' ? 'bg-green-500' : 
                  housakyStatus === 'loading' ? 'bg-yellow-500 animate-pulse' : 'bg-red-500'
                )"></div>
                <span class="text-xs text-muted-foreground">
                  {{ housakyStatus === 'online' ? housakyVersion || 'Online' : 
                     housakyStatus === 'loading' ? 'Checking...' : 'Offline' }}
                </span>
              </div>
            </div>
          </div>
        </div>
        
        <!-- Navigation -->
        <nav class="flex-1 p-2 space-y-1 overflow-y-auto">
          <RouterLink
            v-for="item in navItems"
            :key="item.path"
            :to="item.path"
            :class="cn(
              'flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors',
              route.path === item.path
                ? 'bg-primary text-primary-foreground shadow-sm'
                : 'text-muted-foreground hover:bg-muted hover:text-foreground'
            )"
            :title="!sidebarOpen ? item.title : undefined"
          >
            <component :is="item.icon" class="w-5 h-5 flex-shrink-0" />
            <span v-if="sidebarOpen">{{ item.title }}</span>
          </RouterLink>
        </nav>
        
        <!-- Status Footer -->
        <div v-if="sidebarOpen" class="p-4 border-t">
          <div class="flex items-center justify-between mb-2">
            <span class="text-xs text-muted-foreground">Agent Status</span>
            <div :class="cn(
              'flex items-center gap-1 text-xs',
              housakyStatus === 'online' ? 'text-green-600' : 
              housakyStatus === 'loading' ? 'text-yellow-600' : 'text-red-600'
            )">
              <component :is="housakyStatus === 'online' ? CheckCircle2 : 
                             housakyStatus === 'loading' ? Loader2 : AlertCircle" 
                         :class="cn('w-3 h-3', housakyStatus === 'loading' && 'animate-spin')" />
              {{ housakyStatus === 'online' ? 'Running' : 
                 housakyStatus === 'loading' ? 'Checking' : 'Stopped' }}
            </div>
          </div>
          <ModeToggle />
        </div>
        
        <!-- Collapsed Footer -->
        <div v-else class="p-2 border-t">
          <ModeToggle />
        </div>
      </aside>
      
      <!-- Main Content -->
      <main class="flex-1 overflow-y-auto bg-background">
        <!-- Top Bar -->
        <header class="sticky top-0 z-10 bg-background/80 backdrop-blur-sm border-b px-6 py-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-4">
              <h2 class="text-lg font-semibold">
                {{ navItems.find(item => item.path === route.path)?.title || 'Dashboard' }}
              </h2>
            </div>
            <div class="flex items-center gap-3">
              <Button 
                v-if="housakyStatus === 'offline'"
                variant="outline" 
                size="sm"
                @click="router.push('/config')"
                class="text-yellow-600 border-yellow-200 hover:bg-yellow-50"
              >
                <AlertCircle class="w-4 h-4 mr-2" />
                Setup Required
              </Button>
              <Button variant="ghost" size="icon">
                <Bell class="w-4 h-4" />
              </Button>
            </div>
          </div>
        </header>
        
        <!-- Page Content -->
        <RouterView />
      </main>
    </div>
  </div>
</template>
