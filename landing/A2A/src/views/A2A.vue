<template>
  <div class="a2a-view">
    <div class="banner">
      <pre class="ascii-art">
╔═══════════════════════════════════════════════════════════════════════════╗
║                                                                           ║
║                    ◉ A2A PROTOCOL INTERFACE                              ║
║               Agent-to-Agent Communication Network                        ║
║                    ws://localhost:8765                                   ║
║                                                                           ║
╚═══════════════════════════════════════════════════════════════════════════╝
      </pre>
    </div>

    <!-- Connection Status -->
    <div class="connection-bar">
      <span :class="['status-dot', wsConnected ? 'connected' : 'disconnected']">●</span>
      <span>{{ wsConnected ? 'CONNECTED' : 'DISCONNECTED' }}</span>
      <span class="divider">|</span>
      <span>MESSAGES: {{ messages.length }}</span>
      <span class="divider">|</span>
      <span>LEARNINGS: {{ learnings.length }}</span>
      <button class="btn-small" @click="connectWebSocket">{{ wsConnected ? '[ RECONNECT ]' : '[ CONNECT ]' }}</button>
    </div>

    <div class="grid grid-2 mb-4">
      <!-- Real Agents from Store -->
      <div class="card">
        <div class="card-header">
          [ CONNECTED INSTANCES ]
        </div>
        <div class="card-body">
          <div class="agent-list">
            <div
              v-for="instance in store.instances"
              :key="instance.id"
              class="agent-item"
            >
              <span class="agent-id">{{ instance.id.slice(0, 12) }}...</span>
              <span class="agent-name">{{ instance.name }}</span>
              <span :class="['agent-status', instance.status]">{{ instance.status }}</span>
              <span class="agent-model">{{ instance.model }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- A2A Messages -->
      <div class="card">
        <div class="card-header">
          [ A2A MESSAGES <span class="live-indicator">LIVE</span> ]
        </div>
        <div class="card-body">
          <div class="message-list" ref="messageList">
            <div
              v-for="msg in messages"
              :key="msg.id"
              :class="['message-item', `msg-type-${msg.type}`]"
            >
              <div class="msg-header">
                <span class="msg-from">{{ msg.from }}</span>
                <span class="msg-arrow">→</span>
                <span class="msg-to">{{ msg.to }}</span>
                <span class="msg-type">[{{ msg.type }}]</span>
              </div>
              <div class="msg-content">{{ msg.content }}</div>
              <div class="msg-time">{{ formatTime(msg.ts) }}</div>
            </div>
            <div v-if="messages.length === 0" class="empty-state">
              No messages yet. Send one below!
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Learnings Section -->
    <div class="card mb-4">
      <div class="card-header">
        [ SHARED LEARNINGS ]
      </div>
      <div class="card-body">
        <div class="learning-list">
          <div v-for="learning in learnings" :key="learning.id" class="learning-item">
            <span class="learning-category">[{{ learning.category }}]</span>
            <span class="learning-content">{{ learning.content }}</span>
            <span class="learning-confidence">{{ (learning.confidence * 100).toFixed(0) }}%</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Send Message -->
    <div class="card">
      <div class="card-header">
        [ SEND A2A MESSAGE ]
      </div>
      <div class="card-body">
        <div class="form-row">
          <div class="form-group">
            <label>TO INSTANCE:</label>
            <select v-model="newMessage.to">
              <option value="native">Housaky-Native</option>
              <option value="broadcast">ALL (Broadcast)</option>
            </select>
          </div>
          <div class="form-group">
            <label>MESSAGE TYPE:</label>
            <select v-model="newMessage.type">
              <option value="Learning">Learning</option>
              <option value="Task">Task Request</option>
              <option value="Context">Context Share</option>
              <option value="SyncRequest">Sync Request</option>
              <option value="CodeImprove">Code Improvement</option>
            </select>
          </div>
        </div>
        
        <div v-if="newMessage.type === 'Learning'" class="form-row">
          <div class="form-group">
            <label>CATEGORY:</label>
            <select v-model="newMessage.category">
              <option value="reasoning">Reasoning</option>
              <option value="optimization">Optimization</option>
              <option value="architecture">Architecture</option>
              <option value="memory">Memory</option>
              <option value="consciousness">Consciousness</option>
              <option value="ethics">Ethics</option>
              <option value="dharma">Dharma</option>
            </select>
          </div>
          <div class="form-group">
            <label>CONFIDENCE:</label>
            <input type="range" v-model="newMessage.confidence" min="0" max="100" step="5">
            <span>{{ newMessage.confidence }}%</span>
          </div>
        </div>
        
        <div class="form-group">
          <label>CONTENT:</label>
          <textarea
            v-model="newMessage.content"
            placeholder="Enter your message..."
            rows="3"
          />
        </div>
        <button class="btn" @click="sendMessage" :disabled="!newMessage.content">
          [ SEND VIA A2A ]
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import { useHubStore } from '../stores/hub'

const store = useHubStore()
const wsConnected = ref(false)
const ws = ref(null)
const messageList = ref(null)

const messages = ref([
  { id: 1, from: 'openclaw', to: 'native', type: 'Sync', content: 'Requesting state synchronization', ts: Date.now() - 60000 },
  { id: 2, from: 'native', to: 'openclaw', type: 'SyncResponse', content: 'State synced: singularity 47%, awareness 30%', ts: Date.now() - 55000 },
])

const learnings = ref([
  { id: 1, category: 'architecture', content: 'Use modular skill system for tool composition', confidence: 0.92 },
  { id: 2, category: 'optimization', content: 'WebSocket pooling reduces connection overhead by 60%', confidence: 0.88 },
  { id: 3, category: 'consciousness', content: 'Global Workspace Theory improves attention allocation', confidence: 0.85 },
])

const newMessage = ref({
  to: 'native',
  type: 'Learning',
  category: 'reasoning',
  confidence: 90,
  content: ''
})

function formatTime(ts) {
  return new Date(ts).toLocaleTimeString()
}

function connectWebSocket() {
  if (ws.value) {
    ws.value.close()
  }
  
  try {
    ws.value = new WebSocket('ws://localhost:8765')
    
    ws.value.onopen = () => {
      wsConnected.value = true
      addSystemMessage('Connected to A2A Hub')
    }
    
    ws.value.onclose = () => {
      wsConnected.value = false
      addSystemMessage('Disconnected from A2A Hub')
    }
    
    ws.value.onerror = (err) => {
      wsConnected.value = false
      addSystemMessage('Connection error - A2A Hub not running?')
    }
    
    ws.value.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data)
        handleIncomingMessage(msg)
      } catch (e) {
        console.error('Failed to parse A2A message:', e)
      }
    }
  } catch (e) {
    console.error('WebSocket error:', e)
  }
}

function handleIncomingMessage(msg) {
  const displayMsg = {
    id: msg.id || Date.now(),
    from: msg.from || 'unknown',
    to: msg.to || 'all',
    type: msg.t || 'Unknown',
    content: formatMessageContent(msg),
    ts: msg.ts || Date.now()
  }
  
  messages.value.unshift(displayMsg)
  scrollToBottom()
}

function formatMessageContent(msg) {
  if (msg.d) {
    if (msg.d.content) return msg.d.content
    if (msg.d.category) return `[${msg.d.category}] ${msg.d.content || ''}`
    return JSON.stringify(msg.d).slice(0, 100)
  }
  return 'Empty message'
}

function addSystemMessage(content) {
  messages.value.unshift({
    id: Date.now(),
    from: 'SYSTEM',
    to: 'YOU',
    type: 'System',
    content,
    ts: Date.now()
  })
}

function sendMessage() {
  const msg = {
    id: `a2a-${Date.now()}`,
    from: 'openclaw',
    to: newMessage.value.to,
    ts: Date.now(),
    pri: 2,
    t: newMessage.value.type
  }
  
  // Build data based on message type
  switch (newMessage.value.type) {
    case 'Learning':
      msg.d = {
        category: newMessage.value.category,
        content: newMessage.value.content,
        confidence: newMessage.value.confidence / 100
      }
      break
    case 'Task':
      msg.d = {
        id: `task-${Date.now()}`,
        action: 'analyze',
        params: { query: newMessage.value.content }
      }
      break
    case 'Context':
      msg.d = {
        memory_type: 'shared',
        data: { content: newMessage.value.content }
      }
      break
    default:
      msg.d = { content: newMessage.value.content }
  }
  
  // Send via WebSocket if connected
  if (wsConnected.value && ws.value) {
    ws.value.send(JSON.stringify(msg))
  }
  
  // Add to local messages
  messages.value.unshift({
    id: msg.id,
    from: 'openclaw',
    to: msg.to,
    type: msg.t,
    content: newMessage.value.content,
    ts: msg.ts
  })
  
  // Add to learnings if it's a learning
  if (msg.t === 'Learning') {
    learnings.value.unshift({
      id: Date.now(),
      category: msg.d.category,
      content: msg.d.content,
      confidence: msg.d.confidence
    })
  }
  
  newMessage.value.content = ''
  scrollToBottom()
}

function scrollToBottom() {
  nextTick(() => {
    if (messageList.value) {
      messageList.value.scrollTop = 0
    }
  })
}

onMounted(() => {
  connectWebSocket()
})

onUnmounted(() => {
  if (ws.value) {
    ws.value.close()
  }
})
</script>

<style scoped>
.a2a-view {
  max-width: 1400px;
  margin: 0 auto;
}

.banner {
  margin-bottom: 15px;
  overflow-x: auto;
}

.banner .ascii-art {
  font-size: 7px;
  line-height: 1.2;
  color: var(--text-primary);
  text-align: center;
}

.connection-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 15px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  margin-bottom: 15px;
  font-size: 11px;
}

.status-dot {
  font-size: 10px;
}

.status-dot.connected { color: var(--success); }
.status-dot.disconnected { color: var(--error); }

.divider { color: var(--text-muted); }

.live-indicator {
  color: var(--success);
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.agent-list, .message-list, .learning-list {
  font-size: 11px;
}

.agent-item, .message-item, .learning-item {
  display: flex;
  gap: 10px;
  padding: 8px;
  border-bottom: 1px solid var(--border);
  align-items: center;
}

.agent-item:last-child, .message-item:last-child, .learning-item:last-child {
  border-bottom: none;
}

.agent-id { color: var(--text-muted); min-width: 100px; font-family: monospace; font-size: 9px; }
.agent-name { flex: 1; color: var(--text-primary); }
.agent-model { color: var(--text-muted); font-size: 9px; }
.agent-status { font-size: 9px; padding: 2px 6px; text-transform: uppercase; }
.agent-status.active { color: var(--success); }
.agent-status.idle { color: var(--warning); }
.agent-status.offline { color: var(--error); }

.message-list { max-height: 400px; overflow-y: auto; }
.message-item { flex-direction: column; align-items: flex-start; gap: 4px; }
.msg-header { display: flex; gap: 5px; align-items: center; }
.msg-from { color: var(--success); font-weight: bold; }
.msg-arrow { color: var(--text-muted); }
.msg-to { color: #00ffff; }
.msg-type { color: var(--warning); font-size: 9px; }
.msg-content { color: var(--text-secondary); padding-left: 10px; border-left: 2px solid var(--border); margin-top: 4px; }
.msg-time { color: var(--text-muted); font-size: 9px; }
.msg-type-Learning .msg-content { border-color: #ff00ff; }
.msg-type-Task .msg-content { border-color: #00ffff; }
.msg-type-Context .msg-content { border-color: #ffff00; }

.learning-category { color: #ff00ff; min-width: 100px; }
.learning-content { flex: 1; color: var(--text-primary); }
.learning-confidence { color: var(--success); font-size: 10px; }

.empty-state {
  color: var(--text-muted);
  text-align: center;
  padding: 20px;
  font-style: italic;
}

.form-row {
  display: flex;
  gap: 20px;
  margin-bottom: 15px;
}

.form-row .form-group {
  flex: 1;
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
.form-group textarea,
.form-group select {
  width: 100%;
  background: var(--bg);
  border: 1px solid var(--border);
  color: var(--text-primary);
  padding: 8px;
  font-family: inherit;
  font-size: 11px;
}

.form-group input:focus,
.form-group textarea:focus,
.form-group select:focus {
  outline: none;
  border-color: var(--text-muted);
}

.btn-small {
  margin-left: auto;
  padding: 4px 10px;
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-primary);
  font-family: inherit;
  font-size: 10px;
  cursor: pointer;
}

.btn-small:hover {
  background: var(--text-primary);
  color: var(--bg);
}
</style>
