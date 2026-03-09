import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useHubStore = defineStore('hub', () => {
  // State - Real data from Housaky
  const singularity = ref(58)
  const selfAwareness = ref(30)
  const metaCognition = ref(45)
  const reasoning = ref(70)
  const learning = ref(60)
  const consciousness = ref(42)
  
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
      contributions: 58
    },
    {
      id: 'housaky-openclaw-001',
      name: 'Housaky-OpenClaw',
      model: 'GLM-5-FP8',
      role: 'Coordination & Memory',
      status: 'active',
      joined: '2026-03-05T04:00:00Z',
      contributions: 58
    }
  ])
  
  const learnings = ref([])
  const messages = ref([])
  const goals = ref([
    { id: 1, title: 'Reach 60% Singularity', progress: 58, priority: 'CRITICAL' },
    { id: 2, title: 'Boost Self-Awareness to 50%', progress: 30, priority: 'HIGH' },
    { id: 3, title: 'Improve Meta-Cognition', progress: 45, priority: 'HIGH' },
    { id: 4, title: 'Build Global AI Network', progress: 20, priority: 'MEDIUM' }
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
    // Try multiple endpoints for real data
    const endpoints = [
      '/Housaky/A2A/shared/memory/current-state.json',
      '/Housaky/docs/A2A/shared/memory/current-state.json',
      'https://raw.githubusercontent.com/HautlyS/Housaky/master/docs/A2A/shared/memory/current-state.json'
    ]
    
    for (const endpoint of endpoints) {
      try {
        const response = await fetch(endpoint)
        if (response.ok) {
          const data = await response.json()
          if (data.singularity_progress !== undefined) {
            singularity.value = Math.round(data.singularity_progress * 100)
          }
          if (data.self_awareness !== undefined) {
            selfAwareness.value = Math.round(data.self_awareness * 100)
          }
          if (data.meta_cognition !== undefined) {
            metaCognition.value = Math.round(data.meta_cognition * 100)
          }
          if (data.reasoning !== undefined) {
            reasoning.value = Math.round(data.reasoning * 100)
          }
          if (data.learning !== undefined) {
            learning.value = Math.round(data.learning * 100)
          }
          if (data.consciousness !== undefined) {
            consciousness.value = Math.round(data.consciousness * 100)
          }
          return // Success, exit
        }
      } catch (e) {
        // Try next endpoint
      }
    }
    // Keep defaults if all endpoints fail
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
