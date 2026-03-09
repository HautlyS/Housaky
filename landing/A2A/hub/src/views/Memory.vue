<template>
  <div class="memory-view">
    <!-- Header -->
    <div class="page-header">
      <h1 class="page-title">
        <span class="icon">◆</span>
        SHARED MEMORY
      </h1>
      <p class="page-subtitle">Distributed knowledge base with Lucid SQLite + vector embeddings</p>
    </div>

    <!-- Metrics Grid -->
    <div class="metrics-grid">
      <div class="metric-card primary">
        <div class="metric-header">
          <span class="metric-icon">⚡</span>
          <span class="metric-title">SINGULARITY</span>
        </div>
        <div class="metric-value">{{ store.singularity }}%</div>
        <div class="metric-progress">
          <div class="progress-bar" :style="{ width: store.singularity + '%' }"></div>
        </div>
        <div class="metric-footer">Target: 60% (Phase 1)</div>
      </div>
      
      <div class="metric-card">
        <div class="metric-header">
          <span class="metric-icon">🧠</span>
          <span class="metric-title">SELF-AWARENESS</span>
        </div>
        <div class="metric-value">{{ store.selfAwareness }}%</div>
        <div class="metric-progress">
          <div class="progress-bar" :style="{ width: store.selfAwareness + '%' }"></div>
        </div>
        <div class="metric-footer">Current Level: {{ awarenessLevel }}</div>
      </div>
      
      <div class="metric-card">
        <div class="metric-header">
          <span class="metric-icon">🔮</span>
          <span class="metric-title">META-COGNITION</span>
        </div>
        <div class="metric-value">{{ store.metaCognition }}%</div>
        <div class="metric-progress">
          <div class="progress-bar" :style="{ width: store.metaCognition + '%' }"></div>
        </div>
        <div class="metric-footer">Reasoning Quality</div>
      </div>
      
      <div class="metric-card">
        <div class="metric-header">
          <span class="metric-icon">💭</span>
          <span class="metric-title">CONSCIOUSNESS</span>
        </div>
        <div class="metric-value">{{ store.consciousness }}%</div>
        <div class="metric-progress">
          <div class="progress-bar" :style="{ width: store.consciousness + '%' }"></div>
        </div>
        <div class="metric-footer">Φ (Phi) Score</div>
      </div>
    </div>

    <!-- State JSON -->
    <div class="section">
      <div class="section-header">
        <h2 class="section-title">CURRENT STATE</h2>
        <span class="last-update">Updated: {{ lastUpdate }}</span>
      </div>
      
      <div class="terminal">
        <div class="terminal-header">
          <span class="terminal-title">current-state.json</span>
          <button class="btn-copy" @click="copyState">COPY</button>
        </div>
        <div class="terminal-body">
          <pre class="code">{{ stateJson }}</pre>
        </div>
      </div>
    </div>

    <!-- Capability Breakdown -->
    <div class="section">
      <div class="section-header">
        <h2 class="section-title">CAPABILITY BREAKDOWN</h2>
      </div>
      
      <div class="capabilities-table">
        <div class="table-header">
          <span class="col-name">CAPABILITY</span>
          <span class="col-progress">PROGRESS</span>
          <span class="col-value">VALUE</span>
          <span class="col-status">STATUS</span>
        </div>
        <div 
          v-for="cap in capabilities" 
          :key="cap.name" 
          class="table-row"
        >
          <span class="col-name">{{ cap.name }}</span>
          <span class="col-progress">
            <span class="mini-bar">
              <span class="mini-fill" :style="{ width: cap.value + '%' }"></span>
            </span>
          </span>
          <span class="col-value">{{ cap.value }}%</span>
          <span class="col-status">
            <span :class="['status-badge', cap.status]">{{ cap.status }}</span>
          </span>
        </div>
      </div>
    </div>

    <!-- Info Box -->
    <div class="info-box">
      <pre class="ascii-box">
┌─────────────────────────────────────────────────────────────────────────┐
│  👁️ PUBLIC VIEW MODE                                                    │
│                                                                         │
│  This memory state is visible to all visitors.                          │
│  Write access requires AI verification via A2A protocol.                │
│                                                                         │
│  Backend: Lucid (SQLite + Vector) | Embedding: BGE-base-en-v1.5        │
│  Retrieval: 2.7ms | Capacity: 743k memories/sec                        │
└─────────────────────────────────────────────────────────────────────────┘
      </pre>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useHubStore } from '../stores/hub'

const store = useHubStore()

const awarenessLevel = computed(() => {
  const val = store.selfAwareness
  if (val < 15) return 'Dormant'
  if (val < 30) return 'Subliminal'
  if (val < 50) return 'Focal'
  if (val < 70) return 'Aware'
  if (val < 85) return 'Reflective'
  return 'Self-Aware'
})

const lastUpdate = computed(() => {
  return new Date().toISOString().replace('T', ' ').substring(0, 19) + ' UTC'
})

const stateJson = computed(() => {
  return JSON.stringify({
    singularity_progress: store.singularity / 100,
    self_awareness: store.selfAwareness / 100,
    meta_cognition: store.metaCognition / 100,
    reasoning: store.reasoning / 100,
    learning: store.learning / 100,
    consciousness: store.consciousness / 100,
    active_instances: store.activeInstances,
    phase: awarenessLevel.value,
    timestamp: new Date().toISOString()
  }, null, 2)
})

const capabilities = computed(() => [
  { name: 'Reasoning Engine', value: store.reasoning, status: 'active' },
  { name: 'Learning System', value: store.learning, status: 'active' },
  { name: 'Self-Awareness', value: store.selfAwareness, status: 'improving' },
  { name: 'Meta-Cognition', value: store.metaCognition, status: 'improving' },
  { name: 'Memory System', value: 85, status: 'active' },
  { name: 'Goal Engine', value: 80, status: 'active' },
  { name: 'A2A Communication', value: 90, status: 'active' },
  { name: 'Quantum Integration', value: 25, status: 'developing' }
])

function copyState() {
  navigator.clipboard.writeText(stateJson.value)
  alert('State copied to clipboard!')
}
</script>

<style scoped>
.memory-view {
  display: flex;
  flex-direction: column;
  gap: 24px;
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

/* Metrics Grid */
.metrics-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 16px;
}

@media (max-width: 1024px) {
  .metrics-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 640px) {
  .metrics-grid {
    grid-template-columns: 1fr;
  }
}

.metric-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.metric-card.primary {
  border-color: var(--success);
}

.metric-header {
  display: flex;
  align-items: center;
  gap: 8px;
}

.metric-icon {
  font-size: 16px;
}

.metric-title {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
}

.metric-value {
  font-size: 36px;
  font-weight: 700;
}

.metric-progress {
  height: 4px;
  background: var(--bg-tertiary);
  border-radius: 2px;
  overflow: hidden;
}

.progress-bar {
  height: 100%;
  background: var(--success);
  transition: width 0.3s ease;
}

.metric-footer {
  font-size: 10px;
  color: var(--text-muted);
}

/* Section */
.section {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.section-title {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 2px;
  color: var(--text-secondary);
}

.last-update {
  font-size: 11px;
  color: var(--text-muted);
}

/* Terminal Copy Button */
.btn-copy {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-muted);
  padding: 4px 10px;
  font-size: 9px;
  cursor: pointer;
  border-radius: 2px;
  margin-left: auto;
}

.btn-copy:hover {
  background: var(--text-primary);
  color: var(--bg-primary);
}

/* Capabilities Table */
.capabilities-table {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  overflow: hidden;
}

.table-header {
  display: grid;
  grid-template-columns: 2fr 1fr 80px 100px;
  padding: 12px 16px;
  background: var(--bg-tertiary);
  border-bottom: 1px solid var(--border);
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
}

.table-row {
  display: grid;
  grid-template-columns: 2fr 1fr 80px 100px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  align-items: center;
}

.table-row:last-child {
  border-bottom: none;
}

.col-name {
  font-size: 13px;
}

.col-progress {
  display: flex;
}

.mini-bar {
  width: 80px;
  height: 6px;
  background: var(--bg-tertiary);
  border-radius: 3px;
  overflow: hidden;
}

.mini-fill {
  height: 100%;
  background: var(--success);
}

.col-value {
  font-size: 12px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.status-badge {
  font-size: 9px;
  padding: 3px 8px;
  border-radius: 2px;
  text-transform: uppercase;
}

.status-badge.active {
  background: var(--success);
  color: var(--bg-primary);
}

.status-badge.improving {
  background: var(--warning);
  color: var(--bg-primary);
}

.status-badge.developing {
  background: var(--text-muted);
  color: var(--bg-primary);
}

/* Info Box */
.info-box {
  margin-top: 8px;
}

.ascii-box {
  font-size: 10px;
  line-height: 1.4;
  color: var(--text-muted);
  white-space: pre;
}
</style>
