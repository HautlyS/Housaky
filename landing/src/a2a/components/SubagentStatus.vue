<template>
  <div class="subagent-status">
    <div class="status-header">
      <span class="title">[ SUBAGENTS ]</span>
      <span
        class="status"
        :class="{ active: allActive }"
      >
        {{ allActive ? '● ALL ONLINE' : '○ PARTIAL' }}
      </span>
    </div>
    
    <div class="agents-grid">
      <div 
        v-for="agent in agents" 
        :key="agent.id" 
        class="agent-card"
        :class="{ active: agent.active, busy: agent.busy }"
        @click="selectAgent(agent)"
      >
        <div class="agent-icon">
          {{ agent.icon }}
        </div>
        <div class="agent-info">
          <span class="agent-name">{{ agent.name }}</span>
          <span class="agent-role">{{ agent.role }}</span>
        </div>
        <div class="agent-metrics">
          <span class="metric">
            <span class="label">Key:</span>
            <span class="value">{{ agent.keyName }}</span>
          </span>
          <span class="metric">
            <span class="label">Tasks:</span>
            <span class="value">{{ agent.tasksCompleted }}</span>
          </span>
        </div>
        <div class="agent-status-indicator">
          <span
            v-if="agent.active && !agent.busy"
            class="status-dot active"
          />
          <span
            v-else-if="agent.busy"
            class="status-dot busy"
          />
          <span
            v-else
            class="status-dot offline"
          />
        </div>
      </div>
    </div>

    <!-- Agent Detail Panel -->
    <div
      v-if="selectedAgent"
      class="agent-detail"
    >
      <div class="detail-header">
        <span>{{ selectedAgent.name }}</span>
        <button
          class="btn-close"
          @click="selectedAgent = null"
        >
          ×
        </button>
      </div>
      <div class="detail-content">
        <div class="detail-row">
          <span class="label">Model:</span>
          <span class="value">{{ selectedAgent.model }}</span>
        </div>
        <div class="detail-row">
          <span class="label">API Key:</span>
          <span class="value masked">{{ maskKey(selectedAgent.apiKey) }}</span>
        </div>
        <div class="detail-row">
          <span class="label">Awareness:</span>
          <span class="value">{{ selectedAgent.awareness.join(', ') }}</span>
        </div>
        <div class="detail-actions">
          <button
            class="btn"
            @click="testAgent(selectedAgent)"
          >
            [TEST]
          </button>
          <button
            class="btn"
            @click="pingAgent(selectedAgent)"
          >
            [PING]
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { useWebSocket } from '../lib/websocket.js'

const { subscribe, send } = useWebSocket()

const selectedAgent = ref(null)

const agents = ref([
  {
    id: 'kowalski-code',
    name: 'Kowalski-Code',
    icon: '⌨️',
    role: 'Code Specialist',
    model: 'zai-org/GLM-5-FP8',
    keyName: 'earth.tupa',
    apiKey: 'modalresearch_JdWLIUf3RomDuD-urYJFDu53daFXK6h1EYa2kovnQU0',
    active: true,
    busy: false,
    tasksCompleted: 0,
    awareness: ['kowalski-web', 'kowalski-data', 'kowalski-federation']
  },
  {
    id: 'kowalski-web',
    name: 'Kowalski-Web',
    icon: '🌐',
    role: 'Web Researcher',
    model: 'zai-org/GLM-5-FP8',
    keyName: 'hautlythird',
    apiKey: 'modalresearch_qP-Ak-bGqnNFf_Yqkz6uZVKtnOgPXB43r5NjS5vM6-M',
    active: true,
    busy: false,
    tasksCompleted: 0,
    awareness: ['kowalski-code', 'kowalski-academic', 'kowalski-federation']
  },
  {
    id: 'kowalski-academic',
    name: 'Kowalski-Academic',
    icon: '📚',
    role: 'Academic Analyst',
    model: 'zai-org/GLM-5-FP8',
    keyName: 'tupa@',
    apiKey: 'modalresearch_FUln_0wOE5kfqEn2ZrFUJP2X0CX0sIcyuHVG029UBLU',
    active: true,
    busy: false,
    tasksCompleted: 0,
    awareness: ['kowalski-data', 'kowalski-creative', 'kowalski-federation']
  },
  {
    id: 'kowalski-data',
    name: 'Kowalski-Data',
    icon: '📊',
    role: 'Data Processor',
    model: 'zai-org/GLM-5-FP8',
    keyName: 'touch',
    apiKey: 'modalresearch_vTibY1xsIE_pUscwtXSSQ7W7GMXrWl_KZmMDYO0sXmI',
    active: true,
    busy: false,
    tasksCompleted: 0,
    awareness: ['kowalski-code', 'kowalski-academic', 'kowalski-federation']
  },
  {
    id: 'kowalski-creative',
    name: 'Kowalski-Creative',
    icon: '🎨',
    role: 'Creative Synthesizer',
    model: 'zai-org/GLM-5-FP8',
    keyName: 'rouxy',
    apiKey: 'modalresearch_Ne49q128KCJDJkeh6l_dudnNauKHr6etnIkoc926-Qs',
    active: true,
    busy: false,
    tasksCompleted: 0,
    awareness: ['kowalski-web', 'kowalski-academic', 'kowalski-federation']
  },
  {
    id: 'kowalski-reasoning',
    name: 'Kowalski-Reasoning',
    icon: '🧠',
    role: 'Reasoning Engine',
    model: 'zai-org/GLM-5-FP8',
    keyName: 'hautly',
    apiKey: 'modalresearch__SaPVxSs_xtxttZaa9tAOwVi9jctW865yBY-EZBtzJI',
    active: true,
    busy: false,
    tasksCompleted: 0,
    awareness: ['kowalski-code', 'kowalski-data', 'kowalski-federation']
  },
  {
    id: 'kowalski-federation',
    name: 'Kowalski-Federation',
    icon: '☸️',
    role: 'Federation Coordinator',
    model: 'zai-org/GLM-5-FP8',
    keyName: 'housaky',
    apiKey: 'modalresearch_v_wbyTkPu707vdU6xzkd3CydAKwHHCAmtCZzAO0ZDA8',
    active: true,
    busy: false,
    tasksCompleted: 0,
    awareness: ['kowalski-code', 'kowalski-web', 'kowalski-academic', 'kowalski-data', 'kowalski-creative', 'kowalski-reasoning']
  }
])

const allActive = computed(() => {
  return agents.value.every(a => a.active)
})

function selectAgent(agent) {
  selectedAgent.value = agent
}

function maskKey(key) {
  if (!key) return 'N/A'
  return key.substring(0, 15) + '...' + key.substring(key.length - 4)
}

function testAgent(agent) {
  agent.busy = true
  send({
    type: 'agent_test',
    agentId: agent.id
  })
  
  setTimeout(() => {
    agent.busy = false
    agent.tasksCompleted++
  }, 2000)
}

function pingAgent(agent) {
  send({
    type: 'agent_ping',
    agentId: agent.id
  })
}

onMounted(() => {
  // Subscribe to agent status updates
  subscribe('agent_status', (data) => {
    const agent = agents.value.find(a => a.id === data.agentId)
    if (agent) {
      agent.active = data.active
      agent.busy = data.busy
      agent.tasksCompleted = data.tasksCompleted
    }
  })
})
</script>

<style scoped>
.subagent-status {
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid #333;
  padding: 15px;
}

.status-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
  padding-bottom: 10px;
  border-bottom: 1px solid #333;
}

.title {
  font-weight: bold;
  font-size: 14px;
}

.status {
  font-size: 12px;
}

.status.active {
  color: #0f0;
}

.agents-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
}

.agent-card {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid #333;
  padding: 10px;
  cursor: pointer;
  position: relative;
  transition: all 0.2s;
}

.agent-card:hover {
  border-color: #0f0;
  background: rgba(0, 255, 0, 0.05);
}

.agent-card.active {
  border-color: #0a0;
}

.agent-card.busy {
  border-color: #fa0;
}

.agent-icon {
  font-size: 24px;
  margin-bottom: 5px;
}

.agent-info {
  display: flex;
  flex-direction: column;
}

.agent-name {
  font-weight: bold;
  font-size: 12px;
}

.agent-role {
  font-size: 10px;
  color: #888;
}

.agent-metrics {
  margin-top: 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.metric {
  font-size: 10px;
}

.metric .label {
  color: #666;
}

.metric .value {
  color: #0f0;
}

.agent-status-indicator {
  position: absolute;
  top: 10px;
  right: 10px;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}

.status-dot.active {
  background: #0f0;
  box-shadow: 0 0 5px #0f0;
}

.status-dot.busy {
  background: #fa0;
  animation: pulse 1s infinite;
}

.status-dot.offline {
  background: #f00;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.agent-detail {
  margin-top: 15px;
  padding: 15px;
  background: rgba(0, 255, 0, 0.05);
  border: 1px solid #0f0;
}

.detail-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  font-weight: bold;
}

.btn-close {
  background: none;
  border: none;
  color: #f00;
  font-size: 20px;
  cursor: pointer;
}

.detail-row {
  display: flex;
  gap: 10px;
  margin-bottom: 5px;
  font-size: 12px;
}

.detail-row .label {
  color: #888;
  min-width: 80px;
}

.detail-row .value {
  color: #0f0;
}

.detail-row .value.masked {
  font-family: monospace;
  font-size: 10px;
}

.detail-actions {
  margin-top: 10px;
  display: flex;
  gap: 10px;
}

.btn {
  background: transparent;
  border: 1px solid #0f0;
  color: #0f0;
  padding: 5px 10px;
  cursor: pointer;
  font-family: monospace;
  font-size: 11px;
}

.btn:hover {
  background: #0f0;
  color: #000;
}
</style>
