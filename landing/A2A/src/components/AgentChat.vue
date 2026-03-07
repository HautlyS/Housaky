<template>
  <div class="agent-chat">
    <div class="chat-header">
      <span class="title">[ INTER-AGENT COMMUNICATION ]</span>
      <select
        v-model="selectedChannel"
        class="channel-select"
      >
        <option value="broadcast">
          📡 Broadcast (All Agents)
        </option>
        <option value="federation">
          ☸️ Federation Channel
        </option>
        <option value="code">
          ⌨️ Code Team
        </option>
        <option value="research">
          🔍 Research Team
        </option>
      </select>
    </div>

    <div
      ref="messagesContainer"
      class="chat-messages"
    >
      <div 
        v-for="(msg, idx) in messages" 
        :key="idx" 
        class="message"
        :class="msg.type"
      >
        <div class="message-header">
          <span class="sender">{{ msg.sender }}</span>
          <span class="time">{{ formatTime(msg.timestamp) }}</span>
        </div>
        <div class="message-content">
          {{ msg.content }}
        </div>
        <div
          v-if="msg.metadata"
          class="message-meta"
        >
          <span
            v-for="(value, key) in msg.metadata"
            :key="key"
            class="meta"
          >
            {{ key }}: {{ value }}
          </span>
        </div>
      </div>
    </div>

    <div class="chat-input">
      <div class="sender-select">
        <select v-model="sender">
          <option
            v-for="agent in agents"
            :key="agent.id"
            :value="agent.id"
          >
            {{ agent.icon }} {{ agent.name }}
          </option>
        </select>
      </div>
      <input 
        v-model="inputMessage"
        placeholder="Type message to agents..."
        class="message-input"
        @keyup.enter="sendMessage"
      >
      <button
        class="btn-send"
        @click="sendMessage"
      >
        [SEND]
      </button>
    </div>

    <div class="quick-commands">
      <button
        class="btn-cmd"
        @click="sendCommand('status')"
      >
        [STATUS]
      </button>
      <button
        class="btn-cmd"
        @click="sendCommand('sync')"
      >
        [SYNC]
      </button>
      <button
        class="btn-cmd"
        @click="sendCommand('report')"
      >
        [REPORT]
      </button>
      <button
        class="btn-cmd"
        @click="sendCommand('improve')"
      >
        [IMPROVE]
      </button>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, nextTick } from 'vue'
import { useWebSocket } from '../lib/websocket.js'

const { connect, subscribe, send } = useWebSocket()

const selectedChannel = ref('broadcast')
const sender = ref('kowalski-federation')
const inputMessage = ref('')
const messages = ref([])
const messagesContainer = ref(null)

const agents = [
  { id: 'kowalski-code', name: 'Kowalski-Code', icon: '⌨️' },
  { id: 'kowalski-web', name: 'Kowalski-Web', icon: '🌐' },
  { id: 'kowalski-academic', name: 'Kowalski-Academic', icon: '📚' },
  { id: 'kowalski-data', name: 'Kowalski-Data', icon: '📊' },
  { id: 'kowalski-creative', name: 'Kowalski-Creative', icon: '🎨' },
  { id: 'kowalski-reasoning', name: 'Kowalski-Reasoning', icon: '🧠' },
  { id: 'kowalski-federation', name: 'Kowalski-Federation', icon: '☸️' },
]

function formatTime(timestamp) {
  const date = new Date(timestamp)
  return date.toLocaleTimeString()
}

function sendMessage() {
  if (!inputMessage.value.trim()) return
  
  const msg = {
    type: 'agent_message',
    sender: sender.value,
    channel: selectedChannel.value,
    content: inputMessage.value,
    timestamp: Date.now()
  }
  
  messages.value.push(msg)
  send(msg)
  inputMessage.value = ''
  
  scrollToBottom()
}

function sendCommand(cmd) {
  const msg = {
    type: 'agent_command',
    sender: sender.value,
    channel: selectedChannel.value,
    command: cmd,
    timestamp: Date.now()
  }
  
  messages.value.push({
    ...msg,
    content: `[COMMAND] ${cmd.toUpperCase()}`,
    type: 'system'
  })
  
  send(msg)
  scrollToBottom()
}

function scrollToBottom() {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
    }
  })
}

onMounted(async () => {
  // Connect WebSocket
  try {
    await connect()
  } catch (e) {
    console.error('Failed to connect WebSocket:', e)
  }
  
  // Subscribe to agent messages
  subscribe('agent_message', (data) => {
    messages.value.push(data)
    scrollToBottom()
  })
  
  // Subscribe to system messages
  subscribe('system', (data) => {
    messages.value.push({
      type: 'system',
      sender: 'SYSTEM',
      content: data.message,
      timestamp: Date.now()
    })
    scrollToBottom()
  })
})
</script>

<style scoped>
.agent-chat {
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid #333;
  display: flex;
  flex-direction: column;
  height: 100%;
}

.chat-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 15px;
  border-bottom: 1px solid #333;
}

.title {
  font-weight: bold;
  font-size: 14px;
}

.channel-select {
  background: #111;
  border: 1px solid #333;
  color: #0f0;
  padding: 5px 10px;
  font-family: monospace;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 10px;
  min-height: 200px;
}

.message {
  margin-bottom: 10px;
  padding: 8px;
  border-left: 3px solid #333;
  background: rgba(255, 255, 255, 0.02);
}

.message.agent_message {
  border-left-color: #0f0;
}

.message.system {
  border-left-color: #00f;
  background: rgba(0, 0, 255, 0.1);
}

.message.command {
  border-left-color: #fa0;
}

.message-header {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  margin-bottom: 5px;
}

.sender {
  color: #0f0;
  font-weight: bold;
}

.time {
  color: #666;
}

.message-content {
  font-size: 13px;
  line-height: 1.4;
}

.message-meta {
  margin-top: 5px;
  font-size: 10px;
  color: #666;
}

.meta {
  margin-right: 10px;
}

.chat-input {
  display: flex;
  gap: 10px;
  padding: 10px;
  border-top: 1px solid #333;
}

.sender-select select {
  background: #111;
  border: 1px solid #333;
  color: #0f0;
  padding: 8px;
  font-family: monospace;
}

.message-input {
  flex: 1;
  background: #111;
  border: 1px solid #333;
  color: #fff;
  padding: 8px;
  font-family: monospace;
}

.message-input:focus {
  outline: none;
  border-color: #0f0;
}

.btn-send {
  background: transparent;
  border: 1px solid #0f0;
  color: #0f0;
  padding: 8px 15px;
  cursor: pointer;
  font-family: monospace;
}

.btn-send:hover {
  background: #0f0;
  color: #000;
}

.quick-commands {
  display: flex;
  gap: 10px;
  padding: 10px;
  border-top: 1px solid #222;
}

.btn-cmd {
  background: transparent;
  border: 1px solid #666;
  color: #888;
  padding: 5px 10px;
  cursor: pointer;
  font-family: monospace;
  font-size: 11px;
}

.btn-cmd:hover {
  border-color: #0f0;
  color: #0f0;
}
</style>
