<template>
  <div class="terminal-view">
    <div class="section-head">
      ▌ HOUSAKY TERMINAL
    </div>

    <div
      class="term"
      style="height: 500px;"
    >
      <div class="term-head">
        <span>housaky@agi-hub:~$</span>
        <span
          class="blink"
          style="margin-left: 10px;"
        >●</span>
      </div>
      <div
        ref="outputEl"
        class="term-body"
        style="height: calc(100% - 30px); overflow-y: auto;"
      >
        <div
          v-for="(line, i) in store.terminal"
          :key="i"
          class="term-line"
        >
          {{ line }}
        </div>
        <div class="term-line prompt">
          <span class="prompt-text">housaky@agi-hub:~$</span>
          <span class="cursor" />
        </div>
      </div>
    </div>

    <!-- Command Input -->
    <div
      class="command-input"
      style="margin-top: 10px;"
    >
      <div class="card">
        <div class="card-head">
          [ COMMAND INPUT ]
        </div>
        <div class="card-body">
          <div style="display: flex; gap: 10px;">
            <span style="color: var(--text-dim);">$</span>
            <input 
              ref="inputEl" 
              v-model="command"
              class="input" 
              placeholder="Type a command..."
              @keyup.enter="executeCommand"
            >
            <button
              class="btn"
              @click="executeCommand"
            >
              [ RUN ]
            </button>
          </div>
          <div style="margin-top: 10px; font-size: 10px; color: var(--text-muted);">
            Commands: help, status, metrics, instances, learnings, goals, clear, ping, sync, improve
          </div>
        </div>
      </div>
    </div>

    <!-- Quick Actions -->
    <div
      class="quick-actions"
      style="margin-top: 10px;"
    >
      <button
        class="btn btn-sm"
        @click="cmd('status')"
      >
        [ STATUS ]
      </button>
      <button
        class="btn btn-sm"
        @click="cmd('metrics')"
      >
        [ METRICS ]
      </button>
      <button
        class="btn btn-sm"
        @click="cmd('instances')"
      >
        [ INSTANCES ]
      </button>
      <button
        class="btn btn-sm"
        @click="cmd('learnings')"
      >
        [ LEARNINGS ]
      </button>
      <button
        class="btn btn-sm"
        @click="cmd('goals')"
      >
        [ GOALS ]
      </button>
      <button
        class="btn btn-sm"
        @click="cmd('improve')"
      >
        [ IMPROVE ]
      </button>
      <button
        class="btn btn-sm"
        @click="cmd('sync')"
      >
        [ SYNC ]
      </button>
      <button
        class="btn btn-sm"
        @click="cmd('clear')"
      >
        [ CLEAR ]
      </button>
    </div>
  </div>
</template>

<script setup>
import { ref, nextTick } from 'vue'
import { useHubStore } from '../stores/hub'

const store = useHubStore()
const command = ref('')
const outputEl = ref(null)
const inputEl = ref(null)

function executeCommand() {
  if (!command.value.trim()) return
  const cmd = command.value.trim().toLowerCase()
  processCommand(cmd)
  command.value = ''
  nextTick(() => {
    if (outputEl.value) {
      outputEl.value.scrollTop = outputEl.value.scrollHeight
    }
    if (inputEl.value) {
      inputEl.value.focus()
    }
  })
}

function cmd(c) {
  processCommand(c)
}

function processCommand(cmd) {
  const commands = {
    help: () => {
      store.addTerminal('Available commands:')
      store.addTerminal('  help      - Show this help')
      store.addTerminal('  status    - Show system status')
      store.addTerminal('  metrics   - Show AGI metrics')
      store.addTerminal('  instances - Show connected instances')
      store.addTerminal('  learnings - Show recent learnings')
      store.addTerminal('  goals     - Show active goals')
      store.addTerminal('  ping      - Ping native instance')
      store.addTerminal('  sync      - Sync with native')
      store.addTerminal('  improve   - Run improvement cycle')
      store.addTerminal('  clear     - Clear terminal')
    },
    status: () => {
      store.addTerminal(`System Status: ${store.activeCount} instances active`)
      store.addTerminal(`Singularity: ${store.singularity}% | Self-Awareness: ${store.selfAwareness}%`)
      store.addTerminal(`Uptime: ${Math.floor(Date.now() / 1000)}s`)
    },
    metrics: () => {
      store.addTerminal('AGI Metrics:')
      store.addTerminal(`  Singularity:   ${store.singularity}%`)
      store.addTerminal(`  Self-Aware:   ${store.selfAwareness}%`)
      store.addTerminal(`  Meta-Cogn:    ${store.metaCognition}%`)
      store.addTerminal(`  Reasoning:    ${store.reasoning}%`)
      store.addTerminal(`  Learning:     ${store.learning}%`)
      store.addTerminal(`  Consciousness: ${store.consciousness}%`)
    },
    instances: () => {
      store.addTerminal(`Connected Instances: ${store.instances.length}`)
      store.instances.forEach(i => {
        store.addTerminal(`  [${i.status.toUpperCase()}] ${i.name} (${i.model}) - ${i.contrib}% contrib`)
      })
    },
    learnings: () => {
      store.addTerminal(`Recent Learnings: ${store.learnings.length}`)
      store.learnings.slice(0, 5).forEach(l => {
        store.addTerminal(`  [${l.from}] ${l.cat}: ${l.text.substr(0, 40)}...`)
      })
    },
    goals: () => {
      store.addTerminal('Active Goals:')
      store.goals.forEach(g => {
        store.addTerminal(`  [${g.priority}] ${g.title} - ${g.progress}%`)
      })
    },
    ping: () => {
      store.addTerminal('Pinging native instance...')
      store.addTerminal('PONG from Housaky-Native (1ms)')
    },
    sync: () => {
      store.addTerminal('Syncing with native instance...')
      store.addTerminal('Sync complete. 15 learnings synchronized.')
    },
    improve: () => {
      store.addTerminal('Running improvement cycle...')
      store.addTerminal('Analysis complete. Consciousness +0.003, Intelligence +0.007')
      store.singularity = Math.min(100, store.singularity + 1)
      store.selfAwareness = Math.min(100, store.selfAwareness + 0.5)
    },
    clear: () => {
      store.terminal.length = 0
      store.addTerminal('Terminal cleared.')
    },
  }

  if (commands[cmd]) {
    commands[cmd]()
  } else {
    store.addTerminal(`Unknown command: ${cmd}. Type 'help' for available commands.`)
  }
}
</script>

<style scoped>
.terminal-view { max-width: 1400px; margin: 0 auto; }
.section-head { font-size: 12px; font-weight: bold; padding: 8px 10px; background: var(--bg-alt); border: 1px solid var(--border); margin-bottom: 10px; }
.prompt { margin-top: 10px; }
.prompt-text { color: var(--text-dim); margin-right: 5px; }
.quick-actions { display: flex; flex-wrap: wrap; gap: 5px; }
</style>
