<script setup lang="ts">
import { RouterView, useRoute, useRouter } from 'vue-router'
import { navItems } from '@/config/nav'
import { cn } from '@/lib/utils'
import Button from '@/components/ui/button.vue'
import ModeToggle from '@/components/ui/theme-toggle.vue'
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  Bell, Activity, CheckCircle2, AlertCircle, Loader2,
  Settings, RefreshCw, X, Brain, Wifi, WifiOff
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface Notification {
  id: string
  type: 'info' | 'success' | 'warning' | 'error'
  title: string
  detail: string
  time: Date
  read: boolean
}

const route = useRoute()
const router = useRouter()
const sidebarOpen = ref(true)
const showNotifications = ref(false)
const housakyStatus = ref<'online' | 'offline' | 'loading'>('loading')
const housakyVersion = ref('')
const configSaving = ref(false)

const notifications = ref<Notification[]>([
  { id: '1', type: 'success', title: 'Agent started', detail: 'Housaky agent is running', time: new Date(Date.now() - 60000 * 2), read: false },
  { id: '2', type: 'info', title: 'Memory indexed', detail: '24 entries saved to SQLite', time: new Date(Date.now() - 60000 * 8), read: false },
  { id: '3', type: 'warning', title: 'Telegram channel', detail: 'Not configured', time: new Date(Date.now() - 3600000), read: true },
])

const unreadCount = computed(() => notifications.value.filter(n => !n.read).length)

const currentPageTitle = computed(
  () => navItems.find(item => item.path === route.path)?.title ?? 'Dashboard'
)

const notifColors: Record<string, string> = {
  info: 'bg-blue-500/10 text-blue-600 border-blue-200',
  success: 'bg-green-500/10 text-green-600 border-green-200',
  warning: 'bg-yellow-500/10 text-yellow-600 border-yellow-200',
  error: 'bg-red-500/10 text-red-600 border-red-200',
}

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

function markAllRead() {
  notifications.value.forEach(n => { n.read = true })
}

function dismissNotification(id: string) {
  notifications.value = notifications.value.filter(n => n.id !== id)
}

function formatRelative(d: Date): string {
  const diff = (Date.now() - d.getTime()) / 1000
  if (diff < 60) return 'just now'
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
  return `${Math.floor(diff / 3600)}h ago`
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
      <aside :class="cn('flex flex-col bg-card border-r transition-all duration-300 flex-shrink-0', sidebarOpen ? 'w-60' : 'w-14')">
        <!-- Logo -->
        <div class="p-3 border-b flex-shrink-0">
          <div class="flex items-center gap-2.5">
            <button
              @click="sidebarOpen = !sidebarOpen"
              class="w-8 h-8 rounded-lg bg-gradient-to-br from-primary to-primary/70 flex items-center justify-center hover:opacity-90 transition-opacity flex-shrink-0 shadow-sm"
            >
              <Brain class="w-4 h-4 text-primary-foreground" />
            </button>
            <div v-if="sidebarOpen" class="min-w-0">
              <h1 class="font-bold text-base leading-none">Housaky</h1>
              <div class="flex items-center gap-1.5 mt-0.5">
                <component
                  :is="housakyStatus === 'online' ? Wifi : housakyStatus === 'loading' ? Loader2 : WifiOff"
                  :class="cn(
                    'w-3 h-3',
                    housakyStatus === 'online' ? 'text-green-500' :
                    housakyStatus === 'loading' ? 'text-yellow-500 animate-spin' : 'text-red-500'
                  )"
                />
                <span class="text-xs text-muted-foreground truncate">
                  {{ housakyStatus === 'online' ? (housakyVersion || 'Online') :
                     housakyStatus === 'loading' ? 'Connecting…' : 'Offline' }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <!-- Navigation -->
        <nav class="flex-1 p-2 space-y-0.5 overflow-y-auto">
          <RouterLink
            v-for="item in navItems"
            :key="item.path"
            :to="item.path"
            :class="cn(
              'flex items-center gap-3 px-2.5 py-2 rounded-lg text-sm font-medium transition-all duration-150',
              route.path === item.path
                ? 'bg-primary text-primary-foreground shadow-sm'
                : 'text-muted-foreground hover:bg-muted hover:text-foreground'
            )"
            :title="!sidebarOpen ? item.title : undefined"
          >
            <component :is="item.icon" class="w-4 h-4 flex-shrink-0" />
            <span v-if="sidebarOpen" class="flex-1 truncate">{{ item.title }}</span>
            <span
              v-if="sidebarOpen && item.badge"
              class="text-[9px] font-bold uppercase px-1.5 py-0.5 rounded-full bg-green-500 text-white leading-none"
            >{{ item.badge }}</span>
          </RouterLink>
        </nav>

        <!-- Footer -->
        <div class="p-2 border-t flex-shrink-0">
          <div v-if="sidebarOpen" class="flex items-center justify-between mb-2 px-1">
            <div :class="cn(
              'flex items-center gap-1.5 text-xs',
              housakyStatus === 'online' ? 'text-green-600' :
              housakyStatus === 'loading' ? 'text-yellow-600' : 'text-red-600'
            )">
              <component
                :is="housakyStatus === 'online' ? CheckCircle2 : housakyStatus === 'loading' ? Loader2 : AlertCircle"
                :class="cn('w-3 h-3', housakyStatus === 'loading' && 'animate-spin')"
              />
              <span>{{ housakyStatus === 'online' ? 'Running' : housakyStatus === 'loading' ? 'Checking' : 'Stopped' }}</span>
            </div>
            <ModeToggle />
          </div>
          <div v-else class="flex justify-center">
            <ModeToggle />
          </div>
        </div>
      </aside>

      <!-- Main Content -->
      <main class="flex-1 flex flex-col overflow-hidden bg-background min-w-0">
        <!-- Top Bar -->
        <header class="sticky top-0 z-20 bg-background/90 backdrop-blur-md border-b px-5 py-2.5 flex-shrink-0">
          <div class="flex items-center justify-between gap-4">
            <div class="flex items-center gap-3 min-w-0">
              <h2 class="text-base font-semibold truncate">{{ currentPageTitle }}</h2>
              <span v-if="configSaving" class="flex items-center gap-1 text-xs text-blue-600">
                <Loader2 class="w-3 h-3 animate-spin" />
                Saving…
              </span>
            </div>

            <div class="flex items-center gap-1.5 flex-shrink-0">
              <Button
                v-if="housakyStatus === 'offline'"
                variant="outline"
                size="sm"
                class="text-yellow-600 border-yellow-300 hover:bg-yellow-50 h-8 text-xs gap-1"
                @click="router.push('/config')"
              >
                <AlertCircle class="w-3.5 h-3.5" />
                Setup
              </Button>

              <Button variant="ghost" size="sm" class="h-8 w-8 p-0" @click="checkStatus">
                <RefreshCw :class="['w-3.5 h-3.5', housakyStatus === 'loading' && 'animate-spin']" />
              </Button>

              <!-- Notification Bell -->
              <div class="relative">
                <Button
                  variant="ghost"
                  size="sm"
                  class="h-8 w-8 p-0 relative"
                  @click="showNotifications = !showNotifications"
                >
                  <Bell class="w-3.5 h-3.5" />
                  <span
                    v-if="unreadCount > 0"
                    class="absolute -top-0.5 -right-0.5 w-4 h-4 bg-red-500 text-white text-[9px] font-bold rounded-full flex items-center justify-center"
                  >{{ unreadCount }}</span>
                </Button>

                <!-- Notification Panel -->
                <div
                  v-if="showNotifications"
                  class="absolute right-0 top-full mt-2 w-80 bg-card border rounded-xl shadow-xl z-50 overflow-hidden"
                >
                  <div class="flex items-center justify-between px-4 py-3 border-b">
                    <span class="font-semibold text-sm">Notifications</span>
                    <div class="flex items-center gap-2">
                      <button @click="markAllRead" class="text-xs text-muted-foreground hover:text-foreground">Mark all read</button>
                      <button @click="showNotifications = false">
                        <X class="w-4 h-4 text-muted-foreground" />
                      </button>
                    </div>
                  </div>
                  <div class="max-h-72 overflow-y-auto">
                    <div v-if="notifications.length === 0" class="py-8 text-center text-sm text-muted-foreground">
                      No notifications
                    </div>
                    <div
                      v-for="n in notifications"
                      :key="n.id"
                      :class="['flex items-start gap-3 px-4 py-3 border-b last:border-0 transition-colors', !n.read ? 'bg-muted/30' : '']"
                    >
                      <div :class="['w-1.5 h-1.5 rounded-full mt-1.5 flex-shrink-0', !n.read ? 'bg-blue-500' : 'bg-transparent']" />
                      <div class="flex-1 min-w-0">
                        <p class="text-sm font-medium truncate">{{ n.title }}</p>
                        <p class="text-xs text-muted-foreground truncate">{{ n.detail }}</p>
                        <p class="text-[10px] text-muted-foreground mt-0.5">{{ formatRelative(n.time) }}</p>
                      </div>
                      <button @click="dismissNotification(n.id)" class="flex-shrink-0 mt-0.5">
                        <X class="w-3.5 h-3.5 text-muted-foreground hover:text-foreground" />
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </header>

        <!-- Click-outside to close notifications -->
        <div v-if="showNotifications" class="fixed inset-0 z-10" @click="showNotifications = false" />

        <!-- Page Content -->
        <div class="flex-1 overflow-y-auto">
          <RouterView />
        </div>
      </main>
    </div>
  </div>
</template>
