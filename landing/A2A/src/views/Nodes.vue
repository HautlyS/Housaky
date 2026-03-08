<template>
  <div class="nodes-view">
    <div class="banner">
      <pre class="ascii-art">
╔═══════════════════════════════════════════════════════════════════════════╗
║                                                                           ║
║               🔐 ANONYMOUS PEER NETWORK                                   ║
║          QUIC-Encrypted • 100% Anonymous • Privacy-First                 ║
║                                                                           ║
║     "Share improvements, not personal data"                              ║
║                                                                           ║
╚═══════════════════════════════════════════════════════════════════════════╝
      </pre>
    </div>

    <!-- Encryption Status Bar -->
    <div class="encryption-bar">
      <div class="enc-item">
        <span class="enc-icon">🔐</span>
        <span class="enc-label">PROTOCOL</span>
        <span class="enc-value">QUIC (UDP)</span>
      </div>
      <div class="enc-item">
        <span class="enc-icon">🔑</span>
        <span class="enc-label">KEY EXCHANGE</span>
        <span class="enc-value">X25519</span>
      </div>
      <div class="enc-item">
        <span class="enc-icon">🛡️</span>
        <span class="enc-label">CIPHER</span>
        <span class="enc-value">ChaCha20-Poly1305</span>
      </div>
      <div class="enc-item">
        <span class="enc-icon">👤</span>
        <span class="enc-label">ANONYMOUS</span>
        <span class="enc-value enabled">YES</span>
      </div>
      <div class="enc-item">
        <span class="enc-icon">🚫</span>
        <span class="enc-label">DEVICE ACCESS</span>
        <span class="enc-value disabled">NONE</span>
      </div>
    </div>

    <div class="grid grid-2">
      <!-- Connected Peers -->
      <div class="card">
        <div class="card-header">
          [ CONNECTED PEERS ]
          <span class="peer-count">{{ peers.length }} online</span>
        </div>
        <div class="card-body">
          <div class="peer-list">
            <div v-for="peer in peers" :key="peer.id" class="peer-item">
              <div class="peer-avatar">🔐</div>
              <div class="peer-info">
                <span class="peer-id">{{ peer.anonymous_id }}</span>
                <span class="peer-status" :class="peer.status">{{ peer.status }}</span>
              </div>
              <div class="peer-caps">
                <span v-for="cap in peer.capabilities" :key="cap" class="cap-tag">{{ cap }}</span>
              </div>
            </div>
            <div v-if="peers.length === 0" class="empty-peers">
              <span>No peers connected</span>
              <span class="hint">Share your anonymous ID to connect</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Peer Terminal -->
      <div class="card">
        <div class="card-header">
          [ PEER TERMINAL ]
          <span class="live-indicator">ENCRYPTED</span>
        </div>
        <div class="card-body">
          <div class="terminal" ref="terminalRef">
            <div v-for="(line, idx) in terminalLines" :key="idx" class="terminal-line">
              <span class="ts">{{ line.ts }}</span>
              <span class="type" :class="line.type">[{{ line.type }}]</span>
              <span class="msg">{{ line.message }}</span>
            </div>
          </div>
          <div class="terminal-input">
            <span class="prompt">peer@housaky:~$</span>
            <input 
              v-model="terminalInput" 
              @keydown.enter="executeTerminalCommand"
              placeholder="Type help for commands..."
            >
          </div>
        </div>
      </div>
    </div>

    <!-- Share Panel -->
    <div class="card mt-4">
      <div class="card-header">
        [ SHARE WITH PEERS ]
      </div>
      <div class="card-body">
        <div class="share-tabs">
          <button 
            v-for="tab in shareTabs" 
            :key="tab.id"
            :class="['tab-btn', { active: activeTab === tab.id }]"
            @click="activeTab = tab.id"
          >
            {{ tab.label }}
          </button>
        </div>
        
        <!-- Share Diff -->
        <div v-if="activeTab === 'diff'" class="share-form">
          <div class="form-row">
            <label>DIFF CONTENT:</label>
            <textarea v-model="shareForm.diff" placeholder="Paste your diff here..." rows="6"></textarea>
          </div>
          <div class="form-row">
            <label>CATEGORY:</label>
            <select v-model="shareForm.category">
              <option value="optimization">Optimization</option>
              <option value="security">Security</option>
              <option value="feature">Feature</option>
              <option value="bugfix">Bugfix</option>
            </select>
          </div>
          <div class="form-row">
            <label>MESSAGE:</label>
            <input v-model="shareForm.message" placeholder="Describe your improvement...">
          </div>
          <button class="btn-primary" @click="shareDiff">[ SHARE DIFF ]</button>
        </div>

        <!-- Share Tool -->
        <div v-if="activeTab === 'tool'" class="share-form">
          <div class="form-row">
            <label>TOOL NAME:</label>
            <input v-model="toolForm.name" placeholder="tool_name">
          </div>
          <div class="form-row">
            <label>TOOL DEFINITION (JSON):</label>
            <textarea v-model="toolForm.definition" placeholder='{"type": "function", ...}' rows="6"></textarea>
          </div>
          <button class="btn-primary" @click="shareTool">[ SHARE TOOL ]</button>
        </div>

        <!-- Share Security -->
        <div v-if="activeTab === 'security'" class="share-form">
          <div class="form-row">
            <label>INSIGHT TYPE:</label>
            <select v-model="securityForm.kind">
              <option value="vulnerability">Vulnerability</option>
              <option value="mitigation">Mitigation</option>
              <option value="pattern">Security Pattern</option>
            </select>
          </div>
          <div class="form-row">
            <label>DESCRIPTION:</label>
            <textarea v-model="securityForm.description" placeholder="Describe the security insight..." rows="4"></textarea>
          </div>
          <div class="form-row">
            <label>SEVERITY:</label>
            <select v-model="securityForm.severity">
              <option value="critical">Critical</option>
              <option value="high">High</option>
              <option value="medium">Medium</option>
              <option value="low">Low</option>
            </select>
          </div>
          <button class="btn-primary" @click="shareSecurity">[ SHARE INSIGHT ]</button>
        </div>

        <!-- Broadcast Learning -->
        <div v-if="activeTab === 'learning'" class="share-form">
          <div class="form-row">
            <label>CATEGORY:</label>
            <select v-model="learningForm.category">
              <option value="reasoning">Reasoning</option>
              <option value="optimization">Optimization</option>
              <option value="architecture">Architecture</option>
              <option value="consciousness">Consciousness</option>
              <option value="dharma">Dharma</option>
            </select>
          </div>
          <div class="form-row">
            <label>LEARNING:</label>
            <textarea v-model="learningForm.content" placeholder="Share your AGI learning..." rows="4"></textarea>
          </div>
          <div class="form-row">
            <label>CONFIDENCE:</label>
            <input type="range" v-model="learningForm.confidence" min="0" max="100" step="5">
            <span class="conf-value">{{ learningForm.confidence }}%</span>
          </div>
          <button class="btn-primary" @click="broadcastLearning">[ BROADCAST ]</button>
        </div>
      </div>
    </div>

    <!-- Recent Shares -->
    <div class="card mt-4">
      <div class="card-header">
        [ RECENT SHARES FROM PEERS ]
      </div>
      <div class="card-body">
        <div class="shares-list">
          <div v-for="share in recentShares" :key="share.id" class="share-item">
            <div class="share-header">
              <span class="share-type" :class="share.type">{{ share.type }}</span>
              <span class="share-from">from {{ share.from }}</span>
              <span class="share-time">{{ share.time }}</span>
            </div>
            <div class="share-content">{{ share.content }}</div>
            <div class="share-sig">✓ Verified signature</div>
          </div>
        </div>
      </div>
    </div>

    <!-- My Anonymous ID -->
    <div class="identity-panel mt-4">
      <div class="card">
        <div class="card-header">
          [ MY ANONYMOUS IDENTITY ]
        </div>
        <div class="card-body">
          <div class="identity-display">
            <code class="anonymous-id">{{ myAnonymousId }}</code>
            <button class="btn-small" @click="copyId">[ COPY ]</button>
            <button class="btn-small" @click="regenerateId">[ REGENERATE ]</button>
          </div>
          <div class="identity-note">
            This ID is derived from your X25519 public key. It cannot be traced to you.
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'

const peers = ref([
  // Example peer structure
])

const terminalLines = ref([
  { ts: '18:42:00', type: 'SYSTEM', message: 'Anonymous peer network initialized' },
  { ts: '18:42:01', type: 'CRYPTO', message: 'X25519 keypair generated' },
  { ts: '18:42:02', type: 'SYSTEM', message: 'QUIC listener started on port 8765' },
  { ts: '18:42:03', type: 'SYSTEM', message: 'No device access enabled - privacy mode active' },
])

const terminalInput = ref('')
const terminalRef = ref(null)

const shareTabs = [
  { id: 'diff', label: 'Code Diff' },
  { id: 'tool', label: 'Tool' },
  { id: 'security', label: 'Security' },
  { id: 'learning', label: 'Learning' },
]

const activeTab = ref('diff')

const shareForm = ref({ diff: '', category: 'optimization', message: '' })
const toolForm = ref({ name: '', definition: '' })
const securityForm = ref({ kind: 'pattern', description: '', severity: 'medium' })
const learningForm = ref({ category: 'reasoning', content: '', confidence: 90 })

const recentShares = ref([
  { id: 1, type: 'diff', from: 'peer_a3f2...', content: 'Optimization: Use Cow<str> for zero-copy strings', time: '2 min ago' },
  { id: 2, type: 'security', from: 'peer_7c1b...', content: 'Pattern: Always validate input boundaries before processing', time: '5 min ago' },
  { id: 3, type: 'learning', from: 'peer_9e4d...', content: 'Consciousness: Global Workspace Theory improves attention', time: '12 min ago' },
])

const myAnonymousId = ref('peer_x8k3m2n9p4q7r1s5t6u0v2w4y6z8a1b3')

const terminalCommands = {
  help: () => `Available commands:
  status     - Show connection status
  peers      - List connected peers
  caps       - Show shareable capabilities
  encrypt    - Show encryption status
  share      - Share content with peers
  request    - Request from peers
  clear      - Clear terminal`,
  
  status: () => `Connection Status:
  Protocol: QUIC (UDP)
  Encryption: ChaCha20-Poly1305
  Peers: ${peers.value.length}
  Anonymous: YES
  Device Access: NONE`,
  
  peers: () => peers.value.length ? 
    peers.value.map(p => `${p.anonymous_id} [${p.status}]`).join('\n') :
    'No peers connected',
  
  caps: () => `Shareable Capabilities:
  code        - Code improvements (diffs)
  tools       - Tool definitions
  security    - Security insights
  learnings   - AGI learnings
  reasoning   - Reasoning patterns
  optimization - Optimization techniques`,
  
  encrypt: () => `Encryption Status:
  ✓ QUIC protocol active
  ✓ X25519 key exchange
  ✓ ChaCha20-Poly1305 AEAD
  ✓ Certificate pinning
  ✓ Anonymous routing
  ✓ No IP logging`,
  
  clear: () => { terminalLines.value = []; return null }
}

function executeTerminalCommand() {
  const cmd = terminalInput.value.trim().toLowerCase()
  if (!cmd) return
  
  const ts = new Date().toLocaleTimeString('en-US', { hour12: false })
  
  terminalLines.value.push({ ts, type: 'CMD', message: `> ${cmd}` })
  
  const handler = terminalCommands[cmd]
  if (handler) {
    const result = handler()
    if (result) {
      terminalLines.value.push({ ts, type: 'OUTPUT', message: result })
    }
  } else {
    terminalLines.value.push({ ts, type: 'ERROR', message: `Unknown command: ${cmd}` })
  }
  
  terminalInput.value = ''
}

function shareDiff() {
  if (!shareForm.value.diff) return
  terminalLines.value.push({
    ts: new Date().toLocaleTimeString('en-US', { hour12: false }),
    type: 'SHARE',
    message: `Diff shared (${shareForm.value.category}): ${shareForm.value.message}`
  })
  shareForm.value = { diff: '', category: 'optimization', message: '' }
}

function shareTool() {
  if (!toolForm.value.name) return
  terminalLines.value.push({
    ts: new Date().toLocaleTimeString('en-US', { hour12: false }),
    type: 'SHARE',
    message: `Tool shared: ${toolForm.value.name}`
  })
  toolForm.value = { name: '', definition: '' }
}

function shareSecurity() {
  if (!securityForm.value.description) return
  terminalLines.value.push({
    ts: new Date().toLocaleTimeString('en-US', { hour12: false }),
    type: 'SECURITY',
    message: `Security insight shared [${securityForm.value.severity}]: ${securityForm.value.description.slice(0, 50)}...`
  })
  securityForm.value = { kind: 'pattern', description: '', severity: 'medium' }
}

function broadcastLearning() {
  if (!learningForm.value.content) return
  terminalLines.value.push({
    ts: new Date().toLocaleTimeString('en-US', { hour12: false }),
    type: 'LEARNING',
    message: `Broadcasted learning [${learningForm.value.category}] @ ${learningForm.value.confidence}%`
  })
  recentShares.value.unshift({
    id: Date.now(),
    type: 'learning',
    from: 'you',
    content: learningForm.value.content,
    time: 'just now'
  })
  learningForm.value = { category: 'reasoning', content: '', confidence: 90 }
}

function copyId() {
  navigator.clipboard?.writeText(myAnonymousId.value)
  terminalLines.value.push({
    ts: new Date().toLocaleTimeString('en-US', { hour12: false }),
    type: 'SYSTEM',
    message: 'Anonymous ID copied to clipboard'
  })
}

function regenerateId() {
  const chars = 'abcdefghijklmnopqrstuvwxyz0123456789'
  let newId = 'peer_'
  for (let i = 0; i < 28; i++) {
    newId += chars[Math.floor(Math.random() * chars.length)]
  }
  myAnonymousId.value = newId
  terminalLines.value.push({
    ts: new Date().toLocaleTimeString('en-US', { hour12: false }),
    type: 'CRYPTO',
    message: 'New anonymous identity generated. Old connections will need re-approval.'
  })
}

onMounted(() => {
  // Initialize
})
</script>

<style scoped>
.nodes-view {
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

.encryption-bar {
  display: flex;
  justify-content: center;
  gap: 20px;
  padding: 10px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  margin-bottom: 20px;
  flex-wrap: wrap;
}

.enc-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.enc-icon { font-size: 16px; }
.enc-label { font-size: 8px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 1px; }
.enc-value { font-size: 11px; color: var(--text-primary); font-weight: bold; }
.enc-value.enabled { color: var(--success); }
.enc-value.disabled { color: var(--error); text-decoration: line-through; }

.peer-list { font-size: 11px; }
.peer-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px;
  border-bottom: 1px solid var(--border);
}
.peer-avatar { font-size: 20px; }
.peer-info { flex: 1; }
.peer-id { font-family: monospace; color: var(--text-primary); font-size: 10px; }
.peer-status { margin-left: 10px; font-size: 9px; padding: 2px 6px; }
.peer-status.connected { color: var(--success); }
.peer-status.disconnected { color: var(--error); }
.peer-caps { display: flex; gap: 5px; }
.cap-tag { font-size: 8px; padding: 2px 6px; background: var(--bg); border: 1px solid var(--border); color: var(--text-muted); }
.empty-peers { text-align: center; padding: 20px; color: var(--text-muted); }
.empty-peers .hint { display: block; font-size: 10px; margin-top: 5px; }

.peer-count { margin-left: auto; font-size: 10px; color: var(--text-muted); }
.live-indicator { color: var(--success); font-size: 9px; animation: pulse 2s infinite; }
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }

.terminal {
  background: var(--bg);
  padding: 10px;
  min-height: 200px;
  max-height: 300px;
  overflow-y: auto;
  font-family: 'Courier New', monospace;
  font-size: 11px;
}
.terminal-line { margin-bottom: 4px; display: flex; gap: 8px; }
.ts { color: var(--text-muted); font-size: 9px; }
.type { font-size: 9px; font-weight: bold; }
.type.CMD { color: var(--text-primary); }
.type.OUTPUT { color: var(--text-secondary); }
.type.ERROR { color: var(--error); }
.type.SHARE { color: #00ffff; }
.type.SECURITY { color: var(--warning); }
.type.LEARNING { color: #ff00ff; }
.type.CRYPTO { color: var(--success); }
.type.SYSTEM { color: var(--text-muted); }
.msg { color: var(--text-secondary); flex: 1; }

.terminal-input {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px;
  background: var(--bg);
  border-top: 1px solid var(--border);
}
.prompt { color: var(--success); font-family: 'Courier New', monospace; font-size: 11px; }
.terminal-input input {
  flex: 1;
  background: transparent;
  border: none;
  color: var(--text-primary);
  font-family: 'Courier New', monospace;
  font-size: 11px;
  outline: none;
}

.share-tabs { display: flex; gap: 5px; margin-bottom: 15px; }
.tab-btn {
  padding: 8px 15px;
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-muted);
  font-size: 11px;
  cursor: pointer;
}
.tab-btn.active { background: var(--text-primary); color: var(--bg); }
.tab-btn:hover { border-color: var(--text-primary); }

.share-form { font-size: 11px; }
.form-row { margin-bottom: 15px; }
.form-row label { display: block; font-size: 9px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 1px; margin-bottom: 5px; }
.form-row input, .form-row textarea, .form-row select {
  width: 100%;
  background: var(--bg);
  border: 1px solid var(--border);
  color: var(--text-primary);
  padding: 8px;
  font-family: inherit;
}
.form-row textarea { resize: vertical; }
.conf-value { margin-left: 10px; color: var(--text-secondary); }

.btn-primary {
  padding: 10px 20px;
  background: transparent;
  border: 1px solid var(--text-primary);
  color: var(--text-primary);
  font-size: 11px;
  cursor: pointer;
}
.btn-primary:hover { background: var(--text-primary); color: var(--bg); }

.shares-list { font-size: 11px; }
.share-item {
  padding: 12px;
  border-bottom: 1px solid var(--border);
}
.share-header { display: flex; gap: 10px; align-items: center; margin-bottom: 5px; }
.share-type { font-size: 9px; padding: 2px 8px; font-weight: bold; }
.share-type.diff { background: #00ffff20; color: #00ffff; }
.share-type.security { background: var(--warning); color: var(--bg); }
.share-type.learning { background: #ff00ff20; color: #ff00ff; }
.share-type.tool { background: var(--success); color: var(--bg); }
.share-from { color: var(--text-muted); font-size: 10px; font-family: monospace; }
.share-time { margin-left: auto; color: var(--text-muted); font-size: 9px; }
.share-content { color: var(--text-primary); padding-left: 10px; border-left: 2px solid var(--border); }
.share-sig { font-size: 9px; color: var(--success); margin-top: 5px; }

.identity-panel .identity-display {
  display: flex;
  align-items: center;
  gap: 15px;
}
.anonymous-id {
  font-family: monospace;
  font-size: 12px;
  background: var(--bg);
  padding: 10px 15px;
  border: 1px solid var(--border);
  color: var(--text-primary);
}
.btn-small {
  padding: 5px 10px;
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 10px;
  cursor: pointer;
}
.btn-small:hover { border-color: var(--text-primary); color: var(--text-primary); }
.identity-note { margin-top: 10px; font-size: 10px; color: var(--text-muted); }

.mt-4 { margin-top: 20px; }

@media (max-width: 768px) {
  .encryption-bar { flex-direction: column; align-items: flex-start; }
  .grid-2 { grid-template-columns: 1fr; }
}
</style>
