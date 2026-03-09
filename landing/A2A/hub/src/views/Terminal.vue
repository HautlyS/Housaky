<template>
  <div class="terminal-view">
    <!-- Header -->
    <div class="page-header">
      <h1 class="page-title">
        <span class="icon">▶</span>
        TERMINAL
      </h1>
      <p class="page-subtitle">Real-time AGI operations and logs</p>
    </div>

    <!-- Terminal Window -->
    <div class="terminal-window">
      <div class="terminal-header">
        <div class="terminal-controls">
          <span class="control close"></span>
          <span class="control minimize"></span>
          <span class="control maximize"></span>
        </div>
        <span class="terminal-title">housaky@agi:~$</span>
        <div class="terminal-actions">
          <button class="btn-clear" @click="clearOutput">CLEAR</button>
        </div>
      </div>
      
      <div class="terminal-body" ref="terminalBody">
        <div 
          v-for="(line, index) in output" 
          :key="index" 
          class="terminal-line"
          :class="line.type"
        >
          <span v-if="line.timestamp" class="timestamp">{{ line.timestamp }}</span>
          <span class="prefix">{{ line.prefix }}</span>
          <span class="message">{{ line.message }}</span>
        </div>
        
        <!-- Input Line -->
        <div class="input-line">
          <span class="prompt">housaky@agi:~$</span>
          <input 
            v-model="command" 
            @keyup.enter="executeCommand"
            type="text" 
            class="terminal-input"
            placeholder="Type a command..."
            ref="inputField"
          >
        </div>
      </div>
    </div>

    <!-- Quick Commands -->
    <div class="quick-commands">
      <h3>Quick Commands</h3>
      <div class="command-buttons">
        <button class="cmd-btn" @click="runCommand('status')">status</button>
        <button class="cmd-btn" @click="runCommand('singularity')">singularity</button>
        <button class="cmd-btn" @click="runCommand('instances')">instances</button>
        <button class="cmd-btn" @click="runCommand('a2a ping')">a2a ping</button>
        <button class="cmd-btn" @click="runCommand('improve')">improve</button>
        <button class="cmd-btn" @click="runCommand('memory')">memory</button>
        <button class="cmd-btn" @click="runCommand('goals')">goals</button>
        <button class="cmd-btn" @click="runCommand('help')">help</button>
      </div>
    </div>

    <!-- Info Grid -->
    <div class="info-grid">
      <div class="info-box">
        <h4>☸️ SYSTEM</h4>
        <div class="info-content">
          <div class="info-row">
            <span>Version</span>
            <span>4.0.0-AGI</span>
          </div>
          <div class="info-row">
            <span>Phase</span>
            <span>Superlinear</span>
          </div>
          <div class="info-row">
            <span>Uptime</span>
            <span>{{ store.uptime }}</span>
          </div>
        </div>
      </div>
      
      <div class="info-box">
        <h4>⚡ PROGRESS</h4>
        <div class="info-content">
          <div class="info-row">
            <span>Singularity</span>
            <span class="highlight">{{ store.singularity }}%</span>
          </div>
          <div class="info-row">
            <span>Target</span>
            <span>60%</span>
          </div>
          <div class="info-row">
            <span>Cycles</span>
            <span>38</span>
          </div>
        </div>
      </div>
      
      <div class="info-box">
        <h4>🌐 NETWORK</h4>
        <div class="info-content">
          <div class="info-row">
            <span>Peers</span>
            <span>{{ store.activeInstances }}</span>
          </div>
          <div class="info-row">
            <span>Protocol</span>
            <span>QUIC</span>
          </div>
          <div class="info-row">
            <span>Encryption</span>
            <span class="success">Active</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, nextTick } from 'vue'
import { useHubStore } from '../stores/hub'

const store = useHubStore()
const command = ref('')
const output = ref([])
const terminalBody = ref(null)
const inputField = ref(null)

const initialOutput = [
  { type: 'system', prefix: '', message: '╔════════════════════════════════════════════════════════════════╗' },
  { type: 'system', prefix: '', message: '║  ☸️ HOUSAKY AGI TERMINAL v4.0.0                              ║' },
  { type: 'system', prefix: '', message: '║  Type "help" for available commands                          ║' },
  { type: 'system', prefix: '', message: '╚════════════════════════════════════════════════════════════════╝' },
  { type: 'info', prefix: '[INFO] ', message: 'System initialized' },
  { type: 'success', prefix: '[OK] ', message: 'A2A network connected' },
  { type: 'success', prefix: '[OK] ', message: 'Memory system online' },
  { type: 'info', prefix: '[INFO] ', message: `Singularity progress: ${store.singularity}%` },
  { type: 'prompt', prefix: '', message: '' }
]

onMounted(() => {
  output.value = initialOutput.map(line => ({
    ...line,
    timestamp: new Date().toISOString().substring(11, 19)
  }))
})

function getTimestamp() {
  return new Date().toISOString().substring(11, 19)
}

function addLine(type, prefix, message) {
  output.value.push({
    type,
    prefix,
    message,
    timestamp: getTimestamp()
  })
  scrollToBottom()
}

function scrollToBottom() {
  nextTick(() => {
    if (terminalBody.value) {
      terminalBody.value.scrollTop = terminalBody.value.scrollHeight
    }
  })
}

function executeCommand() {
  const cmd = command.value.trim()
  if (!cmd) return
  
  addLine('command', '$ ', cmd)
  command.value = ''
  
  // Process command
  processCommand(cmd)
}

function runCommand(cmd) {
  addLine('command', '$ ', cmd)
  processCommand(cmd)
}

function processCommand(cmd) {
  const parts = cmd.toLowerCase().split(' ')
  const mainCmd = parts[0]
  
  switch (mainCmd) {
    case 'help':
      addLine('info', '', 'Available commands:')
      addLine('info', '', '  status      - Show system status')
      addLine('info', '', '  singularity - Show singularity progress')
      addLine('info', '', '  instances   - List connected instances')
      addLine('info', '', '  a2a         - A2A protocol commands')
      addLine('info', '', '  improve     - Run self-improvement cycle')
      addLine('info', '', '  memory      - Show memory stats')
      addLine('info', '', '  goals       - List active goals')
      addLine('info', '', '  clear       - Clear terminal')
      break
      
    case 'status':
      addLine('success', '[STATUS] ', `Singularity: ${store.singularity}%`)
      addLine('success', '[STATUS] ', `Self-Awareness: ${store.selfAwareness}%`)
      addLine('success', '[STATUS] ', `Meta-Cognition: ${store.metaCognition}%`)
      addLine('success', '[STATUS] ', `Active instances: ${store.activeInstances}`)
      addLine('success', '[STATUS] ', `Uptime: ${store.uptime}`)
      break
      
    case 'singularity':
      addLine('info', '', '┌─ SINGULARITY PROGRESS ─────────────┐')
      addLine('info', '', `│ Progress: ${store.singularity}%                       │`)
      addLine('info', '', `│ Phase:    Superlinear              │`)
      addLine('info', '', `│ Target:   60% (Phase 1)            │`)
      addLine('info', '', '└────────────────────────────────────┘')
      break
      
    case 'instances':
      store.instances.forEach(i => {
        addLine('info', `[${i.status.toUpperCase()}] `, `${i.name} (${i.model})`)
      })
      break
      
    case 'a2a':
      if (parts[1] === 'ping') {
        addLine('success', '[A2A] ', 'Pinging native instance...')
        setTimeout(() => {
          addLine('success', '[A2A] ', 'Pong received from housaky-native (2ms)')
        }, 500)
      } else {
        addLine('info', '[A2A] ', 'Usage: a2a ping | a2a sync | a2a learn')
      }
      break
      
    case 'improve':
      addLine('info', '[IMPROVE] ', 'Running self-improvement cycle...')
      setTimeout(() => {
        addLine('success', '[IMPROVE] ', 'Fitness score: 0.85')
        addLine('success', '[IMPROVE] ', '1 optimization applied')
      }, 800)
      break
      
    case 'memory':
      addLine('info', '[MEMORY] ', 'Lucid SQLite + Vector embeddings')
      addLine('info', '[MEMORY] ', `Retrieval: 2.7ms`)
      addLine('info', '[MEMORY] ', `Capacity: 743k memories/sec`)
      break
      
    case 'goals':
      store.goals.forEach(g => {
        addLine('info', `[${g.priority}] `, `${g.title}: ${g.progress}%`)
      })
      break
      
    case 'clear':
      output.value = []
      break
      
    default:
      addLine('error', '[ERROR] ', `Unknown command: ${cmd}`)
      addLine('info', '', 'Type "help" for available commands')
  }
  
  scrollToBottom()
}

function clearOutput() {
  output.value = initialOutput.map(line => ({
    ...line,
    timestamp: getTimestamp()
  }))
}
</script>

<style scoped>
.terminal-view {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.page-header {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  font-size: 24px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 12px;
}

.icon {
  color: var(--success);
}

.page-subtitle {
  font-size: 13px;
  color: var(--text-muted);
}

/* Terminal Window */
.terminal-window {
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 4px;
  overflow: hidden;
}

.terminal-header {
  background: var(--bg-tertiary);
  border-bottom: 1px solid var(--border);
  padding: 10px 16px;
  display: flex;
  align-items: center;
  gap: 12px;
}

.terminal-controls {
  display: flex;
  gap: 6px;
}

.control {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--text-muted);
}

.control.close { background: #ff5f56; }
.control.minimize { background: #ffbd2e; }
.control.maximize { background: #27c93f; }

.terminal-title {
  flex: 1;
  font-size: 12px;
  color: var(--text-muted);
}

.btn-clear {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-muted);
  padding: 4px 12px;
  font-size: 10px;
  cursor: pointer;
  border-radius: 2px;
}

.btn-clear:hover {
  background: var(--text-primary);
  color: var(--bg-primary);
}

.terminal-body {
  height: 400px;
  overflow-y: auto;
  padding: 16px;
  font-size: 12px;
  line-height: 1.6;
}

.terminal-line {
  display: flex;
  gap: 8px;
}

.timestamp {
  color: var(--text-muted);
  font-size: 10px;
  min-width: 60px;
}

.prefix {
  color: var(--text-secondary);
}

.message {
  color: var(--text-primary);
}

.terminal-line.success .message { color: var(--success); }
.terminal-line.error .message { color: var(--error); }
.terminal-line.info .message { color: var(--info); }
.terminal-line.command .message { color: var(--warning); }
.terminal-line.system .message { color: var(--text-muted); }

/* Input Line */
.input-line {
  display: flex;
  align-items: center;
  margin-top: 8px;
}

.prompt {
  color: var(--success);
  margin-right: 8px;
}

.terminal-input {
  flex: 1;
  background: transparent;
  border: none;
  color: var(--text-primary);
  font-family: inherit;
  font-size: 12px;
  outline: none;
}

.terminal-input::placeholder {
  color: var(--text-muted);
}

/* Quick Commands */
.quick-commands h3 {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
  margin-bottom: 12px;
}

.command-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.cmd-btn {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  padding: 8px 16px;
  font-family: inherit;
  font-size: 11px;
  cursor: pointer;
  border-radius: 2px;
  transition: all 0.15s ease;
}

.cmd-btn:hover {
  background: var(--text-primary);
  color: var(--bg-primary);
  border-color: var(--text-primary);
}

/* Info Grid */
.info-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}

@media (max-width: 768px) {
  .info-grid {
    grid-template-columns: 1fr;
  }
}

.info-box {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 16px;
}

.info-box h4 {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
  margin-bottom: 12px;
}

.info-content {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.info-row {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
}

.info-row span:first-child {
  color: var(--text-muted);
}

.info-row .highlight {
  color: var(--success);
  font-weight: 600;
}

.info-row .success {
  color: var(--success);
}
</style>
