<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardDescription from '@/components/ui/card-description.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import Button from '@/components/ui/button.vue'
import Badge from '@/components/ui/badge.vue'
import { 
  Cpu, 
  Usb,
  Radio,
  Zap,
  RefreshCw,
  Power,
  Settings,
  Terminal,
  Info,
  AlertCircle,
  CheckCircle2,
  Loader2,
  Plus,
  Activity
} from 'lucide-vue-next'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

interface Device {
  id: string
  name: string
  type: 'usb' | 'serial' | 'gpio'
  path: string
  connected: boolean
  info?: {
    vid?: string
    pid?: string
    manufacturer?: string
  }
}

interface Board {
  id: string
  name: string
  type: 'nucleo-f401re' | 'arduino-uno' | 'rpi-gpio' | 'esp32'
  path: string
  connected: boolean
  firmware_version?: string
}

const devices = ref<Device[]>([])
const boards = ref<Board[]>([])
const loading = ref(true)
const scanning = ref(false)
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

async function scanDevices() {
  scanning.value = true
  if (!isTauri) {
    scanning.value = false
    return
  }
  try {
    const discovered = await invoke<Device[]>('hardware_discover')
    if (discovered && discovered.length > 0) {
      devices.value = discovered
    }
  } catch (e) {
    console.error('Hardware scan failed:', e)
  } finally {
    scanning.value = false
  }
}

async function loadHardware() {
  loading.value = true
  try {
    await scanDevices()
  } finally {
    loading.value = false
  }
}

function flashBoard(board: Board) {
  alert(`Flash ${board.name}?\n\nThis would upload new firmware to the board.`)
}

function connectBoard(board: Board) {
  board.connected = !board.connected
}

function viewDeviceInfo(device: Device) {
  const info = device.info
  alert(`Device: ${device.name}\nPath: ${device.path}\n${info?.vid ? `VID: ${info.vid}\nPID: ${info.pid}` : ''}${info?.manufacturer ? `\nManufacturer: ${info.manufacturer}` : ''}`)
}

onMounted(async () => {
  await checkHousaky()
  await loadHardware()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">Hardware</h1>
        <p class="text-sm text-muted-foreground">Discover and manage connected hardware</p>
      </div>
      <Button @click="scanDevices" :disabled="scanning || !housakyInstalled">
        <RefreshCw :class="['w-4 h-4 mr-2', scanning && 'animate-spin']" />
        {{ scanning ? 'Scanning...' : 'Scan Devices' }}
      </Button>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ devices.filter(d => d.connected).length }}</div>
          <p class="text-sm text-muted-foreground">USB Devices</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ boards.filter(b => b.connected).length }}</div>
          <p class="text-sm text-muted-foreground">Boards Connected</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">{{ boards.length }}</div>
          <p class="text-sm text-muted-foreground">Total Boards</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent class="pt-6">
          <div class="text-2xl font-bold">4</div>
          <p class="text-sm text-muted-foreground">Supported Types</p>
        </CardContent>
      </Card>
    </div>

    <div v-if="loading" class="flex items-center justify-center py-12">
      <Loader2 class="w-8 h-8 animate-spin text-muted-foreground" />
    </div>

    <template v-else>
      <Card>
        <CardHeader>
          <CardTitle class="flex items-center gap-2">
            <Usb class="w-5 h-5" />
            USB Devices
          </CardTitle>
          <CardDescription>Discovered USB and serial devices</CardDescription>
        </CardHeader>
        <CardContent>
          <div v-if="devices.length === 0" class="text-center py-8">
            <AlertCircle class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
            <p class="text-muted-foreground mb-2">No USB devices found</p>
            <p class="text-sm text-muted-foreground">Connect a device and click Scan</p>
          </div>
          
          <div v-else class="space-y-2">
            <div 
              v-for="device in devices" 
              :key="device.id"
              class="flex items-center justify-between p-4 rounded-lg border hover:bg-muted/50 transition-colors"
            >
              <div class="flex items-center gap-4">
                <div :class="[
                  'w-10 h-10 rounded-lg flex items-center justify-center',
                  device.connected ? 'bg-green-100 dark:bg-green-900/20' : 'bg-muted'
                ]">
                  <Usb :class="['w-5 h-5', device.connected ? 'text-green-600' : 'text-muted-foreground']" />
                </div>
                <div>
                  <p class="font-medium">{{ device.name }}</p>
                  <p class="text-sm text-muted-foreground">
                    {{ device.path }}
                    <span v-if="device.info?.vid" class="ml-2">
                      VID:{{ device.info.vid }} PID:{{ device.info.pid }}
                    </span>
                  </p>
                </div>
              </div>
              <div class="flex items-center gap-3">
                <Badge :variant="device.connected ? 'success' : 'secondary'">
                  {{ device.connected ? 'Connected' : 'Disconnected' }}
                </Badge>
                <Button variant="outline" size="sm" @click="viewDeviceInfo(device)">
                  <Info class="w-4 h-4" />
                </Button>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <div class="flex items-center justify-between">
            <div>
              <CardTitle class="flex items-center gap-2">
                <Cpu class="w-5 h-5" />
                Configured Boards
              </CardTitle>
              <CardDescription>Microcontroller boards and peripherals</CardDescription>
            </div>
            <Button size="sm" variant="outline">
              <Plus class="w-4 h-4 mr-2" />
              Add Board
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <div v-if="boards.length === 0" class="text-center py-8">
            <Cpu class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
            <p class="text-muted-foreground mb-2">No boards configured</p>
            <p class="text-sm text-muted-foreground">Add a board to get started</p>
          </div>
          
          <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            <div 
              v-for="board in boards" 
              :key="board.id"
              class="p-4 rounded-lg border hover:shadow-md transition-shadow"
            >
              <div class="flex items-start justify-between mb-4">
                <div class="flex items-center gap-3">
                  <div :class="[
                    'w-10 h-10 rounded-lg flex items-center justify-center',
                    board.connected ? 'bg-blue-100 dark:bg-blue-900/20' : 'bg-muted'
                  ]">
                    <Cpu :class="['w-5 h-5', board.connected ? 'text-blue-600' : 'text-muted-foreground']" />
                  </div>
                  <div>
                    <p class="font-medium">{{ board.name }}</p>
                    <p class="text-xs text-muted-foreground capitalize">{{ board.type.replace('-', ' ') }}</p>
                  </div>
                </div>
                <Badge :variant="board.connected ? 'success' : 'secondary'">
                  {{ board.connected ? 'Online' : 'Offline' }}
                </Badge>
              </div>
              
              <div class="space-y-1 text-sm text-muted-foreground mb-4">
                <p class="flex items-center gap-2">
                  <Activity class="w-3 h-3" />
                  Path: {{ board.path }}
                </p>
                <p v-if="board.firmware_version" class="flex items-center gap-2">
                  <Zap class="w-3 h-3" />
                  Firmware: v{{ board.firmware_version }}
                </p>
              </div>
              
              <div class="flex gap-2">
                <Button 
                  variant="outline" 
                  size="sm" 
                  class="flex-1"
                  @click="connectBoard(board)"
                >
                  <Power class="w-3 h-3 mr-1" />
                  {{ board.connected ? 'Disconnect' : 'Connect' }}
                </Button>
                <Button 
                  variant="outline" 
                  size="sm"
                  @click="flashBoard(board)"
                  :disabled="!board.connected"
                >
                  <Zap class="w-3 h-3" />
                </Button>
                <Button variant="outline" size="sm">
                  <Terminal class="w-3 h-3" />
                </Button>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </template>
  </div>
</template>
