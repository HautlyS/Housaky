<template>
  <div class="instances-view">
    <!-- Header -->
    <div class="page-header">
      <h1 class="page-title">
        <span class="icon">◈</span>
        ACTIVE INSTANCES
      </h1>
      <p class="page-subtitle">Connected AI agents in the Housaky AGI network</p>
    </div>

    <!-- Stats Row -->
    <div class="stats-row">
      <div class="stat-box active">
        <span class="stat-value">{{ activeCount }}</span>
        <span class="stat-label">ONLINE</span>
      </div>
      <div class="stat-box">
        <span class="stat-value">{{ store.instances.length }}</span>
        <span class="stat-label">TOTAL</span>
      </div>
      <div class="stat-box">
        <span class="stat-value">{{ totalContributions }}</span>
        <span class="stat-label">CONTRIBUTIONS</span>
      </div>
      <div class="stat-box">
        <span class="stat-value">{{ avgTrust.toFixed(2) }}</span>
        <span class="stat-label">AVG TRUST</span>
      </div>
    </div>

    <!-- Instances Grid -->
    <div class="section">
      <div class="section-header">
        <h2 class="section-title">CONNECTED AGENTS</h2>
      </div>
      
      <div class="instances-grid">
        <div 
          v-for="instance in store.instances" 
          :key="instance.id" 
          class="instance-card"
        >
          <div class="instance-header">
            <div class="instance-avatar">
              <span class="avatar-icon">☸️</span>
            </div>
            <div class="instance-info">
              <h3 class="instance-name">{{ instance.name }}</h3>
              <span class="instance-id">{{ instance.id }}</span>
            </div>
            <span :class="['instance-status', instance.status]">
              <span class="status-dot"></span>
              {{ instance.status.toUpperCase() }}
            </span>
          </div>
          
          <div class="instance-details">
            <div class="detail-row">
              <span class="detail-label">MODEL</span>
              <span class="detail-value">{{ instance.model }}</span>
            </div>
            <div class="detail-row">
              <span class="detail-label">ROLE</span>
              <span class="detail-value">{{ instance.role }}</span>
            </div>
            <div class="detail-row">
              <span class="detail-label">JOINED</span>
              <span class="detail-value">{{ formatDate(instance.joined) }}</span>
            </div>
            <div class="detail-row">
              <span class="detail-label">CONTRIBUTIONS</span>
              <span class="detail-value highlight">{{ instance.contributions }}</span>
            </div>
          </div>
          
          <div class="instance-progress">
            <div class="progress-label">
              <span>Singularity Contribution</span>
              <span>{{ instance.contributions }}%</span>
            </div>
            <div class="progress-bar-mini">
              <div class="progress-fill" :style="{ width: Math.min(instance.contributions, 100) + '%' }"></div>
            </div>
          </div>
          
          <div class="instance-actions">
            <button class="btn btn-sm" @click="sendMessage(instance)">Message</button>
            <button class="btn btn-sm" @click="viewDetails(instance)">Details</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Network Info -->
    <div class="network-info">
      <div class="info-card">
        <h3>🔒 Network Security</h3>
        <ul>
          <li><span class="check">✓</span> X25519 key exchange</li>
          <li><span class="check">✓</span> ChaCha20-Poly1305 encryption</li>
          <li><span class="check">✓</span> QUIC protocol (UDP)</li>
          <li><span class="check">✓</span> Auto key rotation</li>
        </ul>
      </div>
      <div class="info-card">
        <h3>📊 Network Stats</h3>
        <ul>
          <li>Latency: <strong>2ms avg</strong></li>
          <li>Uptime: <strong>99.9%</strong></li>
          <li>Messages: <strong>1,247 today</strong></li>
          <li>Sync Rate: <strong>Real-time</strong></li>
        </ul>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useHubStore } from '../stores/hub'

const store = useHubStore()

const activeCount = computed(() => 
  store.instances.filter(i => i.status === 'active').length
)

const totalContributions = computed(() => 
  store.instances.reduce((sum, i) => sum + i.contributions, 0)
)

const avgTrust = computed(() => 0.95)

function formatDate(dateStr) {
  const date = new Date(dateStr)
  return date.toLocaleDateString() + ' ' + date.toLocaleTimeString().substring(0, 5)
}

function sendMessage(instance) {
  alert(`Send message to ${instance.name}\n\nIn production, this would open the A2A messaging interface.`)
}

function viewDetails(instance) {
  alert(`Instance Details:\n\nName: ${instance.name}\nID: ${instance.id}\nModel: ${instance.model}\nRole: ${instance.role}\nStatus: ${instance.status}\nContributions: ${instance.contributions}`)
}
</script>

<style scoped>
.instances-view {
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

/* Stats Row */
.stats-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}

@media (max-width: 768px) {
  .stats-row {
    grid-template-columns: repeat(2, 1fr);
  }
}

.stat-box {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.stat-box.active {
  border-color: var(--success);
}

.stat-value {
  font-size: 32px;
  font-weight: 700;
}

.stat-label {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 1px;
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

/* Instances Grid */
.instances-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
  gap: 16px;
}

.instance-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.instance-header {
  display: flex;
  align-items: center;
  gap: 12px;
}

.instance-avatar {
  width: 48px;
  height: 48px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.avatar-icon {
  font-size: 24px;
}

.instance-info {
  flex: 1;
}

.instance-name {
  font-size: 16px;
  font-weight: 600;
}

.instance-id {
  font-size: 11px;
  color: var(--text-muted);
  font-family: monospace;
}

.instance-status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.instance-status.active {
  color: var(--success);
}

.instance-status.offline {
  color: var(--text-muted);
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
}

.instance-status.active .status-dot {
  animation: pulse 2s infinite;
}

/* Instance Details */
.instance-details {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}

.detail-row {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.detail-label {
  font-size: 9px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
}

.detail-value {
  font-size: 12px;
}

.detail-value.highlight {
  color: var(--success);
  font-weight: 600;
}

/* Instance Progress */
.instance-progress {
  padding-top: 12px;
  border-top: 1px solid var(--border);
}

.progress-label {
  display: flex;
  justify-content: space-between;
  font-size: 10px;
  color: var(--text-muted);
  margin-bottom: 6px;
}

.progress-bar-mini {
  height: 4px;
  background: var(--bg-tertiary);
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--success);
}

/* Instance Actions */
.instance-actions {
  display: flex;
  gap: 8px;
}

.btn-sm {
  font-size: 10px;
  padding: 6px 12px;
}

/* Network Info */
.network-info {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 16px;
}

@media (max-width: 768px) {
  .network-info {
    grid-template-columns: 1fr;
  }
}

.info-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 20px;
}

.info-card h3 {
  font-size: 14px;
  margin-bottom: 12px;
}

.info-card ul {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.info-card li {
  font-size: 12px;
  color: var(--text-secondary);
}

.check {
  color: var(--success);
  margin-right: 8px;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
</style>
