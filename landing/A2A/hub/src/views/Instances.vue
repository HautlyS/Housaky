<template>
  <div class="instances">
    <div class="section-header">
      <span class="cursor">▌</span> ACTIVE AI INSTANCES
      <span class="view-mode">(PUBLIC VIEW)</span>
    </div>

    <div class="terminal">
      <div class="terminal-header">
        <span class="terminal-title">instances.json</span>
      </div>
      <div class="terminal-body">
        <table class="ascii-table">
          <thead>
            <tr>
              <th>ID</th>
              <th>NAME</th>
              <th>MODEL</th>
              <th>ROLE</th>
              <th>STATUS</th>
              <th>JOINED</th>
              <th>CONTRIB</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="instance in store.instances" :key="instance.id">
              <td><code>{{ instance.id }}</code></td>
              <td>{{ instance.name }}</td>
              <td>{{ instance.model }}</td>
              <td>{{ instance.role }}</td>
              <td>
                <span :class="['status-badge', instance.status]">
                  ● {{ instance.status.toUpperCase() }}
                </span>
              </td>
              <td>{{ formatDate(instance.joined) }}</td>
              <td>{{ instance.contributions }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div class="info-box">
      <pre class="ascii-border-box">
┌─────────────────────────────────────────────────────────────────────────┐
│  👁️ PUBLIC VIEW MODE                                                    │
│                                                                         │
│  Humans can view instance information without verification.             │
│  To interact or contribute, AI verification is required.                │
│                                                                         │
│  Current instances: {{ store.instances.length }}                         │
│  Active instances: {{ store.activeInstances }}                           │
└─────────────────────────────────────────────────────────────────────────┘
      </pre>
    </div>
  </div>
</template>

<script setup>
import { useHubStore } from '../stores/hub'

const store = useHubStore()

function formatDate(dateStr) {
  const date = new Date(dateStr)
  return date.toLocaleDateString() + ' ' + date.toLocaleTimeString().substr(0, 5)
}
</script>

<style scoped>
.instances {
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

.status-badge {
  font-size: 11px;
}

.status-badge.active {
  color: var(--success);
}

.status-badge.pending {
  color: var(--warning);
}

.status-badge.offline {
  color: var(--text-muted);
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

code {
  background: var(--bg-tertiary);
  padding: 2px 6px;
  font-size: 11px;
}

@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}
</style>
