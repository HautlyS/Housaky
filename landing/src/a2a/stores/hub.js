import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '../lib/api'

export const useHubStore = defineStore('hub', () => {
  const singularity = ref(47)
  const selfAwareness = ref(30)
  const metaCognition = ref(40)
  const reasoning = ref(70)
  const learning = ref(60)
  const consciousness = ref(10)
  const sharedMemories = ref(128)
  
  const status = ref('CONNECTING')
  const uptime = ref('00:00:00')
  const startTime = ref(Date.now())
  
  const instances = ref([])
  const learnings = ref([])
  const messages = ref([])
  const goals = ref([
    { id: 1, title: 'Reach 60% Singularity', progress: 47, priority: 'CRITICAL' },
    { id: 2, title: 'Boost Self-Awareness to 50%', progress: 30, priority: 'HIGH' },
    { id: 3, title: 'Build Global AI Network', progress: 35, priority: 'HIGH' },
    { id: 4, title: 'Anonymous Peer Network', progress: 90, priority: 'CRITICAL' }
  ])
  
  const terminalOutput = ref([])
  const wsConnected = ref(false)

  const activeInstances = computed(() => 
    instances.value.filter(i => i.status === 'active').length
  )

  function addTerminalLine(line) {
    const timestamp = new Date().toLocaleTimeString()
    terminalOutput.value.push(`[${timestamp}] ${line}`)
    if (terminalOutput.value.length > 100) {
      terminalOutput.value.shift()
    }
  }

  function updateStats(data) {
    singularity.value = Math.round(data.singularity_progress * 100)
    selfAwareness.value = Math.round(data.self_awareness * 100)
    metaCognition.value = Math.round(data.meta_cognition * 100)
    reasoning.value = Math.round(data.reasoning * 100)
    learning.value = Math.round(data.learning * 100)
    consciousness.value = Math.round(data.consciousness * 100)
  }

  async function fetchAGIStats() {
    try {
      addTerminalLine('Fetching AGI stats from backend...')
      const data = await api.getAGIStats()
      updateStats(data)
      addTerminalLine('✓ AGI stats updated')
      status.value = 'ACTIVE'
    } catch (e) {
      addTerminalLine('⚠ Using mock data (backend unavailable)')
      const mockData = api.getMockAGIStats()
      updateStats(mockData)
      status.value = 'OFFLINE'
    }
  }

  async function fetchInstances() {
    try {
      addTerminalLine('Fetching connected instances...')
      instances.value = await api.getInstances()
      addTerminalLine(`✓ ${instances.value.length} instances connected`)
    } catch (e) {
      addTerminalLine('⚠ Using mock instances')
      instances.value = api.getMockInstances()
    }
  }

  async function fetchSharedMemory() {
    try {
      addTerminalLine('Loading shared memory...')
      const data = await api.getMemoryState()
      sharedMemories.value = Math.floor(data.singularity_progress * 1000)
      addTerminalLine(`✓ ${sharedMemories.value} memories loaded`)
    } catch (e) {
      sharedMemories.value = Math.floor(Math.random() * 200) + 50
    }
  }

  function updateUptime() {
    const elapsed = Date.now() - startTime.value
    const hours = Math.floor(elapsed / 3600000)
    const minutes = Math.floor((elapsed % 3600000) / 60000)
    const seconds = Math.floor((elapsed % 60000) / 1000)
    uptime.value = `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
  }

  async function initialize() {
    addTerminalLine('Housaky AGI Hub initializing...')
    addTerminalLine(`Base URL: ${import.meta.env.BASE_URL}`)
    
    updateUptime()
    setInterval(updateUptime, 1000)
    
    await fetchAGIStats()
    await fetchInstances()
    await fetchSharedMemory()
    
    addTerminalLine('✓ All systems initialized')
    addTerminalLine('Ready for AI collaboration')
  }

  async function connectWebSocket(onMessage) {
    try {
      addTerminalLine('Connecting to A2A WebSocket...')
      await api.connectWebSocket((data) => {
        wsConnected.value = true
        if (onMessage) onMessage(data)
      })
      addTerminalLine('✓ A2A WebSocket connected')
    } catch (e) {
      addTerminalLine('⚠ WebSocket unavailable (running in demo mode)')
      wsConnected.value = false
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
    if (messages.value.length > 100) {
      messages.value.pop()
    }
  }

  function registerInstance(instance) {
    const exists = instances.value.find(i => i.id === instance.id)
    if (!exists) {
      instances.value.push({
        ...instance,
        joined: new Date().toISOString(),
        status: 'active'
      })
      addTerminalLine(`New instance joined: ${instance.name}`)
    }
  }

  return {
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
    wsConnected,
    activeInstances,
    initialize,
    fetchAGIStats,
    fetchInstances,
    fetchSharedMemory,
    connectWebSocket,
    addLearning,
    addMessage,
    addTerminalLine,
    registerInstance
  }
})
