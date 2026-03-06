<template>
  <div class="security-view">
    <!-- Header -->
    <div class="panel">
      <div class="panel-header">
        <span class="title">[ SECURITY DASHBOARD ]</span>
        <span
          class="status"
          :class="{ active: store.security.activeThreats === 0 }"
        >
          {{ store.security.activeThreats === 0 ? '● SECURE' : '⚠ THREATS DETECTED' }}
        </span>
      </div>
    </div>

    <!-- Stats Grid -->
    <div class="stats-grid">
      <div class="stat-box">
        <div class="stat-value">
          {{ store.security.blockedIPs }}
        </div>
        <div class="stat-label">
          BLOCKED IPS
        </div>
      </div>
      <div class="stat-box">
        <div class="stat-value">
          {{ store.security.activeThreats }}
        </div>
        <div class="stat-label">
          ACTIVE THREATS
        </div>
      </div>
      <div class="stat-box">
        <div class="stat-value">
          {{ store.security.captchaPassed }}
        </div>
        <div class="stat-label">
          CAPTCHA PASSED
        </div>
      </div>
      <div class="stat-box">
        <div class="stat-value">
          {{ store.security.captchaFailed }}
        </div>
        <div class="stat-label">
          CAPTCHA FAILED
        </div>
      </div>
      <div class="stat-box">
        <div class="stat-value">
          {{ store.security.spamBlocked }}
        </div>
        <div class="stat-label">
          SPAM BLOCKED
        </div>
      </div>
      <div class="stat-box">
        <div class="stat-value">
          {{ lastAttackTime }}
        </div>
        <div class="stat-label">
          LAST ATTACK
        </div>
      </div>
    </div>

    <!-- Protection Layers -->
    <div class="panel">
      <div class="panel-header">
        <span>PROTECTION LAYERS</span>
      </div>
      <div class="protection-layers">
        <div
          v-for="layer in protectionLayers"
          :key="layer.name"
          class="layer"
        >
          <span class="layer-name">{{ layer.name }}</span>
          <span
            class="layer-status"
            :class="{ active: layer.active }"
          >
            {{ layer.active ? '● ACTIVE' : '○ INACTIVE' }}
          </span>
          <div class="layer-bar">
            <div
              class="bar-fill"
              :style="{ width: layer.effectiveness + '%' }"
            />
          </div>
          <span class="layer-stat">{{ layer.effectiveness }}%</span>
        </div>
      </div>
    </div>

    <!-- Recent Threats -->
    <div class="panel">
      <div class="panel-header">
        <span>RECENT THREATS</span>
        <button
          class="btn btn-sm"
          @click="refreshThreats"
        >
          [REFRESH]
        </button>
      </div>
      <div class="threats-list">
        <div
          v-for="threat in recentThreats"
          :key="threat.id"
          class="threat"
        >
          <span class="threat-type">{{ threat.type }}</span>
          <span class="threat-ip">{{ threat.ip }}</span>
          <span class="threat-time">{{ formatTime(threat.ts) }}</span>
          <span
            class="threat-action"
            :class="threat.action"
          >{{ threat.action }}</span>
        </div>
        <div
          v-if="recentThreats.length === 0"
          class="empty"
        >
          No threats detected
        </div>
      </div>
    </div>

    <!-- IP Management -->
    <div class="panel">
      <div class="panel-header">
        <span>IP MANAGEMENT</span>
      </div>
      <div class="ip-form">
        <input 
          v-model="ipInput" 
          placeholder="Enter IP address" 
          class="input"
          @keyup.enter="manageIP"
        >
        <button
          class="btn"
          @click="blockIP"
        >
          [BLOCK]
        </button>
        <button
          class="btn"
          @click="whitelistIP"
        >
          [WHITELIST]
        </button>
      </div>
    </div>

    <!-- Actions -->
    <div class="panel">
      <div class="panel-header">
        <span>ACTIONS</span>
      </div>
      <div class="actions">
        <button
          class="btn"
          @click="purgeExpired"
        >
          [PURGE EXPIRED]
        </button>
        <button
          class="btn"
          @click="exportLogs"
        >
          [EXPORT LOGS]
        </button>
        <button
          class="btn btn-danger"
          @click="resetCounters"
        >
          [RESET COUNTERS]
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useHubStore } from '../stores/hub'

const store = useHubStore()
const ipInput = ref('')
const recentThreats = ref([])

const protectionLayers = ref([
  { name: 'Rate Limiter', active: true, effectiveness: 95 },
  { name: 'Traffic Analyzer', active: true, effectiveness: 88 },
  { name: 'Content Filter', active: true, effectiveness: 92 },
  { name: 'AI Captcha', active: true, effectiveness: 85 },
  { name: 'IP Reputation', active: false, effectiveness: 0 },
])

const lastAttackTime = computed(() => {
  if (!store.security.lastAttack) return 'N/A'
  return formatTime(store.security.lastAttack.ts)
})

function formatTime(ts) {
  if (!ts) return 'N/A'
  const date = new Date(ts)
  return date.toLocaleTimeString()
}

function refreshThreats() {
  // In production, fetch from API
  recentThreats.value = []
}

function blockIP() {
  if (!ipInput.value) return
  store.blockIP(ipInput.value, 'Manual block')
  ipInput.value = ''
}

function whitelistIP() {
  if (!ipInput.value) return
  // In production, call API
  ipInput.value = ''
}

function manageIP() {
  // Handle enter key
}

function purgeExpired() {
  store.addTerminal('[SECURITY] Purging expired blocks...')
}

function exportLogs() {
  store.addTerminal('[SECURITY] Exporting security logs...')
}

function resetCounters() {
  store.updateSecurityStats({
    blockedIPs: 0,
    activeThreats: 0,
    captchaPassed: 0,
    captchaFailed: 0,
    spamBlocked: 0,
    lastAttack: null,
  })
}
</script>

<style scoped>
.security-view {
  padding: 20px;
}

.panel {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid #333;
  margin-bottom: 15px;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 15px;
  border-bottom: 1px solid #333;
  font-weight: bold;
}

.title {
  font-size: 14px;
}

.status {
  font-size: 12px;
}

.status.active {
  color: #0f0;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 10px;
  margin-bottom: 15px;
}

.stat-box {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid #333;
  padding: 15px;
  text-align: center;
}

.stat-value {
  font-size: 28px;
  font-weight: bold;
  color: #fff;
}

.stat-label {
  font-size: 10px;
  color: #888;
  margin-top: 5px;
}

.protection-layers {
  padding: 15px;
}

.layer {
  display: grid;
  grid-template-columns: 150px 100px 1fr 50px;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.layer-name {
  font-size: 12px;
}

.layer-status {
  font-size: 10px;
}

.layer-status.active {
  color: #0f0;
}

.layer-bar {
  height: 8px;
  background: #222;
  border-radius: 4px;
  overflow: hidden;
}

.bar-fill {
  height: 100%;
  background: linear-gradient(90deg, #0f0, #0a0);
  transition: width 0.3s;
}

.layer-stat {
  font-size: 11px;
  color: #888;
}

.threats-list {
  padding: 15px;
  max-height: 200px;
  overflow-y: auto;
}

.threat {
  display: grid;
  grid-template-columns: 120px 1fr 80px 80px;
  gap: 10px;
  padding: 8px 0;
  border-bottom: 1px solid #222;
  font-size: 12px;
}

.threat-type {
  color: #f55;
}

.threat-action.BLOCKED {
  color: #f80;
}

.threat-action.BANNED {
  color: #f00;
}

.empty {
  text-align: center;
  color: #666;
  padding: 20px;
}

.ip-form {
  padding: 15px;
  display: flex;
  gap: 10px;
}

.input {
  flex: 1;
  background: #111;
  border: 1px solid #333;
  color: #fff;
  padding: 8px 12px;
  font-family: monospace;
}

.input:focus {
  outline: none;
  border-color: #0f0;
}

.actions {
  padding: 15px;
  display: flex;
  gap: 10px;
}

.btn {
  background: transparent;
  border: 1px solid #0f0;
  color: #0f0;
  padding: 6px 12px;
  cursor: pointer;
  font-family: monospace;
  font-size: 12px;
}

.btn:hover {
  background: #0f0;
  color: #000;
}

.btn-sm {
  padding: 4px 8px;
  font-size: 10px;
}

.btn-danger {
  border-color: #f00;
  color: #f00;
}

.btn-danger:hover {
  background: #f00;
  color: #000;
}
</style>
