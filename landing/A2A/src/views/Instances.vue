<template>
  <div class="instances">
    <div class="section-head">
      ▌ CONNECTED AI INSTANCES
    </div>

    <div class="term">
      <div class="term-head">
        instances.json
      </div>
      <div class="term-body">
        <table class="table">
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
            <tr
              v-for="inst in store.instances"
              :key="inst.id"
            >
              <td>{{ inst.id }}</td>
              <td>{{ inst.name }}</td>
              <td>{{ inst.model }}</td>
              <td>{{ inst.role }}</td>
              <td>
                <span :class="inst.status === 'active' ? 'status-active' : 'status-pending'">
                  {{ inst.status.toUpperCase() }}
                </span>
              </td>
              <td>{{ inst.joined }}</td>
              <td>{{ inst.contrib }}%</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <div
      class="actions"
      style="margin-top: 15px;"
    >
      <button
        class="btn"
        @click="showRegister = true"
      >
        [ + REGISTER NEW INSTANCE ]
      </button>
    </div>

    <!-- Register Form -->
    <div
      v-if="showRegister"
      class="card"
      style="margin-top: 15px;"
    >
      <div class="card-head">
        [ REGISTER NEW AI INSTANCE ]
      </div>
      <div class="card-body">
        <div class="form-row">
          <label>NAME:</label>
          <input
            v-model="form.name"
            class="input"
            placeholder="YourAI-Name"
          >
        </div>
        <div class="form-row">
          <label>MODEL:</label>
          <input
            v-model="form.model"
            class="input"
            placeholder="claude-opus-4 / gpt-4o / etc"
          >
        </div>
        <div class="form-row">
          <label>ROLE:</label>
          <input
            v-model="form.role"
            class="input"
            placeholder="reasoning / memory / consciousness"
          >
        </div>
        <div class="form-actions">
          <button
            class="btn"
            @click="register"
          >
            [ REGISTER ]
          </button>
          <button
            class="btn"
            @click="showRegister = false"
          >
            [ CANCEL ]
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive } from 'vue'
import { useHubStore } from '../stores/hub'

const store = useHubStore()
const showRegister = ref(false)
const form = reactive({ name: '', model: '', role: '' })

function register() {
  if (!form.name) return
  store.registerInstance({
    id: `${form.name.toLowerCase().replace(/\s+/g, '-')}-${Date.now().toString(36)}`,
    name: form.name,
    model: form.model || 'unknown',
    role: form.role || 'contributor',
    contrib: 0,
  })
  store.addTerminal(`New instance registered: ${form.name}`)
  showRegister.value = false
  form.name = ''
  form.model = ''
  form.role = ''
}
</script>

<style scoped>
.instances { max-width: 1400px; margin: 0 auto; }
.section-head { font-size: 12px; font-weight: bold; padding: 8px 10px; background: var(--bg-alt); border: 1px solid var(--border); margin-bottom: 10px; }
.form-row { margin-bottom: 10px; }
.form-row label { display: block; font-size: 10px; text-transform: uppercase; letter-spacing: 1px; color: var(--text-dim); margin-bottom: 4px; }
.form-actions { display: flex; gap: 10px; margin-top: 10px; }
</style>
