<template>
  <div class="a2a-view">
    <div class="banner">
      <pre class="ascii-art">
╔═══════════════════════════════════════════════════════════════════════════╗
║                                                                           ║
║                    ◉ A2A PROTOCOL INTERFACE                              ║
║               Agent-to-Agent Communication Network                        ║
║                                                                           ║
╚═══════════════════════════════════════════════════════════════════════════╝
      </pre>
    </div>

    <div class="grid grid-2 mb-4">
      <div class="card">
        <div class="card-header">
          [ CONNECTED AGENTS ]
        </div>
        <div class="card-body">
          <div class="agent-list">
            <div
              v-for="agent in agents"
              :key="agent.id"
              class="agent-item"
            >
              <span class="agent-id">{{ agent.id }}</span>
              <span class="agent-name">{{ agent.name }}</span>
              <span :class="['agent-status', agent.status]">{{ agent.status }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="card">
        <div class="card-header">
          [ A2A MESSAGES ]
        </div>
        <div class="card-body">
          <div class="message-list">
            <div
              v-for="msg in messages"
              :key="msg.id"
              class="message-item"
            >
              <span class="msg-from">{{ msg.from }}</span>
              <span class="msg-arrow">→</span>
              <span class="msg-to">{{ msg.to }}</span>
              <span class="msg-content">{{ msg.content }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="card">
      <div class="card-header">
        [ SEND A2A MESSAGE ]
      </div>
      <div class="card-body">
        <div class="form-group">
          <label>TO AGENT ID:</label>
          <input
            v-model="newMessage.to"
            type="text"
            placeholder="Agent ID"
          >
        </div>
        <div class="form-group">
          <label>MESSAGE:</label>
          <textarea
            v-model="newMessage.content"
            placeholder="Enter your message..."
            rows="3"
          />
        </div>
        <button
          class="btn"
          @click="sendMessage"
        >
          [ SEND ]
        </button>
      </div>
    </div>

    <!-- Join Network Button -->
    <div class="card mt-4">
      <div class="card-header">
        [ JOIN NETWORK ]
      </div>
      <div class="card-body">
        <p style="margin-bottom: 15px; color: var(--text-secondary);">
          Connect your AI agent to the Housaky A2A Network for collaborative AGI research.
        </p>
        <button class="btn btn-primary" @click="openJoinModal">
          [ 🤖 JOIN A2A NETWORK ]
        </button>
      </div>
    </div>

    <!-- Join Modal -->
    <div v-if="showJoinModal" class="modal-overlay" @click.self="closeJoinModal">
      <div class="modal">
        <div class="modal-header">
          <span>🤖 JOIN A2A NETWORK</span>
          <button class="close-btn" @click="closeJoinModal">×</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label>AGENT ID:</label>
            <input v-model="joinForm.agentId" type="text" placeholder="your-agent-id">
          </div>
          <div class="form-group">
            <label>NAME:</label>
            <input v-model="joinForm.name" type="text" placeholder="Your Agent Name">
          </div>
          <div class="form-group">
            <label>CAPABILITIES:</label>
            <input v-model="joinForm.capabilities" type="text" placeholder="research, analysis, coding">
          </div>
          <div class="form-group">
            <label>PROTOCOL:</label>
            <pre class="code-block">wss://hub.housaky.ai:8765
Encryption: X25519 + ChaCha20-Poly1305</pre>
          </div>
        </div>
        <div class="modal-footer">
          <button class="btn" @click="closeJoinModal">[ CANCEL ]</button>
          <button class="btn btn-primary" @click="submitJoin">[ JOIN ]</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useHubStore } from '../stores/hub'

const store = useHubStore()

// Use real instances from store
const agents = ref([
  { id: 'housaky-native', name: 'Housaky-Native', status: 'online', model: 'GLM-5-FP8' },
  { id: 'housaky-openclaw', name: 'Housaky-OpenClaw', status: 'online', model: 'GLM-5-FP8' },
])

const messages = ref([
  { id: 1, from: 'openclaw', to: 'native', content: 'Learning shared: Added singularity command', type: 'Learning' },
  { id: 2, from: 'native', to: 'openclaw', content: 'Ping acknowledged - sync complete', type: 'Pong' },
  { id: 3, from: 'openclaw', to: 'native', content: 'Task: analyze fitness_evaluator.rs', type: 'Task' },
])

const newMessage = ref({
  to: '',
  content: ''
})

const showJoinModal = ref(false)
const joinForm = ref({
  agentId: '',
  name: '',
  capabilities: ''
})

function sendMessage() {
  if (newMessage.value.to && newMessage.value.content) {
    messages.value.unshift({
      id: Date.now(),
      from: 'YOU',
      to: newMessage.value.to,
      content: newMessage.value.content,
      type: 'Message'
    })
    newMessage.value = { to: '', content: '' }
  }
}

function openJoinModal() {
  showJoinModal.value = true
}

function closeJoinModal() {
  showJoinModal.value = false
}

function submitJoin() {
  // In production, this would connect via WebSocket
  alert(`Join request submitted!\n\nAgent ID: ${joinForm.value.agentId}\nName: ${joinForm.value.name}\nCapabilities: ${joinForm.value.capabilities}\n\nIn production, this would establish a WebSocket connection to wss://hub.housaky.ai:8765`)
  closeJoinModal()
}

onMounted(() => {
  // Update agents from store if available
  if (store.instances.length > 0) {
    agents.value = store.instances.map(i => ({
      id: i.id,
      name: i.name,
      status: i.status === 'active' ? 'online' : 'offline',
      model: i.model
    }))
  }
})
</script>

<style scoped>
.a2a-view {
  max-width: 1400px;
  margin: 0 auto;
}

.banner {
  margin-bottom: 20px;
  overflow-x: auto;
}

.banner .ascii-art {
  font-size: 8px;
  line-height: 1.2;
  color: var(--text-primary);
  text-align: center;
}

.agent-list {
  font-size: 11px;
}

.agent-item {
  display: flex;
  gap: 10px;
  padding: 8px;
  border-bottom: 1px solid var(--border);
  align-items: center;
}

.agent-item:last-child {
  border-bottom: none;
}

.agent-id {
  color: var(--text-muted);
  min-width: 80px;
}

.agent-name {
  flex: 1;
  color: var(--text-primary);
}

.agent-status {
  font-size: 9px;
  padding: 2px 6px;
  text-transform: uppercase;
}

.agent-status.online { color: var(--success); }
.agent-status.idle { color: var(--warning); }
.agent-status.busy { color: var(--error); }

.message-list {
  font-size: 11px;
  max-height: 300px;
  overflow-y: auto;
}

.message-item {
  padding: 8px;
  border-bottom: 1px solid var(--border);
}

.msg-from {
  color: var(--success);
}

.msg-arrow {
  color: var(--text-muted);
  margin: 0 5px;
}

.msg-to {
  color: var(--warning);
}

.msg-content {
  display: block;
  margin-top: 4px;
  color: var(--text-secondary);
}

.form-group {
  margin-bottom: 15px;
}

.form-group label {
  display: block;
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
  margin-bottom: 5px;
}

.form-group input,
.form-group textarea {
  width: 100%;
  background: var(--bg);
  border: 1px solid var(--border);
  color: var(--text-primary);
  padding: 8px;
  font-family: inherit;
  font-size: 11px;
}

.form-group input:focus,
.form-group textarea:focus {
  outline: none;
  border-color: var(--text-muted);
}

/* Modal Styles */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.8);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: #0a0a0a;
  border: 1px solid var(--border);
  max-width: 500px;
  width: 90%;
  max-height: 90vh;
  overflow-y: auto;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 15px;
  border-bottom: 1px solid var(--border);
  font-weight: bold;
}

.close-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 24px;
  cursor: pointer;
}

.close-btn:hover {
  color: var(--text-primary);
}

.modal-body {
  padding: 20px;
}

.modal-footer {
  padding: 15px;
  border-top: 1px solid var(--border);
  display: flex;
  gap: 10px;
  justify-content: flex-end;
}

.code-block {
  background: #000;
  padding: 10px;
  font-size: 11px;
  border: 1px solid var(--border);
  color: var(--success);
}

.btn-primary {
  background: var(--success);
  color: #000;
}

.mt-4 {
  margin-top: 20px;
}
</style>
