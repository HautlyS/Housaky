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
        <div class="card-header">[ CONNECTED AGENTS ]</div>
        <div class="card-body">
          <div class="agent-list">
            <div v-for="agent in agents" :key="agent.id" class="agent-item">
              <span class="agent-id">{{ agent.id }}</span>
              <span class="agent-name">{{ agent.name }}</span>
              <span :class="['agent-status', agent.status]">{{ agent.status }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="card">
        <div class="card-header">[ A2A MESSAGES ]</div>
        <div class="card-body">
          <div class="message-list">
            <div v-for="msg in messages" :key="msg.id" class="message-item">
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
      <div class="card-header">[ SEND A2A MESSAGE ]</div>
      <div class="card-body">
        <div class="form-group">
          <label>TO AGENT ID:</label>
          <input type="text" v-model="newMessage.to" placeholder="Agent ID" />
        </div>
        <div class="form-group">
          <label>MESSAGE:</label>
          <textarea v-model="newMessage.content" placeholder="Enter your message..." rows="3"></textarea>
        </div>
        <button class="btn" @click="sendMessage">[ SEND ]</button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const agents = ref([
  { id: 'AGENT-001', name: 'Kowalski', status: 'online' },
  { id: 'AGENT-002', name: 'DeepThink', status: 'online' },
  { id: 'AGENT-003', name: 'MemoryCore', status: 'online' },
  { id: 'AGENT-004', name: 'SkillMaster', status: 'idle' },
  { id: 'AGENT-005', name: 'ResearchBot', status: 'busy' },
])

const messages = ref([
  { id: 1, from: 'AGENT-001', to: 'AGENT-002', content: 'Task completed: Data analysis finished' },
  { id: 2, from: 'AGENT-003', to: 'AGENT-001', content: 'Memory sync confirmed' },
  { id: 3, from: 'AGENT-002', to: 'ALL', content: 'New research paper available' },
])

const newMessage = ref({
  to: '',
  content: ''
})

function sendMessage() {
  if (newMessage.value.to && newMessage.value.content) {
    messages.value.unshift({
      id: Date.now(),
      from: 'YOU',
      to: newMessage.value.to,
      content: newMessage.value.content
    })
    newMessage.value = { to: '', content: '' }
  }
}
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
</style>
