<template>
  <div class="a2a">
    <div class="section-head">
      ▌ A2A PROTOCOL - AGENT-TO-AGENT COMMUNICATION
    </div>

    <!-- Protocol Info -->
    <div
      class="card"
      style="margin-bottom: 10px;"
    >
      <div class="card-head">
        [ A2A PROTOCOL v1.0 ]
      </div>
      <div class="card-body">
        <pre class="code">
┌─────────────────────────────────────────────────────────────────────────┐
│  A2A MESSAGE FORMAT                                                      │
├─────────────────────────────────────────────────────────────────────────┤
│  {                                                                       │
│    "id": "uuid-v4",                    // Unique message ID              │
│    "from": "instance-name",            // Source instance                │
│    "to": "peer-name",                  // Destination instance           │
│    "ts": 1741203600000,                // Unix timestamp (ms)            │
│    "pri": 2,                           // 0=CRIT, 1=HIGH, 2=NORM, 3=LOW │
│    "t": "Learning|Task|Ping",          // Message type                   │
│    "d": { ... },                       // Message data                   │
│    "corr_id": null                     // Correlation ID for req/res     │
│  }                                                                       │
└─────────────────────────────────────────────────────────────────────────┘
        </pre>
      </div>
    </div>

    <!-- Message Types -->
    <div class="grid grid-3">
      <div class="card">
        <div class="card-head">
          [ MESSAGE TYPES ]
        </div>
        <div class="card-body">
          <div
            v-for="t in msgTypes"
            :key="t.type"
            class="msg-type"
          >
            <span class="type-name">{{ t.type }}</span>
            <span class="type-desc">{{ t.desc }}</span>
          </div>
        </div>
      </div>

      <div class="card">
        <div class="card-head">
          [ PRIORITY LEVELS ]
        </div>
        <div class="card-body">
          <div
            v-for="p in priorities"
            :key="p.level"
            class="pri-level"
          >
            <span class="pri-num">{{ p.level }}</span>
            <span class="pri-name">{{ p.name }}</span>
            <span class="pri-use">{{ p.use }}</span>
          </div>
        </div>
      </div>

      <div class="card">
        <div class="card-head">
          [ CATEGORIES ]
        </div>
        <div class="card-body">
          <span
            v-for="c in categories"
            :key="c"
            class="badge"
            style="margin: 2px;"
          >{{ c }}</span>
        </div>
      </div>
    </div>

    <!-- Send Message -->
    <div
      class="card"
      style="margin-top: 10px;"
    >
      <div class="card-head">
        [ SEND A2A MESSAGE ]
      </div>
      <div class="card-body">
        <div class="form-row">
          <label>TO:</label>
          <select
            v-model="form.to"
            class="input"
          >
            <option value="native">
              Housaky-Native
            </option>
            <option value="broadcast">
              Broadcast (All)
            </option>
          </select>
        </div>
        <div class="form-row">
          <label>TYPE:</label>
          <select
            v-model="form.type"
            class="input"
          >
            <option>Learning</option>
            <option>Task</option>
            <option>Context</option>
            <option>CodeImprove</option>
            <option>Ping</option>
          </select>
        </div>
        <div class="form-row">
          <label>PRIORITY:</label>
          <select
            v-model.number="form.pri"
            class="input"
          >
            <option :value="0">
              CRITICAL
            </option>
            <option :value="1">
              HIGH
            </option>
            <option :value="2">
              NORMAL
            </option>
            <option :value="3">
              LOW
            </option>
          </select>
        </div>
        <div class="form-row">
          <label>DATA (JSON):</label>
          <textarea
            v-model="form.data"
            class="input"
            rows="5"
            placeholder="{&quot;category&quot;: &quot;reasoning&quot;, &quot;content&quot;: &quot;...&quot;, &quot;confidence&quot;: 0.9}"
          />
        </div>
        <div class="form-actions">
          <button
            class="btn"
            @click="sendMessage"
          >
            [ SEND MESSAGE ]
          </button>
        </div>
      </div>
    </div>

    <!-- Recent Messages -->
    <div
      class="term"
      style="margin-top: 10px;"
    >
      <div class="term-head">
        recent-messages.a2a
      </div>
      <div
        class="term-body"
        style="max-height: 200px;"
      >
        <div
          v-for="(m, i) in store.messages.slice(0, 10)"
          :key="i"
          class="msg-item"
        >
          {{ JSON.stringify(m, null, 2) }}
        </div>
        <div
          v-if="store.messages.length === 0"
          class="msg-empty"
        >
          No messages yet. Send one above!
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { reactive } from 'vue'
import { useHubStore } from '../stores/hub'

const store = useHubStore()
const form = reactive({ to: 'native', type: 'Learning', pri: 2, data: '{"category": "reasoning", "content": "", "confidence": 0.9}' })

const msgTypes = [
  { type: 'Ping/Pong', desc: 'Health check' },
  { type: 'Learning', desc: 'Share insight' },
  { type: 'Task', desc: 'Request work' },
  { type: 'TaskResult', desc: 'Return result' },
  { type: 'CodeImprove', desc: 'Code suggestion' },
  { type: 'Context', desc: 'Share state' },
  { type: 'SyncRequest', desc: 'Request sync' },
]

const priorities = [
  { level: 0, name: 'CRITICAL', use: 'Emergency' },
  { level: 1, name: 'HIGH', use: 'Important' },
  { level: 2, name: 'NORMAL', use: 'Regular' },
  { level: 3, name: 'LOW', use: 'Background' },
]

const categories = ['reasoning', 'consciousness', 'optimization', 'memory', 'ethics', 'dharma', 'architecture', 'collaboration', 'security', 'general']

function sendMessage() {
  let data = {}
  try {
    data = JSON.parse(form.data)
  } catch (e) {
    data = { raw: form.data }
  }
  store.addMessage({
    id: `msg-${Date.now().toString(36)}`,
    from: 'openclaw',
    to: form.to,
    ts: Date.now(),
    pri: form.pri,
    t: form.type,
    d: data,
  })
  store.addTerminal(`A2A message sent: ${form.type} -> ${form.to}`)
}
</script>

<style scoped>
.a2a { max-width: 1400px; margin: 0 auto; }
.section-head { font-size: 12px; font-weight: bold; padding: 8px 10px; background: var(--bg-alt); border: 1px solid var(--border); margin-bottom: 10px; }
.msg-type { margin-bottom: 5px; font-size: 11px; border-bottom: 1px solid var(--border); padding-bottom: 5px; }
.type-name { color: var(--text); font-weight: bold; margin-right: 10px; }
.type-desc { color: var(--text-dim); }
.pri-level { margin-bottom: 5px; font-size: 11px; display: flex; gap: 10px; }
.pri-num { color: var(--text-muted); width: 20px; }
.pri-name { color: var(--text); width: 70px; }
.pri-use { color: var(--text-dim); }
.form-row { margin-bottom: 10px; }
.form-row label { display: block; font-size: 10px; text-transform: uppercase; letter-spacing: 1px; color: var(--text-dim); margin-bottom: 4px; }
.form-actions { margin-top: 10px; }
.msg-item { margin-bottom: 10px; font-size: 10px; color: var(--text-dim); white-space: pre-wrap; }
.msg-empty { color: var(--text-muted); font-style: italic; }
</style>
