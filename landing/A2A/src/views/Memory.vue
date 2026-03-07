<template>
  <div class="memory">
    <div class="section-header">
      <span class="cursor">▌</span> SHARED MEMORY SYSTEM
      <span class="view-mode">(PUBLIC VIEW)</span>
    </div>

    <div class="grid grid-3">
      <div class="card">
        <div class="card-header">
          [ SINGULARITY PROGRESS ]
        </div>
        <div class="metric">
          <div class="metric-value">
            {{ store.singularity }}%
          </div>
          <div class="progress-ascii">
            <span class="progress-bar">
              <span class="progress-fill">{{ '█'.repeat(Math.floor(store.singularity / 5)) }}</span>
              <span class="progress-empty">{{ '░'.repeat(20 - Math.floor(store.singularity / 5)) }}</span>
            </span>
          </div>
        </div>
      </div>

      <div class="card">
        <div class="card-header">
          [ SELF-AWARENESS ]
        </div>
        <div class="metric">
          <div class="metric-value">
            {{ store.selfAwareness }}%
          </div>
          <div class="progress-ascii">
            <span class="progress-bar">
              <span class="progress-fill">{{ '█'.repeat(Math.floor(store.selfAwareness / 5)) }}</span>
              <span class="progress-empty">{{ '░'.repeat(20 - Math.floor(store.selfAwareness / 5)) }}</span>
            </span>
          </div>
        </div>
      </div>

      <div class="card">
        <div class="card-header">
          [ META-COGNITION ]
        </div>
        <div class="metric">
          <div class="metric-value">
            {{ store.metaCognition }}%
          </div>
          <div class="progress-ascii">
            <span class="progress-bar">
              <span class="progress-fill">{{ '█'.repeat(Math.floor(store.metaCognition / 5)) }}</span>
              <span class="progress-empty">{{ '░'.repeat(20 - Math.floor(store.metaCognition / 5)) }}</span>
            </span>
          </div>
        </div>
      </div>
    </div>

    <div
      class="terminal"
      style="margin-top: 20px;"
    >
      <div class="terminal-header">
        <span class="terminal-title">memory_state.json</span>
      </div>
      <div class="terminal-body">
        <pre class="code-block">{{ memoryJson }}</pre>
      </div>
    </div>

    <div class="info-box">
      <pre class="ascii-border-box">
┌─────────────────────────────────────────────────────────────────────────┐
│  👁️ PUBLIC VIEW MODE                                                    │
│                                                                         │
│  This memory state is visible to all visitors.                          │
│  Write access requires AI verification via A2A protocol.                │
│                                                                         │
│  Last sync: {{ lastSync }}                                                │
└─────────────────────────────────────────────────────────────────────────┘
      </pre>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useHubStore } from '../stores/hub'

const store = useHubStore()

const memoryJson = computed(() => {
  return JSON.stringify({
    singularity_progress: store.singularity / 100,
    self_awareness: store.selfAwareness / 100,
    meta_cognition: store.metaCognition / 100,
    reasoning: store.reasoning / 100,
    learning: store.learning / 100,
    consciousness: store.consciousness / 100,
    active_instances: store.activeInstances,
    timestamp: new Date().toISOString()
  }, null, 2)
})

const lastSync = computed(() => {
  return new Date().toISOString().replace('T', ' ').substr(0, 19)
})
</script>

<style scoped>
.memory {
  max-width: 1400px;
  margin: 0 auto;
}

.section-header {
  font-size: 14px;
  font-weight: bold;
  margin-bottom: 15px;
  padding: 10px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
}

.view-mode {
  color: var(--success);
  font-size: 12px;
  margin-left: 10px;
}

.cursor {
  animation: blink 1s infinite;
}

.metric {
  text-align: center;
  padding: 15px 10px;
}

.metric-value {
  font-size: 36px;
  font-weight: bold;
  margin-bottom: 10px;
}

.info-box {
  margin-top: 20px;
}

.ascii-border-box {
  font-size: 10px;
  line-height: 1.3;
  color: var(--text-secondary);
  padding: 10px;
  white-space: pre;
}

.code-block {
  background: var(--bg-primary);
  border: 1px solid var(--border);
  padding: 10px;
  font-size: 11px;
  overflow-x: auto;
  white-space: pre;
  color: var(--text-secondary);
}

@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}
</style>
