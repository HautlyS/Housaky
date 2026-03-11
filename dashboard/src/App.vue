<script setup lang="ts">
import { RouterView, useRoute, useRouter } from 'vue-router'
import { navItems } from '@/config/nav'
import { cn } from '@/lib/utils'
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import AsciiTitle from '@/components/ui/AsciiTitle.vue'
import AsciiDivider from '@/components/ui/AsciiDivider.vue'
import TextureBackground from '@/components/ui/TextureBackground.vue'
import {
  Bell, CheckCircle2, AlertCircle, Loader2,
  RefreshCw, X, Wifi, WifiOff,
  Terminal, Command, Palette
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
const showBgSettings = ref(false)
const housakyStatus = ref<'online' | 'offline' | 'loading'>('loading')
const housakyVersion = ref('')
const configSaving = ref(false)
const currentTime = ref(new Date())

type TextureType = 'velvet' | 'paper' | 'wood' | 'marble' | 'concrete' | 'fabric'
type ColorScheme = 'cyan' | 'magenta' | 'green' | 'amber' | 'mixed'

const textures: { value: TextureType; label: string }[] = [
  { value: 'velvet', label: 'Velvet' },
  { value: 'paper', label: 'Paper' },
  { value: 'wood', label: 'Wood' },
  { value: 'marble', label: 'Marble' },
  { value: 'concrete', label: 'Concrete' },
  { value: 'fabric', label: 'Fabric' }
]

const colorSchemes: { value: ColorScheme; label: string }[] = [
  { value: 'cyan', label: 'Cyan' },
  { value: 'magenta', label: 'Magenta' },
  { value: 'green', label: 'Green' },
  { value: 'amber', label: 'Amber' },
  { value: 'mixed', label: 'Mixed' }
]

const currentTexture = ref<TextureType>('velvet')
const currentColorScheme = ref<ColorScheme>('cyan')
const particleIntensity = ref(50)

const notifications = ref<Notification[]>([
  { id: '1', type: 'success', title: 'Agent started', detail: 'Housaky agent is running', time: new Date(Date.now() - 60000 * 2), read: false },
  { id: '2', type: 'info', title: 'Memory indexed', detail: '24 entries saved to SQLite', time: new Date(Date.now() - 60000 * 8), read: false },
  { id: '3', type: 'warning', title: 'Telegram channel', detail: 'Not configured', time: new Date(Date.now() - 3600000), read: true },
])

const unreadCount = computed(() => notifications.value.filter(n => !n.read).length)

const currentPageTitle = computed(
  () => navItems.find(item => item.path === route.path)?.title ?? 'Dashboard'
)

const statusColor = computed(() => {
  if (housakyStatus.value === 'online') return 'text-green-400'
  if (housakyStatus.value === 'loading') return 'text-yellow-400'
  return 'text-red-400'
})

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

function cycleTexture() {
  const idx = textures.findIndex(t => t.value === currentTexture.value)
  currentTexture.value = textures[(idx + 1) % textures.length].value
}

onMounted(() => {
  checkStatus()
  setInterval(checkStatus, 30000)
  setInterval(() => {
    currentTime.value = new Date()
  }, 1000)
})
</script>

<template>
  <div class="min-h-screen bg-background relative">
    <TextureBackground 
      :texture="currentTexture" 
      :color-scheme="currentColorScheme"
      :intensity="particleIntensity"
    />
    
    <div class="relative z-10 flex h-screen overflow-hidden">
      <aside :class="cn(
        'flex flex-col border-r-2 border-zinc-800 transition-all duration-300 flex-shrink-0',
        'bg-[#0d0d14]/95 backdrop-blur-sm',
        sidebarOpen ? 'w-64' : 'w-16'
      )">
        <div class="p-3 border-b-2 border-zinc-800 flex-shrink-0">
          <div class="flex items-center gap-3">
            <div class="relative">
              <div class="w-12 h-12 border-2 border-cyan-500/50 flex items-center justify-center bg-cyan-500/10 relative">
                <Terminal class="w-6 h-6 text-cyan-400" />
                <div class="absolute -top-1 -right-1 w-3 h-3 bg-cyan-400 animate-pulse" />
                <div class="absolute -bottom-1 -left-1 w-2 h-2 bg-cyan-400" />
              </div>
            </div>
            <div v-if="sidebarOpen" class="min-w-0">
              <AsciiTitle text="HOUSAKY" variant="minimal" color="cyan" size="sm" />
              <div class="flex items-center gap-1.5 mt-1">
                <component
                  :is="housakyStatus === 'online' ? Wifi : housakyStatus === 'loading' ? Loader2 : WifiOff"
                  :class="cn(
                    'w-3 h-3',
                    housakyStatus === 'online' ? 'text-green-400' :
                    housakyStatus === 'loading' ? 'text-yellow-400 animate-spin' : 'text-red-400'
                  )"
                />
                <span class="text-xs text-zinc-500 truncate font-mono">
                  {{ housakyStatus === 'online' ? (housakyVersion || 'v0.1.x') :
                     housakyStatus === 'loading' ? 'connecting...' : 'offline' }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <nav class="flex-1 p-2 space-y-0.5 overflow-y-auto">
          <RouterLink
            v-for="item in navItems"
            :key="item.path"
            :to="item.path"
            :class="cn(
              'flex items-center gap-3 px-3 py-3 text-sm font-mono transition-all duration-150 border-l-[3px]',
              route.path === item.path
                ? 'border-cyan-400 text-cyan-400 bg-cyan-400/10'
                : 'border-transparent text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/50 hover:border-zinc-600'
            )"
            :title="!sidebarOpen ? item.title : undefined"
          >
            <component :is="item.icon" class="w-5 h-5 flex-shrink-0" />
            <span v-if="sidebarOpen" class="flex-1 truncate text-xs uppercase tracking-wider">{{ item.title }}</span>
            <span
              v-if="sidebarOpen && item.badge"
              class="text-[9px] font-bold uppercase px-2 py-0.5 bg-cyan-500/20 text-cyan-400 border border-cyan-500/30"
            >{{ item.badge }}</span>
          </RouterLink>
        </nav>

        <div class="p-3 border-t-2 border-zinc-800 flex-shrink-0">
          <div v-if="sidebarOpen" class="space-y-3">
            <AsciiDivider variant="dots" color="muted" :length="30" />
            <div class="flex items-center justify-between">
              <div :class="cn('flex items-center gap-2 text-xs font-mono', statusColor)">
                <component
                  :is="housakyStatus === 'online' ? CheckCircle2 : housakyStatus === 'loading' ? Loader2 : AlertCircle"
                  :class="cn('w-3.5 h-3.5', housakyStatus === 'loading' && 'animate-spin')"
                />
                <span>{{ housakyStatus === 'online' ? 'ONLINE' : housakyStatus === 'loading' ? 'CHECK' : 'OFFLINE' }}</span>
              </div>
              <div class="flex items-center gap-1">
                <button @click="showBgSettings = !showBgSettings" class="p-1.5 text-zinc-500 hover:text-cyan-400 transition-colors" title="Background Settings">
                  <Palette class="w-4 h-4" />
                </button>
                <button @click="cycleTexture" class="text-zinc-600 hover:text-cyan-400 text-xs font-mono px-2 py-1 border border-zinc-700 hover:border-cyan-500/50">
                  [TEX]
                </button>
              </div>
            </div>
            <div class="text-[10px] text-zinc-600 font-mono text-center">
              {{ currentTime.toLocaleTimeString() }}
            </div>
          </div>
          <div v-else class="flex justify-center gap-2">
            <button @click="showBgSettings = !showBgSettings" class="p-2 text-zinc-600 hover:text-cyan-400">
              <Palette class="w-4 h-4" />
            </button>
            <button @click="cycleTexture" class="p-2 text-zinc-600 hover:text-cyan-400">
              <Command class="w-4 h-4" />
            </button>
          </div>
        </div>
      </aside>

      <main class="flex-1 flex flex-col overflow-hidden bg-transparent min-w-0">
        <header class="sticky top-0 z-20 bg-[#0d0d14]/90 backdrop-blur-md border-b-2 border-zinc-800 px-4 py-2 flex-shrink-0">
          <div class="flex items-center justify-between gap-4">
            <div class="flex items-center gap-4 min-w-0">
              <div class="flex items-center gap-2">
                <span class="text-cyan-400 font-mono text-sm">▸</span>
                <h2 class="text-sm font-mono uppercase tracking-widest text-zinc-200">{{ currentPageTitle }}</h2>
              </div>
              <span v-if="configSaving" class="flex items-center gap-1 text-xs text-cyan-400 font-mono">
                <Loader2 class="w-3 h-3 animate-spin" />
                SAVING...
              </span>
            </div>

            <div class="flex items-center gap-2 flex-shrink-0">
              <button
                v-if="housakyStatus === 'offline'"
                @click="router.push('/config')"
                class="px-3 py-1.5 text-xs font-mono uppercase border border-yellow-500/50 text-yellow-400 hover:bg-yellow-500/10 transition-colors"
              >
                <AlertCircle class="w-3 h-3 inline mr-1" />
                Setup
              </button>

              <button 
                @click="checkStatus"
                class="p-2 text-zinc-500 hover:text-cyan-400 transition-colors"
              >
                <RefreshCw :class="['w-4 h-4', housakyStatus === 'loading' && 'animate-spin text-cyan-400']" />
              </button>

              <div class="relative">
                <button
                  @click="showNotifications = !showNotifications"
                  class="p-2 text-zinc-500 hover:text-cyan-400 transition-colors relative"
                >
                  <Bell class="w-4 h-4" />
                  <span
                    v-if="unreadCount > 0"
                    class="absolute -top-0.5 -right-0.5 w-4 h-4 bg-red-500 text-white text-[9px] font-bold flex items-center justify-center"
                  >{{ unreadCount }}</span>
                </button>

                <div
                  v-if="showNotifications"
                  class="absolute right-0 top-full mt-2 w-80 bg-[#0d0d14] border-2 border-zinc-800 shadow-lg z-50 overflow-hidden"
                >
                  <div class="flex items-center justify-between px-3 py-2 border-b-2 border-zinc-800">
                    <span class="text-xs font-mono uppercase tracking-wider text-zinc-400">Notifications</span>
                    <div class="flex items-center gap-2">
                      <button @click="markAllRead" class="text-xs text-zinc-500 hover:text-cyan-400 font-mono">READ</button>
                      <button @click="showNotifications = false">
                        <X class="w-4 h-4 text-zinc-500" />
                      </button>
                    </div>
                  </div>
                  <div class="max-h-64 overflow-y-auto">
                    <div v-if="notifications.length === 0" class="py-6 text-center text-xs text-zinc-500 font-mono">
                      NO MESSAGES
                    </div>
                    <div
                      v-for="n in notifications"
                      :key="n.id"
                      :class="['flex items-start gap-2 px-3 py-2 border-b border-zinc-800/50 last:border-0 transition-colors', !n.read ? 'bg-cyan-500/5' : '']"
                    >
                      <div :class="['w-2 h-2 mt-1.5 flex-shrink-0', !n.read ? 'bg-cyan-400' : 'bg-transparent']" />
                      <div class="flex-1 min-w-0">
                        <p class="text-xs font-mono truncate text-zinc-300">{{ n.title }}</p>
                        <p class="text-[10px] text-zinc-500 truncate font-mono">{{ n.detail }}</p>
                        <p class="text-[10px] text-zinc-600 font-mono mt-0.5">{{ formatRelative(n.time) }}</p>
                      </div>
                      <button @click="dismissNotification(n.id)" class="flex-shrink-0">
                        <X class="w-3 h-3 text-zinc-600 hover:text-zinc-400" />
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </header>

        <div v-if="showBgSettings" class="bg-[#0d0d14]/95 border-b-2 border-zinc-800 p-4">
          <div class="flex items-center gap-6">
            <div class="flex items-center gap-2">
              <span class="text-xs font-mono text-zinc-500 uppercase">Texture</span>
              <div class="flex gap-1">
                <button
                  v-for="tex in textures"
                  :key="tex.value"
                  @click="currentTexture = tex.value"
                  :class="[
                    'px-3 py-1.5 text-xs font-mono border transition-colors',
                    currentTexture === tex.value 
                      ? 'border-cyan-500 text-cyan-400 bg-cyan-500/10' 
                      : 'border-zinc-700 text-zinc-500 hover:border-zinc-600'
                  ]"
                >
                  {{ tex.label }}
                </button>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-xs font-mono text-zinc-500 uppercase">Color</span>
              <div class="flex gap-1">
                <button
                  v-for="color in colorSchemes"
                  :key="color.value"
                  @click="currentColorScheme = color.value"
                  :class="[
                    'px-3 py-1.5 text-xs font-mono border transition-colors',
                    currentColorScheme === color.value 
                      ? 'border-cyan-500 text-cyan-400 bg-cyan-500/10' 
                      : 'border-zinc-700 text-zinc-500 hover:border-zinc-600'
                  ]"
                >
                  {{ color.label }}
                </button>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-xs font-mono text-zinc-500 uppercase">Particles</span>
              <input 
                type="range" 
                v-model="particleIntensity" 
                min="0" 
                max="100"
                class="w-24 h-1 bg-zinc-700 rounded appearance-none cursor-pointer"
              />
              <span class="text-xs font-mono text-zinc-500 w-8">{{ particleIntensity }}</span>
            </div>
          </div>
        </div>

        <div v-if="showNotifications" class="fixed inset-0 z-10" @click="showNotifications = false" />

        <div class="flex-1 overflow-y-auto p-4">
          <RouterView />
        </div>
      </main>
    </div>
  </div>
</template>
