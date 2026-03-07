<template>
  <div class="terminal-view">
    <div class="terminal-container">
      <div class="terminal-header-bar">
        <span class="terminal-title">HOUSAKY TERMINAL v1.0.0</span>
        <div class="terminal-controls">
          <span class="control">_</span>
          <span class="control">□</span>
          <span class="control">×</span>
        </div>
      </div>
      <div
        ref="terminalBody"
        class="terminal-body"
      >
        <div
          v-for="(line, index) in history"
          :key="index"
          class="terminal-line"
        >
          <span class="prompt">housaky@agi:~$</span>
          <span class="command">{{ line.command }}</span>
          <div
            v-if="line.output"
            class="output"
          >
            {{ line.output }}
          </div>
        </div>
        <div class="input-line">
          <span class="prompt">housaky@agi:~$</span>
          <input 
            ref="inputRef" 
            v-model="currentInput" 
            type="text"
            class="terminal-input"
            autofocus
            @keydown.enter="executeCommand"
          >
        </div>
      </div>
    </div>

    <div class="help-panel mt-4">
      <div class="card">
        <div class="card-header">
          [ AVAILABLE COMMANDS ]
        </div>
        <div class="card-body">
          <div class="commands-grid">
            <div class="command-item">
              <span class="cmd">help</span>
              <span class="desc">Show available commands</span>
            </div>
            <div class="command-item">
              <span class="cmd">status</span>
              <span class="desc">System status</span>
            </div>
            <div class="command-item">
              <span class="cmd">agents</span>
              <span class="desc">List connected agents</span>
            </div>
            <div class="command-item">
              <span class="cmd">memory</span>
              <span class="desc">View memory stats</span>
            </div>
            <div class="command-item">
              <span class="cmd">tasks</span>
              <span class="desc">Active tasks</span>
            </div>
            <div class="command-item">
              <span class="cmd">clear</span>
              <span class="desc">Clear terminal</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, nextTick } from 'vue'

const history = ref([
  { command: 'welcome', output: 'Welcome to Housaky Terminal v1.0.0\nType "help" for available commands.' },
])

const currentInput = ref('')
const terminalBody = ref(null)
const inputRef = ref(null)

const commands = {
  help: () => `Available commands:
  help     - Show this help
  status   - Display system status
  agents   - List connected agents
  memory   - View memory statistics
  tasks    - Show active tasks
  clear    - Clear terminal
  whoami   - Current user info
  uptime   - System uptime`,
  
  status: () => `System Status:
  Singularity: 0.1%
  Active Agents: 5
  Memory Used: 48.2 MB
  Tasks Running: 12
  Uptime: ${new Date().toISOString().substr(11, 8)}`,
  
  agents: () => `Connected Agents:
  [ONLINE] AGENT-001 - Kowalski
  [ONLINE] AGENT-002 - DeepThink
  [ONLINE] AGENT-003 - MemoryCore
  [IDLE]   AGENT-004 - SkillMaster
  [BUSY]   AGENT-005 - ResearchBot`,
  
  memory: () => `Memory Statistics:
  SQLite Backend: Connected
  Lucid Backend: Active
  Embeddings: 12,450 vectors
  Context Chunks: 847
  Semantic Search: Enabled`,
  
  tasks: () => `Active Tasks:
  [1] Self-awareness training - 80%
  [2] Memory federation - 60%
  [3] Research analysis - 45%
  [4] Skill optimization - 30%`,
  
  whoami: () => 'housaky@agi',
  
  uptime: () => `System uptime: ${new Date().toISOString().substr(11, 8)}`,
  
  clear: () => {
    history.value = []
    return null
  }
}

function executeCommand() {
  const cmd = currentInput.value.trim()
  if (!cmd) return
  
  let output = null
  
  if (commands[cmd]) {
    output = commands[cmd]()
  } else {
    output = `Command not found: ${cmd}\nType "help" for available commands.`
  }
  
  if (output) {
    history.value.push({ command: cmd, output })
  } else {
    history.value.push({ command: cmd, output: null })
  }
  
  currentInput.value = ''
  
  nextTick(() => {
    if (terminalBody.value) {
      terminalBody.value.scrollTop = terminalBody.value.scrollHeight
    }
  })
}

onMounted(() => {
  if (inputRef.value) {
    inputRef.value.focus()
  }
})
</script>

<style scoped>
.terminal-view {
  max-width: 1200px;
  margin: 0 auto;
}

.terminal-container {
  background: var(--bg);
  border: 1px solid var(--border);
  font-family: 'Courier New', Monaco, Consolas, monospace;
}

.terminal-header-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  background: var(--bg-tertiary);
  border-bottom: 1px solid var(--border);
}

.terminal-title {
  font-size: 11px;
  color: var(--text-secondary);
}

.terminal-controls {
  display: flex;
  gap: 8px;
}

.control {
  font-size: 12px;
  color: var(--text-muted);
  cursor: pointer;
}

.terminal-body {
  padding: 15px;
  min-height: 400px;
  max-height: 500px;
  overflow-y: auto;
  font-size: 12px;
}

.terminal-line {
  margin-bottom: 8px;
}

.prompt {
  color: var(--success);
  margin-right: 8px;
}

.command {
  color: var(--text-primary);
}

.output {
  margin-top: 4px;
  margin-left: 20px;
  color: var(--text-secondary);
  white-space: pre-wrap;
}

.input-line {
  display: flex;
  align-items: center;
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

.help-panel {
  margin-top: 20px;
}

.commands-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}

.command-item {
  display: flex;
  flex-direction: column;
  padding: 8px;
  border: 1px solid var(--border);
}

.command-item .cmd {
  color: var(--text-primary);
  font-weight: bold;
}

.command-item .desc {
  font-size: 10px;
  color: var(--text-muted);
}

.mt-4 {
  margin-top: 20px;
}

@media (max-width: 768px) {
  .commands-grid {
    grid-template-columns: 1fr;
  }
}
</style>
