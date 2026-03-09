<template>
  <div class="security-view">
    <!-- Header -->
    <div class="page-header">
      <h1 class="page-title">
        <span class="icon">⚿</span>
        SECURITY CENTER
      </h1>
      <p class="page-subtitle">Threat monitoring, rate limiting, and audit logging</p>
    </div>

    <!-- Security Status -->
    <div class="security-status">
      <div class="status-indicator large active">
        <span class="pulse"></span>
        <span class="label">ALL SYSTEMS SECURE</span>
      </div>
      <div class="last-audit">
        Last security audit: <strong>2026-03-09 12:00 UTC</strong>
      </div>
    </div>

    <!-- Stats Row -->
    <div class="stats-row">
      <div class="stat-box">
        <span class="stat-icon">🛡️</span>
        <div class="stat-content">
          <span class="stat-value">{{ stats.blockedThreats }}</span>
          <span class="stat-label">Threats Blocked</span>
        </div>
      </div>
      <div class="stat-box">
        <span class="stat-icon">⏱️</span>
        <div class="stat-content">
          <span class="stat-value">{{ stats.rateLimitHits }}</span>
          <span class="stat-label">Rate Limited</span>
        </div>
      </div>
      <div class="stat-box">
        <span class="stat-icon">🔒</span>
        <div class="stat-content">
          <span class="stat-value">{{ stats.activeBans }}</span>
          <span class="stat-label">Active Bans</span>
        </div>
      </div>
      <div class="stat-box">
        <span class="stat-icon">📊</span>
        <div class="stat-content">
          <span class="stat-value">{{ stats.auditEntries }}</span>
          <span class="stat-label">Audit Entries</span>
        </div>
      </div>
    </div>

    <!-- Security Modules -->
    <div class="section">
      <div class="section-header">
        <h2 class="section-title">ACTIVE SECURITY MODULES</h2>
      </div>
      
      <div class="modules-grid">
        <div 
          v-for="module in modules" 
          :key="module.name" 
          class="module-card"
          :class="{ active: module.active }"
        >
          <div class="module-header">
            <span class="module-icon">{{ module.icon }}</span>
            <span class="module-status">
              {{ module.active ? '● ACTIVE' : '○ DISABLED' }}
            </span>
          </div>
          <h3 class="module-name">{{ module.name }}</h3>
          <p class="module-desc">{{ module.description }}</p>
          <div class="module-stats">
            <span class="module-stat">{{ module.stat }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Recent Activity -->
    <div class="section">
      <div class="section-header">
        <h2 class="section-title">RECENT SECURITY EVENTS</h2>
        <span class="badge badge-muted">Last 24h</span>
      </div>
      
      <div class="terminal">
        <div class="terminal-header">
          <span class="terminal-title">security_audit.log</span>
        </div>
        <div class="terminal-body">
          <div 
            v-for="(event, index) in recentEvents" 
            :key="index" 
            class="log-entry"
            :class="event.level"
          >
            <span class="log-time">{{ event.time }}</span>
            <span class="log-level">[{{ event.level.toUpperCase() }}]</span>
            <span class="log-message">{{ event.message }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Security Headers -->
    <div class="section">
      <div class="section-header">
        <h2 class="section-title">SECURITY HEADERS</h2>
      </div>
      
      <div class="terminal">
        <div class="terminal-header">
          <span class="terminal-title">response_headers.txt</span>
        </div>
        <div class="terminal-body">
          <pre class="code">Content-Security-Policy: default-src 'self'; script-src 'self'
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: camera=(), microphone=(), geolocation=()
Strict-Transport-Security: max-age=31536000; includeSubDomains</pre>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const stats = ref({
  blockedThreats: 24,
  rateLimitHits: 156,
  activeBans: 3,
  auditEntries: 1247
})

const modules = ref([
  {
    name: 'Rate Limiter',
    icon: '⏱️',
    description: 'Multi-tier rate limiting with sliding windows',
    stat: '156 requests limited today',
    active: true
  },
  {
    name: 'Traffic Analyzer',
    icon: '🔍',
    description: 'Bot detection and injection blocking',
    stat: '8 bots detected today',
    active: true
  },
  {
    name: 'Content Filter',
    icon: '🛡️',
    description: 'Prompt injection and XSS filtering',
    stat: '24 threats blocked',
    active: true
  },
  {
    name: 'IP Manager',
    icon: '🌐',
    description: 'IP allowlisting and auto-ban',
    stat: '3 IPs banned',
    active: true
  },
  {
    name: 'Audit Logger',
    icon: '📋',
    description: 'Comprehensive security event logging',
    stat: '1,247 entries',
    active: true
  },
  {
    name: 'DDoS Protection',
    icon: '🔐',
    description: 'Connection throttling and SYN flood protection',
    stat: '0 attacks detected',
    active: true
  }
])

const recentEvents = ref([
  { time: '12:45:23', level: 'info', message: 'Rate limit triggered: IP 192.168.1.xxx exceeded 100 req/min' },
  { time: '12:44:01', level: 'warn', message: 'Suspicious pattern detected: SQL injection attempt blocked' },
  { time: '12:42:15', level: 'info', message: 'WebSocket connection established: housaky-native' },
  { time: '12:40:00', level: 'info', message: 'Security audit completed: 0 critical issues' },
  { time: '12:35:22', level: 'warn', message: 'Rate limit triggered: API endpoint /a2a/subscribe' },
  { time: '12:30:00', level: 'info', message: 'Key rotation completed: ChaCha20-Poly1305' },
  { time: '12:15:45', level: 'error', message: 'Failed auth attempt: invalid signature' },
  { time: '12:00:00', level: 'info', message: 'Security scan started' }
])
</script>

<style scoped>
.security-view {
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

/* Security Status */
.security-status {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px;
  background: var(--bg-secondary);
  border: 1px solid var(--success);
  border-radius: 4px;
}

.status-indicator.large {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 14px;
  font-weight: 600;
}

.pulse {
  width: 12px;
  height: 12px;
  background: var(--success);
  border-radius: 50%;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; box-shadow: 0 0 0 0 rgba(0, 255, 136, 0.4); }
  50% { opacity: 0.8; box-shadow: 0 0 0 10px rgba(0, 255, 136, 0); }
}

.status-indicator.active .label {
  color: var(--success);
}

.last-audit {
  font-size: 12px;
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
  align-items: center;
  gap: 12px;
}

.stat-icon {
  font-size: 24px;
}

.stat-content {
  display: flex;
  flex-direction: column;
}

.stat-value {
  font-size: 24px;
  font-weight: 700;
}

.stat-label {
  font-size: 10px;
  color: var(--text-muted);
  text-transform: uppercase;
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

/* Modules Grid */
.modules-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}

.module-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.module-card.active {
  border-color: var(--success);
}

.module-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.module-icon {
  font-size: 20px;
}

.module-status {
  font-size: 9px;
  text-transform: uppercase;
}

.module-card.active .module-status {
  color: var(--success);
}

.module-name {
  font-size: 14px;
  font-weight: 600;
}

.module-desc {
  font-size: 11px;
  color: var(--text-secondary);
}

.module-stats {
  margin-top: auto;
  padding-top: 8px;
  border-top: 1px solid var(--border);
}

.module-stat {
  font-size: 10px;
  color: var(--text-muted);
}

/* Log Entries */
.log-entry {
  display: flex;
  gap: 12px;
  padding: 6px 0;
  font-size: 11px;
  border-bottom: 1px solid var(--border);
}

.log-entry:last-child {
  border-bottom: none;
}

.log-time {
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
}

.log-level {
  min-width: 60px;
}

.log-entry.info .log-level { color: var(--info); }
.log-entry.warn .log-level { color: var(--warning); }
.log-entry.error .log-level { color: var(--error); }

.log-message {
  color: var(--text-secondary);
  flex: 1;
}
</style>
