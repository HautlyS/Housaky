<template>
  <div class="nodes-view">
    <!-- Header -->
    <div class="page-header">
      <h1 class="page-title">
        <span class="icon">⬡</span>
        P2P NODES
      </h1>
      <p class="page-subtitle">Anonymous QUIC-encrypted peer communication network</p>
    </div>

    <!-- Stats Row -->
    <div class="stats-row">
      <div class="stat-box">
        <span class="stat-label">CONNECTED PEERS</span>
        <span class="stat-value">{{ connectedPeers }}</span>
      </div>
      <div class="stat-box">
        <span class="stat-label">SHAREABLE CAPABILITIES</span>
        <span class="stat-value">{{ capabilities.length }}</span>
      </div>
      <div class="stat-box">
        <span class="stat-label">DIFFS SHARED</span>
        <span class="stat-value">{{ diffsShared }}</span>
      </div>
      <div class="stat-box">
        <span class="stat-label">ENCRYPTION</span>
        <span class="stat-value status">ChaCha20-Poly1305</span>
      </div>
    </div>

    <!-- Nodes Grid -->
    <div class="section">
      <div class="section-header">
        <h2 class="section-title">ACTIVE PEERS</h2>
        <span class="badge badge-success">{{ connectedPeers }} ONLINE</span>
      </div>
      
      <div class="nodes-grid">
        <div 
          v-for="peer in peers" 
          :key="peer.id" 
          class="node-card"
        >
          <div class="node-header">
            <div class="node-id">{{ peer.anonymousId }}</div>
            <span :class="['node-status', peer.status]">
              ● {{ peer.status.toUpperCase() }}
            </span>
          </div>
          
          <div class="node-capabilities">
            <span 
              v-for="cap in peer.capabilities" 
              :key="cap" 
              class="capability-tag"
            >
              {{ cap }}
            </span>
          </div>
          
          <div class="node-meta">
            <div class="meta-item">
              <span class="meta-label">Last Seen</span>
              <span class="meta-value">{{ peer.lastSeen }}</span>
            </div>
            <div class="meta-item">
              <span class="meta-label">Trust Score</span>
              <span class="meta-value">{{ peer.trustScore }}</span>
            </div>
          </div>
          
          <div class="node-actions">
            <button class="btn btn-sm" @click="requestImprovement(peer)">Request Improvement</button>
            <button class="btn btn-sm" @click="shareWith(peer)">Share Diff</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Capabilities Section -->
    <div class="section">
      <div class="section-header">
        <h2 class="section-title">SHAREABLE CAPABILITIES</h2>
        <span class="section-subtitle">What peers can share - NO device access</span>
      </div>
      
      <div class="capabilities-grid">
        <div 
          v-for="cap in capabilities" 
          :key="cap.name" 
          class="capability-card"
        >
          <div class="capability-icon">{{ cap.icon }}</div>
          <h3 class="capability-name">{{ cap.name }}</h3>
          <p class="capability-desc">{{ cap.description }}</p>
        </div>
      </div>
    </div>

    <!-- Privacy Notice -->
    <div class="notice-box">
      <div class="notice-icon">🔒</div>
      <div class="notice-content">
        <h3>100% Anonymous & Encrypted</h3>
        <p>
          Peer IDs are derived from public keys - not traceable to individuals. 
          QUIC protocol provides encryption by default. No camera, screen, location, 
          or personal data access.
        </p>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const connectedPeers = ref(1)
const diffsShared = ref(12)

const peers = ref([
  {
    id: 1,
    anonymousId: 'housaky-native-7f3a',
    status: 'online',
    capabilities: ['code-improvements', 'tools', 'security', 'learnings'],
    lastSeen: 'Just now',
    trustScore: 0.95
  }
])

const capabilities = ref([
  { name: 'Code Improvements', icon: '📝', description: 'Share diffs and code optimizations' },
  { name: 'Tool Definitions', icon: '🔧', description: 'Share tool capabilities and APIs' },
  { name: 'Security Insights', icon: '🔒', description: 'Vulnerability patterns and mitigations' },
  { name: 'AGI Learnings', icon: '🧠', description: 'Reasoning patterns and improvements' },
  { name: 'Reasoning Patterns', icon: '💭', description: 'Problem-solving strategies' },
  { name: 'Optimizations', icon: '⚡', description: 'Performance techniques' }
])

function requestImprovement(peer) {
  alert(`Request improvement from ${peer.anonymousId}\n\nIn production, this would send a Task message via A2A protocol.`)
}

function shareWith(peer) {
  alert(`Share diff with ${peer.anonymousId}\n\nIn production, this would open a diff sharing dialog.`)
}
</script>

<style scoped>
.nodes-view {
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
  gap: 8px;
}

.stat-label {
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
}

.stat-value {
  font-size: 24px;
  font-weight: 700;
}

.stat-value.status {
  font-size: 11px;
  color: var(--success);
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

.section-subtitle {
  font-size: 11px;
  color: var(--text-muted);
}

/* Nodes Grid */
.nodes-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}

.node-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.node-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.node-id {
  font-size: 12px;
  font-family: monospace;
  color: var(--text-primary);
}

.node-status {
  font-size: 9px;
  text-transform: uppercase;
}

.node-status.online {
  color: var(--success);
}

.node-status.offline {
  color: var(--text-muted);
}

.node-capabilities {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.capability-tag {
  font-size: 9px;
  padding: 3px 8px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 2px;
  color: var(--text-muted);
  text-transform: uppercase;
}

.node-meta {
  display: flex;
  gap: 16px;
  font-size: 11px;
}

.meta-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.meta-label {
  font-size: 9px;
  color: var(--text-muted);
  text-transform: uppercase;
}

.meta-value {
  color: var(--text-secondary);
}

.node-actions {
  display: flex;
  gap: 8px;
  margin-top: auto;
}

.btn-sm {
  font-size: 10px;
  padding: 6px 12px;
}

/* Capabilities Grid */
.capabilities-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.capability-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 16px;
  text-align: center;
}

.capability-icon {
  font-size: 24px;
  margin-bottom: 8px;
}

.capability-name {
  font-size: 12px;
  font-weight: 600;
  margin-bottom: 4px;
}

.capability-desc {
  font-size: 10px;
  color: var(--text-muted);
  line-height: 1.4;
}

/* Notice Box */
.notice-box {
  display: flex;
  gap: 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--success);
  border-radius: 4px;
  padding: 20px;
}

.notice-icon {
  font-size: 32px;
}

.notice-content h3 {
  font-size: 14px;
  margin-bottom: 8px;
  color: var(--success);
}

.notice-content p {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.6;
}
</style>
