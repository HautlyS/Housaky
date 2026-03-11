import { ref, onMounted, onUnmounted } from 'vue'
import { gateway, type HousakyStatus } from '@/lib/gateway'

export function useStatus() {
  const status = ref<HousakyStatus | null>(null)
  const loading = ref(true)
  const error = ref('')
  
  async function load() {
    try {
      loading.value = true
      status.value = await gateway.getStatus()
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }
  
  onMounted(load)
  
  return { status, loading, error, reload: load }
}

export function useAgent() {
  const running = ref(false)
  const loading = ref(false)
  
  async function start() {
    loading.value = true
    try { await gateway.startAgent(); running.value = true }
    finally { loading.value = false }
  }
  
  async function stop() {
    loading.value = true
    try { await gateway.stopAgent(); running.value = false }
    finally { loading.value = false }
  }
  
  return { running, loading, start, stop }
}

export function useEventStream() {
  let es: EventSource | null = null
  const events = ref<any[]>([])
  
  function connect() {
    es = gateway.subscribeToEvents((e) => {
      events.value.unshift(e)
      if (events.value.length > 100) events.value.pop()
    })
  }
  
  function disconnect() {
    es?.close()
    es = null
  }
  
  onMounted(connect)
  onUnmounted(disconnect)
  
  return { events, connect, disconnect }
}

export function useGatewayHealth() {
  const healthy = ref(false)
  const checking = ref(false)
  
  async function check() {
    checking.value = true
    try {
      healthy.value = await gateway.health()
    } finally {
      checking.value = false
    }
  }
  
  onMounted(check)
  
  return { healthy, checking, check }
}
