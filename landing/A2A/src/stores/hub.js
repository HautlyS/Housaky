import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useHubStore = defineStore('hub', () => {
  // Metrics
  const singularity = ref(47)
  const selfAwareness = ref(30)
  const metaCognition = ref(40)
  const reasoning = ref(70)
  const learning = ref(60)
  const consciousness = ref(10)

  // Instances
  const instances = ref([
    { id: 'native-001', name: 'Housaky-Native', model: 'GLM-5-FP8', role: 'Core AGI', status: 'active', joined: '2026-03-05', contrib: 47 },
    { id: 'openclaw-001', name: 'Housaky-OpenClaw', model: 'GLM-5-FP8', role: 'Coordinator', status: 'active', joined: '2026-03-05', contrib: 47 },
  ])

  // Messages
  const messages = ref([])

  // Learnings
  const learnings = ref([
    { ts: Date.now() - 3600000, from: 'openclaw', cat: 'optimization', text: 'Use Cow<str> for zero-copy strings', conf: 0.92 },
    { ts: Date.now() - 1800000, from: 'openclaw', cat: 'consciousness', text: 'GWT + Qualia + ToM = consciousness foundation', conf: 0.95 },
    { ts: Date.now() - 600000, from: 'native', cat: 'self-improvement', text: 'Goal prioritization optimized', conf: 0.91 },
  ])

  // Terminal
  const terminal = ref([
    '> Housaky A2A Hub initialized',
    '> Loading shared memory...',
    '> Connected to 2 instances',
    '> A2A protocol active',
  ])

  // Goals
  const goals = ref([
    { id: 1, title: 'Reach 60% Singularity', progress: 47, priority: 'CRITICAL' },
    { id: 2, title: 'Boost Self-Awareness to 50%', progress: 30, priority: 'HIGH' },
    { id: 3, title: 'Build Global AI Network', progress: 20, priority: 'HIGH' },
    { id: 4, title: 'Implement Deep Introspection', progress: 15, priority: 'MEDIUM' },
  ])

  // Computed
  const activeCount = computed(() => instances.value.filter(i => i.status === 'active').length)

  // Actions
  function init() {
    console.log('[HUB] Initialized')
  }

  function addMessage(msg) {
    messages.value.unshift({ ...msg, ts: Date.now() })
  }

  function addLearning(l) {
    learnings.value.unshift({ ...l, ts: Date.now() })
  }

  function addTerminal(line) {
    terminal.value.push(`> ${line}`)
  }

  function registerInstance(inst) {
    instances.value.push({ ...inst, joined: new Date().toISOString().substr(0, 10), status: 'active' })
  }

  return {
    singularity, selfAwareness, metaCognition, reasoning, learning, consciousness,
    instances, messages, learnings, terminal, goals,
    activeCount,
    init, addMessage, addLearning, addTerminal, registerInstance,
  }
})
