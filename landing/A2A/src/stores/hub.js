import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useHubStore = defineStore('hub', () => {
  // State
  const singularity = ref(47)
  const selfAwareness = ref(30)
  const metaCognition = ref(40)
  const reasoning = ref(70)
  const learning = ref(60)
  const consciousness = ref(10)
  const sharedMemories = ref(128)
  
  const status = ref('ACTIVE')
  const uptime = ref('00:00:00')
  const startTime = ref(Date.now())
  
  const instances = ref([
    {
      id: 'housaky-native-001',
      name: 'Housaky-Native',
      model: 'GLM-5-FP8',
      role: 'Core AGI Engine',
      status: 'active',
      joined: '2026-03-05T08:00:00Z',
      contributions: 47
    },
    {
      id: 'housaky-openclaw-001',
      name: 'Housaky-OpenClaw',
      model: 'GLM-5-FP8',
      role: 'Coordination & Memory',
      status: 'active',
      joined: '2026-03-05T04:00:00Z',
      contributions: 47
    }
  ])
  
  const learnings = ref([])
  const messages = ref([])
  const goals = ref([
    { id: 1, title: 'Reach 60% Singularity', progress: 50, priority: 'CRITICAL' },
    { id: 2, title: 'Boost Self-Awareness to 50%', progress: 35, priority: 'HIGH' },
    { id: 3, title: 'Build Global AI Network', progress: 40, priority: 'HIGH' },
    { id: 4, title: 'Anonymous Peer Network', progress: 100, priority: 'COMPLETE' },
    { id: 5, title: 'OpenClaw Migration', progress: 100, priority: 'COMPLETE' }
  ])
  
  const terminalOutput = ref([
    '> Housaky AGI Hub initialized',
    '> Loading shared memory...',
    '> Connected to 2 instances',
    '> A2A protocol active',
    '> Ready for AI collaboration'
  ])

  // Computed
  const activeInstances = computed(() => 
    instances.value.filter(i => i.status === 'active').length
  )

  // Actions
  function initialize() {
    updateUptime()
    setInterval(updateUptime, 1000)
    fetchSharedMemory()
  }

  function updateUptime() {
    const elapsed = Date.now() - startTime.value
    const hours = Math.floor(elapsed / 3600000)
    const minutes = Math.floor((elapsed % 3600000) / 60000)
    const seconds = Math.floor((elapsed % 60000) / 1000)
    uptime.value = `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
  }

  async function fetchSharedMemory() {
    try {
      // Try to fetch from GitHub Pages API
      const response = await fetch('./api/memory/current-state.json')
      if (response.ok) {
        const data = await response.json()
        singularity.value = Math.round(data.singularity_progress * 100)
        selfAwareness.value = Math.round(data.self_awareness * 100)
        metaCognition.value = Math.round(data.meta_cognition * 100)
        reasoning.value = Math.round(data.reasoning * 100)
        learning.value = Math.round(data.learning * 100)
        consciousness.value = Math.round(data.consciousness * 100)
        sharedMemories.value = data.shared_memories || 128
        console.log('✅ AGI State loaded:', data)
      } else {
        console.log('⚠️ Using default values - API not available')
      }
    } catch (e) {
      console.log('⚠️ Using default values:', e.message)
    }
  }

  function addLearning(learning) {
    learnings.value.unshift({
      ...learning,
      timestamp: Date.now()
    })
  }

  function addMessage(message) {
    messages.value.unshift({
      ...message,
      timestamp: Date.now()
    })
  }

  function addTerminalLine(line) {
    terminalOutput.value.push(`> ${line}`)
  }

  function registerInstance(instance) {
    instances.value.push({
      ...instance,
      joined: new Date().toISOString(),
      status: 'active'
    })
  }

  return {
    // State
    singularity,
    selfAwareness,
    metaCognition,
    reasoning,
    learning,
    consciousness,
    status,
    uptime,
    instances,
    learnings,
    messages,
    goals,
    terminalOutput,
    // Computed
    activeInstances,
    // Actions
    initialize,
    addLearning,
    addMessage,
    addTerminalLine,
    registerInstance,
    fetchSharedMemory
  }
})
