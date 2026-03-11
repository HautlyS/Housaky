const GATEWAY_URL = import.meta.env.VITE_GATEWAY_URL || 'http://127.0.0.1:8080'

export class GatewayClient {
  private baseUrl: string
  private token: string | null = null
  
  constructor(baseUrl?: string) {
    this.baseUrl = baseUrl || GATEWAY_URL
  }
  
  setToken(token: string) { this.token = token }
  
  private async fetch<T>(path: string, options?: RequestInit): Promise<T> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    }
    if (this.token) headers['Authorization'] = `Bearer ${this.token}`
    
    const res = await fetch(`${this.baseUrl}${path}`, {
      ...options,
      headers: { ...headers, ...options?.headers },
    })
    
    if (!res.ok) {
      const err = await res.json().catch(() => ({ error: res.statusText }))
      throw new Error(err.error || `HTTP ${res.status}`)
    }
    
    return res.json()
  }
  
  async getStatus() { return this.fetch<HousakyStatus>('/api/status') }
  async startAgent() { return this.fetch('/api/agent/start', { method: 'POST' }) }
  async stopAgent() { return this.fetch('/api/agent/stop', { method: 'POST' }) }
  async getAgentStatus() { return this.fetch('/api/agent/status') }
  
  async getSkills() { return this.fetch<{ skills: Skill[] }>('/api/skills') }
  async toggleSkill(name: string) { return this.fetch(`/api/skills/${name}/toggle`, { method: 'POST' }) }
  async installSkill(name: string) { return this.fetch(`/api/skills/${name}/install`, { method: 'POST' }) }
  async uninstallSkill(name: string) { return this.fetch(`/api/skills/${name}`, { method: 'DELETE' }) }
  
  async getChannels() { return this.fetch<{ channels: Channel[] }>('/api/channels') }
  async startChannel(type: string) { return this.fetch(`/api/channels/${type}/start`, { method: 'POST' }) }
  async stopChannel(type: string) { return this.fetch(`/api/channels/${type}/stop`, { method: 'POST' }) }
  async configChannel(type: string, config: any) { return this.fetch(`/api/channels/${type}/config`, { method: 'PUT', body: JSON.stringify(config) }) }
  
  async getKeys() { return this.fetch<{ keys: Key[] }>('/api/keys') }
  async addKey(key: { provider: string, key: string, label?: string }) { return this.fetch('/api/keys', { method: 'POST', body: JSON.stringify(key) }) }
  async deleteKey(provider: string, keyId: string) { return this.fetch(`/api/keys/${provider}/${keyId}`, { method: 'DELETE' }) }
  
  async getA2AInstances() { return this.fetch<{ instances: A2AInstance[] }>('/api/a2a/instances') }
  async pingA2A(id: string) { return this.fetch(`/api/a2a/${id}/ping`, { method: 'POST' }) }
  async getA2AMessages() { return this.fetch<{ messages: A2AMessage[] }>('/api/a2a/messages') }
  async sendA2AMessage(msg: any) { return this.fetch('/api/a2a/send', { method: 'POST', body: JSON.stringify(msg) }) }
  
  async getHardware() { return this.fetch<{ devices: HardwareDevice[] }>('/api/hardware') }
  
  async runDoctor() { return this.fetch<{ output: string }>('/api/doctor/run', { method: 'POST' }) }
  
  async chat(message: string, options?: { model?: string, session?: string }) {
    return this.fetch('/chat', {
      method: 'POST',
      body: JSON.stringify({ message, ...options })
    })
  }
  
  async health() {
    try {
      const res = await fetch(`${this.baseUrl}/health`)
      return res.ok
    } catch {
      return false
    }
  }
  
  connectWebSocket(onMessage: (data: any) => void): WebSocket {
    const wsUrl = this.baseUrl.replace('http', 'ws') + '/ws'
    const ws = new WebSocket(wsUrl)
    ws.onmessage = (e) => onMessage(JSON.parse(e.data))
    return ws
  }
  
  subscribeToEvents(onEvent: (event: any) => void): EventSource {
    return new EventSource(`${this.baseUrl}/events`)
  }
}

export interface HousakyStatus {
  version: string
  provider: string
  model: string
  temperature: number
  memory_backend: string
  memory_auto_save: boolean
  embedding_provider: string
  autonomy_level: string
  workspace_only: boolean
  channels: Record<string, { configured: boolean; active: boolean; allowlist_count: number }>
  secrets_encrypted: boolean
  heartbeat_enabled: boolean
  heartbeat_interval: number
  agent_running: boolean
  uptime_seconds: number
  workspace: string
  config_path: string
}

export interface Skill { name: string; enabled: boolean; description: string; version: string }
export interface Channel { name: string; type: string; configured: boolean; active: boolean; allowlist_count: number }
export interface Key { id: string; provider: string; label?: string; enabled: boolean; created_at: string }
export interface A2AInstance { id: string; name: string; endpoint: string; status: string; last_seen: string }
export interface A2AMessage { id: string; from: string; content: string; timestamp: string; processed: boolean }
export interface HardwareDevice { id: string; name: string; type: string; connected: boolean }

export const gateway = new GatewayClient()
