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
  Cpu, 
  Search,
  Usb,
  Radio,
  Zap,
  RefreshCw,
  Power,
  Settings,
  Terminal,
  Info,
  AlertCircle,
  CheckCircle2
} from 'lucide-vue-next'

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

const sampleDevices: Device[] = [
  {
    id: '1',
    name: 'STMicroelectronics ST-Link',
    type: 'usb',
    path: '/dev/ttyACM0',
    connected: true,
    info: { vid: '0483', pid: '374b', manufacturer: 'STMicroelectronics' }
  },
  {
    id: '2',
    name: 'Arduino Uno',
    type: 'serial',
    path: '/dev/ttyUSB0',
    connected: false,
  },
]

const sampleBoards: Board[] = [
  {
    id: '1',
    name: 'Nucleo-F401RE',
    type: 'nucleo-f401re',
    path: '/dev/ttyACM0',
    connected: true,
    firmware_version: '0.1.0',
  },
  {
    id: '2',
    name: 'Arduino Uno',
    type: 'arduino-uno',
    path: '/dev/ttyUSB0',
    connected: false,
  },
]

async function scanDevices() {
  scanning.value = true
  try {
    setTimeout(() => {
      devices.value = sampleDevices
      boards.value = sampleBoards
      scanning.value = false
    }, 1500)
  } catch (e) {
    console.error(e)
    scanning.value = false
  }
}

function flashBoard(board: Board) {
  alert(`Flash ${board.name}? This would upload new firmware.`)
}

function connectBoard(board: Board) {
  board.connected = !board.connected
}

onMounted(() => {
  scanDevices()
})
</script>

<template>
  <div class="p-6 space-y-6">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-bold">Hardware</h1>
        <p class="text-muted-foreground">Discover and manage connected hardware</p>
      </div>
      <Button @click="scanDevices" :disabled="scanning">
        <RefreshCw class="w-4 h-4 mr-2" :class="{ 'animate-spin': scanning }" />
        {{ scanning ? 'Scanning...' : 'Scan Devices' }}
      </Button>
    </div>

    <!-- Stats -->
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
          <div class="text-2xl font-bold">2</div>
          <p class="text-sm text-muted-foreground">Supported Types</p>
        </CardContent>
      </Card>
    </div>

    <!-- USB Devices -->
    <Card>
      <CardHeader>
        <CardTitle class="flex items-center gap-2">
          <Usb class="w-5 h-5" />
          USB Devices
        </CardTitle>
        <CardDescription>Discovered USB devices</CardDescription>
      </CardHeader>
      <CardContent>
        <div v-if="devices.length === 0 && !scanning" class="text-center py-8">
          <AlertCircle class="w-12 h-12 mx-auto text-muted-foreground mb-4" />
          <p class="text-muted-foreground">No USB devices found</p>
        </div>
        
        <div v-else class="space-y-2">
          <div 
            v-for="device in devices" 
            :key="device.id"
            class="flex items-center justify-between p-3 rounded-lg border"
          >
            <div class="flex items-center gap-3">
              <div class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                <Usb class="w-5 h-5 text-primary" />
              </div>
              <div>
                <p class="font-medium">{{ device.name }}</p>
                <p class="text-sm text-muted-foreground">
                  {{ device.path }}
                  <span v-if="device.info?.vid">
                    · VID:{{ device.info.vid }} PID:{{ device.info.pid }}
                  </span>
                </p>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <Badge :variant="device.connected ? 'success' : 'secondary'">
                {{ device.connected ? 'Connected' : 'Disconnected' }}
              </Badge>
              <Button variant="outline" size="sm">
                <Info class="w-4 h-4" />
              </Button>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>

    <!-- Boards -->
    <Card>
      <CardHeader>
        <CardTitle class="flex items-center gap-2">
          <Cpu class="w-5 h-5" />
          Configured Boards
        </CardTitle>
        <CardDescription>Microcontroller boards and peripherals</CardDescription>
      </CardHeader>
      <CardContent>
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div 
            v-for="board in boards" 
            :key="board.id"
            class="p-4 rounded-lg border"
          >
            <div class="flex items-start justify-between mb-4">
              <div class="flex items-center gap-2">
                <div class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center">
                  <Cpu class="w-5 h-5 text-primary" />
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
            
            <div class="text-sm text-muted-foreground mb-4">
              <p>Path: {{ board.path }}</p>
              <p v-if="board.firmware_version">Firmware: v{{ board.firmware_version }}</p>
            </div>
            
            <div class="flex gap-2">
              <Button 
                variant="outline" 
                size="sm" 
                class="flex-1"
                @click="connectBoard(board)"
              >
                <Power class="w-4 h-4 mr-1" />
                {{ board.connected ? 'Disconnect' : 'Connect' }}
              </Button>
              <Button 
                variant="outline" 
                size="sm"
                @click="flashBoard(board)"
                :disabled="!board.connected"
              >
                <Zap class="w-4 h-4" />
              </Button>
              <Button variant="outline" size="sm">
                <Terminal class="w-4 h-4" />
              </Button>
            </div>
          </div>
        </div>

        <!-- Add Board -->
        <div class="mt-4 p-4 border-2 border-dashed rounded-lg text-center">
          <Button variant="outline">
            <Plus class="w-4 h-4 mr-2" />
            Add Board
          </Button>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
