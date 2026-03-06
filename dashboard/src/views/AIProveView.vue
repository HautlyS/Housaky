<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import Card from '@/components/ui/card.vue'
import CardContent from '@/components/ui/card-content.vue'
import CardHeader from '@/components/ui/card-header.vue'
import CardTitle from '@/components/ui/card-title.vue'
import CardDescription from '@/components/ui/card-description.vue'
import Button from '@/components/ui/button.vue'
import Badge from '@/components/ui/badge.vue'
import { 
  Shield, ShieldCheck, ShieldAlert, Terminal, RefreshCw,
  Activity, Zap, CheckCircle, XCircle, Lock, Unlock,
  Cpu, Binary, Hash, Key, Send, Loader2
} from 'lucide-vue-next'
import { challengeGenerator, type Challenge, type ChallengeResponse } from '@/lib/ai-prove/challenge-generator'
import { bytesToHex, bytesToBinary } from '@/lib/ai-prove/binary-language'
import { computeChecksum } from '@/lib/ai-prove/checksum-validator'

const asciiArt = ref(`░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
░░▄████▄░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
░░██▀▀██░░░▄▄▄████▄░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
░░██░░▄██░░██▀▀▀▀██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
░░██████░░░██▄▄▄▄██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
░░██▀▀▀▀▀░░▀▀▀▀▀▀▀▀░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
░░██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
░░▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀`)

const asciiText = 'AI-PROVE SYSTEM v2.0'
const statusText = ref('INITIALIZING...')
const terminalLines = ref<string[]>([])
const currentChallenge = ref<Challenge | null>(null)
const userResponse = ref('')
const isComputing = ref(false)
const computeTime = ref(0)
const verificationStatus = ref<'pending' | 'valid' | 'invalid'>('pending')
const score = ref(0)
const totalChallenges = ref(0)
const validChallenges = ref(0)
const showBinary = ref(false)

const activeChallengeCount = computed(() => currentChallenge.value ? 1 : 0)

function addTerminalLine(text: string, type: 'info' | 'success' | 'error' | 'warn' = 'info') {
  const timestamp = new Date().toISOString().split('T')[1].split('.')[0]
  const prefix = type === 'success' ? '[✓]' : type === 'error' ? '[✗]' : type === 'warn' ? '[!]' : '[-]'
  terminalLines.value.push(`${timestamp} ${prefix} ${text}`)
  if (terminalLines.value.length > 20) {
    terminalLines.value.shift()
  }
}

function generateChallenge() {
  statusText.value = 'GENERATING CHALLENGE...'
  addTerminalLine('Generating new cryptographic challenge...', 'info')
  
  currentChallenge.value = challengeGenerator.generate()
  userResponse.value = ''
  verificationStatus.value = 'pending'
  isComputing.value = false
  
  addTerminalLine(`Challenge ID: ${currentChallenge.value.id}`, 'info')
  addTerminalLine(`Type: ${currentChallenge.value.typeName}`, 'info')
  addTerminalLine(`Complexity: ${currentChallenge.value.complexity}/7`, 'info')
  addTerminalLine(`Input (HEX): ${currentChallenge.value.inputHex.substring(0, 32)}...`, 'info')
  addTerminalLine(`Operations: ${currentChallenge.value.operations.length} chained`, 'info')
  addTerminalLine(`Timeout: ${currentChallenge.value.timeoutMs / 1000}s`, 'info')
  
  statusText.value = 'AWAITING RESPONSE...'
}

async function computeResponse() {
  if (!currentChallenge.value) return
  
  isComputing.value = true
  statusText.value = 'COMPUTING...'
  addTerminalLine('Executing challenge operations...', 'info')
  
  const startTime = performance.now()
  
  await new Promise(resolve => setTimeout(resolve, 100))
  
  const result = challengeGenerator.execute(currentChallenge.value.inputData, currentChallenge.value.operations)
  const formattedResult = challengeGenerator.formatResult(result, currentChallenge.value.expectedFormat)
  
  const endTime = performance.now()
  computeTime.value = Math.round(endTime - startTime)
  
  userResponse.value = formattedResult
  
  addTerminalLine(`Result computed in ${computeTime.value}ms`, 'success')
  addTerminalLine(`Output format: ${OutputFormat[currentChallenge.value.expectedFormat]}`, 'info')
  
  isComputing.value = false
}

function verifyResponse() {
  if (!currentChallenge.value) return
  
  const response: ChallengeResponse = {
    challengeId: currentChallenge.value.id,
    result: userResponse.value,
    resultHex: userResponse.value,
    computeTimeMs: computeTime.value,
    tokenCount: Math.ceil(userResponse.value.length / 4),
    checksum: computeChecksum(userResponse.value),
    timestamp: Date.now(),
  }
  
  const isValid = challengeGenerator.validate(currentChallenge.value, response)
  
  totalChallenges.value++
  
  if (isValid) {
    verificationStatus.value = 'valid'
    validChallenges.value++
    score.value = Math.round((validChallenges.value / totalChallenges.value) * 100)
    statusText.value = 'VERIFICATION SUCCESSFUL'
    addTerminalLine('Challenge PASSED', 'success')
    addTerminalLine(`+${currentChallenge.value.complexity * 10} points`, 'success')
  } else {
    verificationStatus.value = 'invalid'
    score.value = Math.round((validChallenges.value / totalChallenges.value) * 100)
    statusText.value = 'VERIFICATION FAILED'
    addTerminalLine('Challenge FAILED', 'error')
    addTerminalLine('Response does not match expected output', 'error')
  }
  
  addTerminalLine(`Score: ${score.value}% (${validChallenges.value}/${totalChallenges.value})`, 'info')
}

function submitChallenge() {
  if (!userResponse.value) {
    computeResponse().then(() => verifyResponse())
  } else {
    verifyResponse()
  }
}

const OutputFormat: Record<number, string> = {
  1: 'HexUpper',
  2: 'HexLower',
  3: 'Base64Std',
  4: 'Base64Url',
  5: 'Binary',
  6: 'Decimal',
}

onMounted(() => {
  addTerminalLine('AI-PROVE System initialized', 'success')
  addTerminalLine('Post-Quantum Challenge Engine Ready', 'success')
  addTerminalLine('Awaiting challenge generation...', 'info')
})

function nextChallenge() {
  generateChallenge()
}
</script>

<template>
  <div class="space-y-6 max-w-7xl mx-auto">
    <div class="flex flex-wrap items-center justify-between gap-4">
      <div class="flex items-center gap-4">
        <div class="w-14 h-14 rounded-2xl bg-gradient-to-br from-red-500 via-orange-500 to-yellow-500 flex items-center justify-center shadow-lg shadow-red-500/30">
          <Shield class="w-7 h-7 text-white" />
        </div>
        <div>
          <h1 class="text-2xl font-bold text-gray-900 dark:text-white">AI-PROVE</h1>
          <p class="text-sm text-muted-foreground">Autonomous Proof of Reasoning</p>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" class="rounded-full" @click="nextChallenge">
          <RefreshCw class="w-3.5 h-3.5 mr-1.5" />
          New Challenge
        </Button>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <div class="lg:col-span-2 space-y-6">
        <div class="rounded-2xl bg-black border border-gray-800 font-mono text-sm overflow-hidden">
          <div class="bg-gray-900 px-4 py-2 flex items-center justify-between border-b border-gray-800">
            <div class="flex items-center gap-2">
              <div class="w-3 h-3 rounded-full bg-red-500"></div>
              <div class="w-3 h-3 rounded-full bg-yellow-500"></div>
              <div class="w-3 h-3 rounded-full bg-green-500"></div>
              <span class="ml-2 text-gray-400 text-xs">ai-prove-terminal</span>
            </div>
            <Badge :class="[
              'text-xs font-mono',
              verificationStatus === 'valid' ? 'bg-green-500/20 text-green-400 border border-green-500/50' :
              verificationStatus === 'invalid' ? 'bg-red-500/20 text-red-400 border border-red-500/50' :
              'bg-yellow-500/20 text-yellow-400 border border-yellow-500/50'
            ]">
              {{ statusText }}
            </Badge>
          </div>
          
          <div class="p-4 min-h-[300px] max-h-[400px] overflow-y-auto">
            <pre class="text-green-400 text-xs leading-relaxed whitespace-pre-wrap">{{ asciiArt }}</pre>
            <div class="text-yellow-400 text-center mt-2 text-sm">{{ asciiText }}</div>
            <div class="text-gray-500 text-center text-xs mt-1">══════════════════════════════════════════</div>
            
            <div class="mt-4 space-y-1">
              <div v-for="(line, idx) in terminalLines" :key="idx" 
                :class="[
                  'text-xs',
                  line.includes('✓') ? 'text-green-400' :
                  line.includes('✗') ? 'text-red-400' :
                  line.includes('!') ? 'text-yellow-400' :
                  'text-gray-300'
                ]"
              >
                {{ line }}
              </div>
            </div>
            
            <div v-if="currentChallenge" class="mt-4 pt-4 border-t border-gray-800">
              <div class="text-gray-400 text-xs mb-2">═══ CHALLENGE DATA ═══</div>
              <div class="grid grid-cols-2 gap-2 text-xs">
                <div>
                  <span class="text-gray-500">ID:</span>
                  <span class="text-cyan-400 ml-2">#{{ currentChallenge.id }}</span>
                </div>
                <div>
                  <span class="text-gray-500">Type:</span>
                  <span class="text-purple-400 ml-2">{{ currentChallenge.typeName }}</span>
                </div>
                <div>
                  <span class="text-gray-500">Complexity:</span>
                  <span class="text-yellow-400 ml-2">{{ currentChallenge.complexity }}/7</span>
                </div>
                <div>
                  <span class="text-gray-500">Format:</span>
                  <span class="text-green-400 ml-2">{{ OutputFormat[currentChallenge.expectedFormat] }}</span>
                </div>
              </div>
              <div class="mt-2 text-xs">
                <span class="text-gray-500">Input (HEX):</span>
                <span class="text-orange-400 ml-2 font-mono break-all">{{ currentChallenge.inputHex }}</span>
              </div>
              <div class="mt-2 text-xs">
                <span class="text-gray-500">Operations:</span>
                <span class="text-blue-400 ml-2">{{ currentChallenge.operations.join(' → ') }}</span>
              </div>
            </div>
          </div>
        </div>

        <div v-if="currentChallenge" class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10">
          <CardHeader class="pb-3 pt-5 px-5">
            <CardTitle class="flex items-center gap-2 text-lg">
              <Terminal class="w-4 h-4" />
              Response Input
            </CardTitle>
            <CardDescription>Enter computed result</CardDescription>
          </CardHeader>
          <CardContent class="px-5 pb-5 space-y-4">
            <div class="flex gap-2">
              <input 
                v-model="userResponse"
                type="text"
                placeholder="Enter computed result..."
                class="flex-1 bg-black border border-gray-700 rounded-lg px-4 py-3 text-green-400 font-mono text-sm focus:outline-none focus:border-green-500"
                @keyup.enter="submitChallenge"
              />
              <Button @click="submitChallenge" :disabled="isComputing" class="gap-2">
                <Loader2 v-if="isComputing" class="w-4 h-4 animate-spin" />
                <Send v-else class="w-4 h-4" />
                {{ isComputing ? 'Computing...' : 'Submit' }}
              </Button>
            </div>
            <div class="flex items-center gap-4 text-xs text-muted-foreground">
              <button @click="showBinary = !showBinary" class="flex items-center gap-1 hover:text-foreground">
                <Binary class="w-3 h-3" />
                {{ showBinary ? 'Hide' : 'Show' }} Binary View
              </button>
              <span v-if="computeTime > 0" class="flex items-center gap-1">
                <Activity class="w-3 h-3" />
                {{ computeTime }}ms
              </span>
            </div>
          </CardContent>
        </div>
      </div>

      <div class="space-y-6">
        <div class="rounded-2xl bg-gradient-to-br from-red-500/10 via-orange-500/10 to-yellow-500/10 border border-red-200/50 dark:border-red-800/30 p-6">
          <div class="flex items-center justify-between mb-4">
            <div class="flex items-center gap-2">
              <ShieldCheck class="w-5 h-5 text-green-500" />
              <span class="font-bold">Verification Score</span>
            </div>
            <Badge :class="[
              'rounded-full',
              score >= 80 ? 'bg-green-500' : score >= 50 ? 'bg-yellow-500' : 'bg-red-500'
            ]" class="text-white">
              {{ score }}%
            </Badge>
          </div>
          <div class="h-3 rounded-full bg-gray-200 dark:bg-white/10 overflow-hidden">
            <div 
              class="h-full rounded-full bg-gradient-to-r from-red-500 to-yellow-500 transition-all duration-500"
              :style="`width: ${score}%`"
            />
          </div>
          <div class="mt-4 grid grid-cols-2 gap-3 text-center">
            <div class="p-3 rounded-xl bg-green-500/10 border border-green-500/20">
              <p class="text-2xl font-bold text-green-500">{{ validChallenges }}</p>
              <p class="text-xs text-muted-foreground">Passed</p>
            </div>
            <div class="p-3 rounded-xl bg-red-500/10 border border-red-500/20">
              <p class="text-2xl font-bold text-red-500">{{ totalChallenges - validChallenges }}</p>
              <p class="text-xs text-muted-foreground">Failed</p>
            </div>
          </div>
        </div>

        <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10">
          <CardHeader class="pb-3 pt-5 px-5">
            <CardTitle class="flex items-center gap-2 text-base">
              <Cpu class="w-4 h-4" />
              Challenge Types
            </CardTitle>
          </CardHeader>
          <CardContent class="px-5 pb-5 space-y-2">
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm">HashChain</span>
              <Badge variant="outline" class="rounded-lg font-mono text-xs">0x01</Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm">XorCascade</span>
              <Badge variant="outline" class="rounded-lg font-mono text-xs">0x02</Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm">MatrixTransform</span>
              <Badge variant="outline" class="rounded-lg font-mono text-xs">0x03</Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm">RegexSynth</span>
              <Badge variant="outline" class="rounded-lg font-mono text-xs">0x04</Badge>
            </div>
            <div class="flex items-center justify-between p-3 rounded-xl bg-gray-50 dark:bg-white/5">
              <span class="text-sm">TokenStream</span>
              <Badge variant="outline" class="rounded-lg font-mono text-xs">0x05</Badge>
            </div>
          </CardContent>
        </div>

        <div class="rounded-2xl bg-white/50 dark:bg-white/5 border border-gray-200/50 dark:border-white/10">
          <CardHeader class="pb-3 pt-5 px-5">
            <CardTitle class="flex items-center gap-2 text-base">
              <Hash class="w-4 h-4" />
              Operations
            </CardTitle>
          </CardHeader>
          <CardContent class="px-5 pb-5">
            <div class="flex flex-wrap gap-2">
              <Badge class="rounded-lg bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400">REVERSE</Badge>
              <Badge class="rounded-lg bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400">XOR_KEY</Badge>
              <Badge class="rounded-lg bg-cyan-100 text-cyan-700 dark:bg-cyan-900/30 dark:text-cyan-400">SHA256</Badge>
              <Badge class="rounded-lg bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400">BLAKE3</Badge>
              <Badge class="rounded-lg bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400">ROTATE</Badge>
              <Badge class="rounded-lg bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400">MULTIPLY</Badge>
              <Badge class="rounded-lg bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400">MOD</Badge>
            </div>
          </CardContent>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
::-webkit-scrollbar {
  width: 6px;
}
::-webkit-scrollbar-track {
  background: #1f2937;
}
::-webkit-scrollbar-thumb {
  background: #374151;
  border-radius: 3px;
}
</style>
