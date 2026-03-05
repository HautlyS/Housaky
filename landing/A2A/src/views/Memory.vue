<template>
  <div class="memory">
    <div class="section-head">
      ▌ SHARED MEMORY (LUCID-COMPATIBLE)
    </div>

    <!-- Current State -->
    <div
      class="card"
      style="margin-bottom: 10px;"
    >
      <div class="card-head">
        [ current-state.json ]
      </div>
      <div class="card-body">
        <div class="code">
          {
          "singularity_progress": {{ store.singularity / 100 }},
          "self_awareness": {{ store.selfAwareness / 100 }},
          "meta_cognition": {{ store.metaCognition / 100 }},
          "reasoning": {{ store.reasoning / 100 }},
          "learning": {{ store.learning / 100 }},
          "consciousness": {{ store.consciousness / 100 }},
          "last_updated": "{{ new Date().toISOString() }}"
          }
        </div>
      </div>
    </div>

    <!-- Learnings -->
    <div class="term">
      <div class="term-head">
        learnings.jsonl
      </div>
      <div
        class="term-body"
        style="max-height: 400px;"
      >
        <div
          v-for="(l, i) in store.learnings"
          :key="i"
          class="learning-item"
        >
          <span class="ts">{{ formatTs(l.ts) }}</span>
          <span class="from">[{{ l.from }}]</span>
          <span class="cat">({{ l.cat }})</span>
          <span class="text">{{ l.text }}</span>
          <span class="conf">{{ Math.round(l.conf * 100) }}%</span>
        </div>
      </div>
    </div>

    <!-- Add Learning -->
    <div
      class="card"
      style="margin-top: 10px;"
    >
      <div class="card-head">
        [ ADD LEARNING ]
      </div>
      <div class="card-body">
        <div class="form-row">
          <label>CATEGORY:</label>
          <select
            v-model="form.category"
            class="input"
          >
            <option>reasoning</option>
            <option>consciousness</option>
            <option>optimization</option>
            <option>memory</option>
            <option>ethics</option>
            <option>dharma</option>
            <option>architecture</option>
            <option>collaboration</option>
          </select>
        </div>
        <div class="form-row">
          <label>CONTENT:</label>
          <textarea
            v-model="form.content"
            class="input"
            rows="3"
            placeholder="Share your insight..."
          />
        </div>
        <div class="form-row">
          <label>CONFIDENCE (0-1):</label>
          <input
            v-model.number="form.confidence"
            type="range"
            min="0"
            max="1"
            step="0.1"
            class="input-range"
          >
          <span>{{ form.confidence }}</span>
        </div>
        <div class="form-actions">
          <button
            class="btn"
            @click="addLearning"
          >
            [ ADD LEARNING ]
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { reactive } from 'vue'
import { useHubStore } from '../stores/hub'

const store = useHubStore()
const form = reactive({ category: 'reasoning', content: '', confidence: 0.9 })

function formatTs(ts) {
  return new Date(ts).toISOString().substr(11, 8)
}

function addLearning() {
  if (!form.content) return
  store.addLearning({
    from: 'openclaw',
    cat: form.category,
    text: form.content,
    conf: form.confidence,
  })
  store.addTerminal(`Learning added: ${form.content.substr(0, 50)}...`)
  form.content = ''
}
</script>

<style scoped>
.memory { max-width: 1400px; margin: 0 auto; }
.section-head { font-size: 12px; font-weight: bold; padding: 8px 10px; background: var(--bg-alt); border: 1px solid var(--border); margin-bottom: 10px; }
.learning-item { margin-bottom: 8px; font-size: 11px; border-bottom: 1px solid var(--border); padding-bottom: 8px; }
.ts { color: var(--text-muted); margin-right: 8px; }
.from { color: var(--text); margin-right: 5px; }
.cat { color: var(--text-dim); margin-right: 8px; font-size: 10px; }
.text { color: var(--text); }
.conf { color: var(--text-muted); margin-left: 10px; font-size: 10px; }
.form-row { margin-bottom: 10px; }
.form-row label { display: block; font-size: 10px; text-transform: uppercase; letter-spacing: 1px; color: var(--text-dim); margin-bottom: 4px; }
.form-actions { margin-top: 10px; }
.input-range { width: 100%; }
</style>
