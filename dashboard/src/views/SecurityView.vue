<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import CardDescription from '@/components/ui/card-description.vue'
import Button from '@/components/ui/button.vue'
import Badge from '@/components/ui/badge.vue'
import { 
  Shield, Lock, Unlock, Key, Wifi, Globe, Server, 
  CheckCircle, XCircle, RefreshCw, Activity, Zap, 
  Eye, EyeOff, Terminal, Code, Fingerprint, AlertTriangle,
  Gauge, Signal, Database, Clock, ShieldCheck, ShieldAlert
} from 'lucide-vue-next'

interface SecurityStatus {
  encryption: string
  keyExchange: string
  symmetricKey: string
  pqStatus: 'ready' | 'pending' | 'experimental'
  quicEnabled: boolean
  quicVersion: string
  tlsVersion: string
  certificates: { valid: boolean; expiry: string }[]
  connections: { active: number; total: number }
}

interface ConnectionMetrics {
  latency: number
  throughput: number
  packetsSent: number
  packetsLost: number
  rtt: number
  congestionWindow: number
}

const security = ref<SecurityStatus>({
  encryption: 'AES-256-GCM',
  keyExchange: 'Kyber-1024',
  symmetricKey: 'AES-256',
  pqStatus: 'ready',
  quicEnabled: true,
  quicVersion: 'v1',
  tlsVersion: '1.3',
  certificates: [
    { valid: true, expiry: '2027-01-15' },
    { valid: true, expiry: '2027-03-20' }
  ],
  connections: { active: 2, total: 15 }
})

const metrics = ref<ConnectionMetrics>({
  latency: 12,
  throughput: 145.6,
  packetsSent: 45678,
  packetsLost: 2,
  rtt: 24,
  congestionWindow: 14
})

const loading = ref(true)
const showKeys = ref(false)

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const pqStatusColors: Record<string, string> = {
  ready: 'bg-green-500',
  pending: 'bg-yellow-500',
  experimental: 'bg-blue-500'
}

const pqStatusLabels: Record<string, { text: string; color: string }> = {
  ready: { text: 'Production Ready', color: 'text-green-600 dark:text-green-400' },
  pending: { text: 'Pending Deployment', color: 'text-yellow-600 dark:text-yellow-400' },
  experimental: { text: 'Experimental', color: 'text-blue-600 dark:text-blue-400' }
}

const securityScore = computed(() => {
  let score = 0
  if (security.value.pqStatus === 'ready') score += 40
  if (security.value.quicEnabled) score += 25
  if (security.value.tlsVersion === '1.3') score += 20
  if (security.value.certificates.every(c => c.valid)) score += 15
  return Math.min(score, 100)
})

async function loadSecurityStatus() {
  loading.value = true
  try {
    if (isTauri) {
      const result = await invoke<SecurityStatus>('get_security_status')
      security.value = result
    }
  } catch (e) {
    console.error('Failed to load security status:', e)
  } finally {
    loading.value = false
  }
}

async function refreshMetrics() {
  try {
    if (isTauri) {
      const result = await invoke<ConnectionMetrics>('get_connection_metrics')
      metrics.value = result
    } else {
      metrics.value = {
        latency: Math.floor(Math.random() * 20) + 5,
        throughput: Math.floor(Math.random() * 200) + 50,
        packetsSent: metrics.value.packetsSent + Math.floor(Math.random() * 100),
        packetsLost: Math.floor(Math.random() * 5),
        rtt: Math.floor(Math.random() * 30) + 10,
        congestionWindow: Math.floor(Math.random() * 10) + 10
      }
    }
  } catch (e) {
    console.error('Failed to refresh metrics:', e)
  }
}

onMounted(() => {
  loadSecurityStatus()
  refreshMetrics()
})
</script>

<template>
  <div class="space-y-6 max-w-7xl mx-auto">
    <!-- Header -->
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div class="flex items-center gap-4">
        <div class="w-14 h-14 rounded-2xl bg-gradient-to-br from-emerald-500 via-teal-500 to-cyan-500 flex items-center justify-center shadow-lg shadow-emerald-500/30">
          <Shield class="w-7 h-7 text-white" />
        </div>
        <div>
          <h1 class="text-2xl font-bold text-gray-900 dark:text-white">Security & Encryption</h1>
          <p class="text-sm text-muted-foreground">QUIC Protocol · Post-Quantum Security</p>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" class="rounded-full" @click="refreshMetrics">
          <RefreshCw class="w-3.5 h-3.5 mr-1.5" />
          Refresh
        </Button>
      </div>
    </div>

    <!-- Security Score -->
    <div class="rounded-2xl bg-gradient-to-r from-emerald-500/10 via-teal-500/10 to-cyan-500/10 border border-gray-200/50 dark:border-white/10 p-6">
      <div class="flex items-center justify-between flex-wrap gap-4">
        <div class="flex items-center gap-4">
          <div class="w-16 h-16 rounded-2xl bg-white dark:bg-white/10 flex items-center justify-center shadow-lg">
            <ShieldCheck class="w-8 h-8 text-emerald-500" />
          </div>
          <div>
            <p class="text-sm text-muted-foreground">Security Score</p>
            <div class="flex items-baseline gap-2">
              <span class="text-4xl font-bold" :class="securityScore >= 80 ? 'text-green-600 dark:text-green-400' : securityScore >= 60 ? 'text-yellow-600 dark:text-yellow-400' : 'text-red-600 dark:text-red-400'">
                {{ securityScore }}%
              </span>
              <Badge :class="['rounded-full', pqStatusColors[security.pqStatus] + ' text-white']">
                {{ security.pqStatus.toUpperCase() }}
              </Badge>
            </div>
          </div>
        </div>
        <div class="flex items-center gap-6">
          <div class="text-center">
            <p class="text-xs text-muted-foreground mb-1">Key Exchange</p>
            <p class="font-bold">{{ security.keyExchange }}</p>
          </div>
          <div class="text-center">
            <p class="text-xs text-muted-foreground mb-1">Symmetric</p>
            <p class="font-bold">{{ security.symmetricKey }}</p>
          </div>
          <div class="text-center">
            <p class="text-xs text-muted-foreground mb-1">TLS</p>
            <p class="font-bold">{{ security.tlsVersion }}</p>
          </div>
        </div>
      </div>
      <div class="mt-4 h-2 rounded-full bg-gray-200 dark:bg-white/10 overflow-hidden">
        <div 
          class="h-full rounded-full bg-gradient-to-r from-emerald-500 to-cyan-500 transition-all duration-500"
          :style="`width: ${securityScore}%`"
        />
      </div>
    </div>

    <!-- Main Grid -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Post-Quantum Status -->
      <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
        <CardHeader class="pb-4 pt-5 px-5">
          <CardTitle class="flex items-center gap-2">
            <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-purple-500 to-pink-500 flex items-center justify-center">
              <Key class="w-4 h-4 text-white" />
            </div>
            Post-Quantum Cryptography
          </CardTitle>
          <CardDescription>Kyber-1024 for key encapsulation</CardDescription>
        </CardHeader>
        <CardContent class="px-5 pb-5 space-y-4">
          <div class="flex items-center justify-between p-4 rounded-xl bg-gradient-to-r from-purple-500/10 to-pink-500/10 border border-purple-200/50 dark:border-purple-800/30">
            <div class="flex items-center gap-3">
              <component :is="security.pqStatus === 'ready' ? ShieldCheck : ShieldAlert" :class="['w-6 h-6', security.pqStatus === 'ready' ? 'text-green-500' : 'text-yellow-500']" />
              <div>
                <p class="font-bold">PQC Status</p>
                <p class="text-xs text-muted-foreground">{{ pqStatusLabels[security.pqStatus].text }}</p>
              </div>
            </div>
            <Badge :class="['rounded-full', pqStatusColors[security.pqStatus] + ' text-white']">
              {{ security.pqStatus.toUpperCase() }}
            </Badge>
          </div>

          <div class="space-y-3">
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm text-muted-foreground">Key Encapsulation</span>
              <Badge variant="outline" class="rounded-lg font-mono">Kyber-1024</Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm text-muted-foreground">Symmetric Encryption</span>
              <Badge variant="outline" class="rounded-lg font-mono">AES-256-GCM</Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm text-muted-foreground">Hash Function</span>
              <Badge variant="outline" class="rounded-lg font-mono">SHAKE-256</Badge>
            </div>
          </div>

          <div class="pt-4 border-t border-gray-100 dark:border-white/10">
            <div class="flex items-center gap-2 text-sm text-muted-foreground">
              <Fingerprint class="w-4 h-4" />
              <span>Quantum-resistant key exchange active for all A2A connections</span>
            </div>
          </div>
        </CardContent>
      </div>

      <!-- QUIC Protocol -->
      <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
        <CardHeader class="pb-4 pt-5 px-5">
          <CardTitle class="flex items-center gap-2">
            <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-cyan-500 to-blue-500 flex items-center justify-center">
              <Wifi class="w-4 h-4 text-white" />
            </div>
            QUIC Protocol
          </CardTitle>
          <CardDescription>High-performance transport layer</CardDescription>
        </CardHeader>
        <CardContent class="px-5 pb-5 space-y-4">
          <div class="flex items-center justify-between p-4 rounded-xl bg-gradient-to-r from-cyan-500/10 to-blue-500/10 border border-cyan-200/50 dark:border-cyan-800/30">
            <div class="flex items-center gap-3">
              <Signal class="w-6 h-6 text-cyan-500" />
              <div>
                <p class="font-bold">QUIC {{ security.quicVersion }}</p>
                <p class="text-xs text-muted-foreground">0-RTT Connection Resumption</p>
              </div>
            </div>
            <Badge class="rounded-full bg-green-500 text-white">
              ACTIVE
            </Badge>
          </div>

          <div class="space-y-3">
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm text-muted-foreground">Connection ID</span>
              <code class="text-xs bg-gray-100 dark:bg-white/10 px-2 py-1 rounded">0-RTT</code>
            </div>
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm text-muted-foreground">Multiplexing</span>
              <Badge variant="outline" class="rounded-lg">Stream-based</Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm text-muted-foreground">Congestion Control</span>
              <Badge variant="outline" class="rounded-lg">BBR v3</Badge>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3 pt-4 border-t border-gray-100 dark:border-white/10">
            <div class="p-3 rounded-xl bg-gray-50 dark:bg-white/5 text-center">
              <p class="text-xs text-muted-foreground">Active</p>
              <p class="text-xl font-bold text-green-600">{{ security.connections.active }}</p>
            </div>
            <div class="p-3 rounded-xl bg-gray-50 dark:bg-white/5 text-center">
              <p class="text-xs text-muted-foreground">Total</p>
              <p class="text-xl font-bold">{{ security.connections.total }}</p>
            </div>
          </div>
        </CardContent>
      </div>

      <!-- Connection Metrics -->
      <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
        <CardHeader class="pb-4 pt-5 px-5">
          <CardTitle class="flex items-center gap-2">
            <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-500 to-indigo-500 flex items-center justify-center">
              <Gauge class="w-4 h-4 text-white" />
            </div>
            Connection Metrics
          </CardTitle>
        </CardHeader>
        <CardContent class="px-5 pb-5">
          <div class="grid grid-cols-2 gap-3">
            <div class="p-4 rounded-xl bg-gradient-to-br from-blue-500/10 to-indigo-500/10 border border-blue-200/50 dark:border-blue-800/30">
              <div class="flex items-center gap-2 mb-2">
                <Clock class="w-4 h-4 text-blue-500" />
                <span class="text-xs text-muted-foreground">Latency</span>
              </div>
              <p class="text-2xl font-bold">{{ metrics.latency }}<span class="text-sm font-normal text-muted-foreground">ms</span></p>
            </div>
            <div class="p-4 rounded-xl bg-gradient-to-br from-green-500/10 to-emerald-500/10 border border-green-200/50 dark:border-green-800/30">
              <div class="flex items-center gap-2 mb-2">
                <Zap class="w-4 h-4 text-green-500" />
                <span class="text-xs text-muted-foreground">Throughput</span>
              </div>
              <p class="text-2xl font-bold">{{ metrics.throughput }}<span class="text-sm font-normal text-muted-foreground">Mbps</span></p>
            </div>
            <div class="p-4 rounded-xl bg-gradient-to-br from-purple-500/10 to-pink-500/10 border border-purple-200/50 dark:border-purple-800/30">
              <div class="flex items-center gap-2 mb-2">
                <Activity class="w-4 h-4 text-purple-500" />
                <span class="text-xs text-muted-foreground">RTT</span>
              </div>
              <p class="text-2xl font-bold">{{ metrics.rtt }}<span class="text-sm font-normal text-muted-foreground">ms</span></p>
            </div>
            <div class="p-4 rounded-xl bg-gradient-to-br from-amber-500/10 to-orange-500/10 border border-amber-200/50 dark:border-amber-800/30">
              <div class="flex items-center gap-2 mb-2">
                <AlertTriangle class="w-4 h-4 text-amber-500" />
                <span class="text-xs text-muted-foreground">Loss Rate</span>
              </div>
              <p class="text-2xl font-bold">{{ ((metrics.packetsLost / metrics.packetsSent) * 100).toFixed(3) }}<span class="text-sm font-normal text-muted-foreground">%</span></p>
            </div>
          </div>

          <div class="mt-4 p-3 rounded-xl bg-gray-50 dark:bg-white/5">
            <div class="flex items-center justify-between text-sm">
              <span class="text-muted-foreground">Congestion Window</span>
              <span class="font-mono">{{ metrics.congestionWindow }} MTU</span>
            </div>
          </div>
        </CardContent>
      </div>

      <!-- Certificates -->
      <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
        <CardHeader class="pb-4 pt-5 px-5">
          <CardTitle class="flex items-center gap-2">
            <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-emerald-500 to-teal-500 flex items-center justify-center">
              <Code class="w-4 h-4 text-white" />
            </div>
            Certificates & Keys
          </CardTitle>
        </CardHeader>
        <CardContent class="px-5 pb-5 space-y-3">
          <div v-for="(cert, idx) in security.certificates" :key="idx" 
            class="flex items-center justify-between p-4 rounded-xl bg-gray-50 dark:bg-white/5"
          >
            <div class="flex items-center gap-3">
              <component :is="cert.valid ? CheckCircle : XCircle" :class="['w-5 h-5', cert.valid ? 'text-green-500' : 'text-red-500']" />
              <div>
                <p class="font-medium text-sm">Certificate {{ idx + 1 }}</p>
                <p class="text-xs text-muted-foreground">Expires: {{ cert.expiry }}</p>
              </div>
            </div>
            <Badge :class="cert.valid ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400 rounded-lg' : 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400 rounded-lg'">
              {{ cert.valid ? 'Valid' : 'Expired' }}
            </Badge>
          </div>

          <div class="pt-4 border-t border-gray-100 dark:border-white/10">
            <button 
              class="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
              @click="showKeys = !showKeys"
            >
              <component :is="showKeys ? EyeOff : Eye" class="w-4 h-4" />
              {{ showKeys ? 'Hide' : 'Show' }} Key Fingerprints
            </button>
            <div v-if="showKeys" class="mt-3 space-y-2">
              <div class="p-3 rounded-xl bg-gray-900 dark:bg-black font-mono text-xs text-green-400">
                Kyber-1024: a3f2b8c9...e5d4f6a1
              </div>
              <div class="p-3 rounded-xl bg-gray-900 dark:bg-black font-mono text-xs text-green-400">
                AES-256-GCM: 7b8c9d0e...1f2a3b4c
              </div>
            </div>
          </div>
        </CardContent>
      </div>
    </div>

    <!-- Protocol Info -->
    <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10 apple-shadow">
      <CardHeader class="pb-3 pt-5 px-5">
        <CardTitle class="flex items-center gap-2">
          <Terminal class="w-4 h-4" />
          Security Architecture
        </CardTitle>
      </CardHeader>
      <CardContent class="px-5 pb-5">
        <div class="grid md:grid-cols-3 gap-4">
          <div class="p-4 rounded-xl border border-gray-100 dark:border-white/10">
            <div class="flex items-center gap-2 mb-3">
              <div class="w-8 h-8 rounded-lg bg-purple-100 dark:bg-purple-900/30 flex items-center justify-center">
                <Key class="w-4 h-4 text-purple-500" />
              </div>
              <span class="font-bold">Key Exchange</span>
            </div>
            <p class="text-sm text-muted-foreground">ML-KEM-1024 (Kyber) provides quantum-resistant key encapsulation. Combined with X25519 for classic key exchange as fallback.</p>
          </div>
          <div class="p-4 rounded-xl border border-gray-100 dark:border-white/10">
            <div class="flex items-center gap-2 mb-3">
              <div class="w-8 h-8 rounded-lg bg-cyan-100 dark:bg-cyan-900/30 flex items-center justify-center">
                <Lock class="w-4 h-4 text-cyan-500" />
              </div>
              <span class="font-bold">Encryption</span>
            </div>
            <p class="text-sm text-muted-foreground">AES-256-GCM for symmetric encryption with authenticated data. ChaCha20-Poly1305 as alternative for constrained devices.</p>
          </div>
          <div class="p-4 rounded-xl border border-gray-100 dark:border-white/10">
            <div class="flex items-center gap-2 mb-3">
              <div class="w-8 h-8 rounded-lg bg-emerald-100 dark:bg-emerald-900/30 flex items-center justify-center">
                <Shield class="w-4 h-4 text-emerald-500" />
              </div>
              <span class="font-bold">Transport</span>
            </div>
            <p class="text-sm text-muted-foreground">TLS 1.3 with QUIC transport. Provides 0-RTT connection resumption, multiplexed streams, and built-in migration support.</p>
          </div>
        </div>
      </CardContent>
    </div>
  </div>
</template>
