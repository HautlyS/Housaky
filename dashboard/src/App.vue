<script setup lang="ts">
import { RouterView, useRoute, useRouter } from 'vue-router'
import { navItems } from '@/config/nav'
import { cn } from '@/lib/utils'
import Button from '@/components/ui/button.vue'
import ModeToggle from '@/components/ui/theme-toggle.vue'
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import {
  Bell, CheckCircle2, AlertCircle, Loader2,
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
  <div class="min-h-screen bg-background gradient-mesh">
    <div class="flex h-screen overflow-hidden">
      <!-- Sidebar -->
      <aside :class="cn(
        'flex flex-col backdrop-blur-xl border-r border-white/20 dark:border-white/10 transition-all duration-300 flex-shrink-0',
        'bg-white/50 dark:bg-[#0D0D0D]/80',
        sidebarOpen ? 'w-64' : 'w-16'
      )">
        <!-- Logo -->
        <div class="p-4 border-b border-gray-200/50 dark:border-white/10 flex-shrink-0">
          <div class="flex items-center gap-3">
            <div
              class="w-10 h-10 rounded-2xl bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 flex items-center justify-center hover:scale-105 transition-transform shadow-lg"
            >
              <Brain class="w-5 h-5 text-white" />
            </div>
            <div v-if="sidebarOpen" class="min-w-0">
              <h1 class="font-bold text-lg leading-none bg-gradient-to-r from-indigo-600 to-purple-600 bg-clip-text text-transparent">Housaky</h1>
              <div class="flex items-center gap-1.5 mt-1">
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
        <nav class="flex-1 p-3 space-y-1 overflow-y-auto">
          <RouterLink
            v-for="item in navItems"
            :key="item.path"
            :to="item.path"
            :class="cn(
              'flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium transition-all duration-200',
              route.path === item.path
                ? 'bg-gradient-to-r from-indigo-500 to-purple-500 text-white shadow-md shadow-indigo-500/25'
                : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-white/10 hover:text-gray-900 dark:hover:text-white'
            )"
            :title="!sidebarOpen ? item.title : undefined"
          >
            <component :is="item.icon" class="w-5 h-5 flex-shrink-0" />
            <span v-if="sidebarOpen" class="flex-1 truncate">{{ item.title }}</span>
            <span
              v-if="sidebarOpen && item.badge"
              class="text-[9px] font-bold uppercase px-2 py-0.5 rounded-full bg-white/20 text-white"
            >{{ item.badge }}</span>
          </RouterLink>
        </nav>

        <!-- Footer -->
        <div class="p-3 border-t border-gray-200/50 dark:border-white/10 flex-shrink-0">
          <div v-if="sidebarOpen" class="flex items-center justify-between mb-3 px-1">
            <div :class="cn(
              'flex items-center gap-2 text-xs font-medium',
              housakyStatus === 'online' ? 'text-green-600' :
              housakyStatus === 'loading' ? 'text-yellow-600' : 'text-red-500'
            )">
              <component
                :is="housakyStatus === 'online' ? CheckCircle2 : housakyStatus === 'loading' ? Loader2 : AlertCircle"
                :class="cn('w-3.5 h-3.5', housakyStatus === 'loading' && 'animate-spin')"
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
      <main class="flex-1 flex flex-col overflow-hidden bg-transparent min-w-0">
        <!-- Top Bar -->
        <header class="sticky top-0 z-20 backdrop-blur-xl bg-white/60 dark:bg-[#0D0D0D]/60 border-b border-gray-200/50 dark:border-white/10 px-6 py-3 flex-shrink-0">
          <div class="flex items-center justify-between gap-4">
            <div class="flex items-center gap-4 min-w-0">
              <h2 class="text-lg font-semibold text-gray-900 dark:text-white">{{ currentPageTitle }}</h2>
              <span v-if="configSaving" class="flex items-center gap-1 text-xs text-blue-600">
                <Loader2 class="w-3 h-3 animate-spin" />
                Saving…
              </span>
            </div>

            <div class="flex items-center gap-2 flex-shrink-0">
              <Button
                v-if="housakyStatus === 'offline'"
                variant="outline"
                size="sm"
                class="rounded-full text-xs gap-1.5 border-yellow-300 text-yellow-700 hover:bg-yellow-50"
                @click="router.push('/config')"
              >
                <AlertCircle class="w-3.5 h-3.5" />
                Setup
              </Button>

              <Button 
                variant="ghost" 
                size="sm" 
                class="rounded-full h-9 w-9 p-0 hover:bg-gray-100 dark:hover:bg-white/10"
                @click="checkStatus"
              >
                <RefreshCw :class="['w-4 h-4', housakyStatus === 'loading' && 'animate-spin text-indigo-500']" />
              </Button>

              <!-- Notification Bell -->
              <div class="relative">
                <Button
                  variant="ghost"
                  size="sm"
                  class="rounded-full h-9 w-9 p-0 hover:bg-gray-100 dark:hover:bg-white/10 relative"
                  @click="showNotifications = !showNotifications"
                >
                  <Bell class="w-4 h-4" />
                  <span
                    v-if="unreadCount > 0"
                    class="absolute -top-0.5 -right-0.5 w-4 h-4 bg-red-500 text-white text-[9px] font-bold rounded-full flex items-center justify-center"
                  >{{ unreadCount }}</span>
                </Button>

                <!-- Notification Panel -->
                <div
                  v-if="showNotifications"
                  class="absolute right-0 top-full mt-2 w-80 backdrop-blur-xl bg-white/90 dark:bg-[#1A1A1A]/90 border border-gray-200/50 dark:border-white/10 rounded-2xl shadow-xl apple-shadow-lg z-50 overflow-hidden"
                >
                  <div class="flex items-center justify-between px-4 py-3 border-b border-gray-200/50 dark:border-white/10">
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
                      :class="['flex items-start gap-3 px-4 py-3 border-b border-gray-100 dark:border-white/5 last:border-0 transition-colors', !n.read ? 'bg-indigo-500/5' : '']"
                    >
                      <div :class="['w-1.5 h-1.5 rounded-full mt-1.5 flex-shrink-0', !n.read ? 'bg-indigo-500' : 'bg-transparent']" />
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
        <div class="flex-1 overflow-y-auto p-6">
          <RouterView />
        </div>
      </main>
    </div>
  </div>
</template>
