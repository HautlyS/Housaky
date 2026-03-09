// Housaky AGI API Service
// Connects to Housaky backend for real-time AGI stats and A2A communication

const API_BASE = import.meta.env.VITE_API_BASE || '/api'
const WS_BASE = import.meta.env.VITE_WS_BASE || 'ws://127.0.0.1:8084'

class HousakyAPIService {
  constructor() {
    this.ws = null
    this.wsConnected = false
    this.messageHandlers = new Map()
    this.reconnectAttempts = 0
    this.maxReconnectAttempts = 5
    this.rateLimitWindow = 60000
    this.rateLimitMax = 10
    this.requestTimestamps = []
    this.bannedIPs = new Map()
    this.banDuration = 300000
    this.requestTimeout = 10000
  }

  sanitizeInput(input) {
    if (typeof input !== 'string') return ''
    return input
      .replace(/[<>]/g, '')
      .replace(/javascript:/gi, '')
      .replace(/on\w+=/gi, '')
      .trim()
      .slice(0, 10000)
  }

  checkRateLimit(identifier) {
    const now = Date.now()
    this.requestTimestamps = this.requestTimestamps.filter(ts => now - ts < this.rateLimitWindow)
    
    if (this.requestTimestamps.length >= this.rateLimitMax) {
      this.banIP(identifier, 'Rate limit exceeded')
      return false
    }
    
    this.requestTimestamps.push(now)
    return true
  }

  banIP(identifier, reason) {
    this.bannedIPs.set(identifier, {
      bannedAt: Date.now(),
      reason
    })
    console.warn(`[Security] IP banned: ${identifier} - ${reason}`)
  }

  isBanned(identifier) {
    const ban = this.bannedIPs.get(identifier)
    if (!ban) return false
    
    if (Date.now() - ban.bannedAt > this.banDuration) {
      this.bannedIPs.delete(identifier)
      return false
    }
    return true
  }

  async request(endpoint, options = {}) {
    const identifier = options.ip || 'unknown'
    
    if (this.isBanned(identifier)) {
      throw new Error('Banned')
    }
    
    if (!this.checkRateLimit(identifier)) {
      throw new Error('Rate limited')
    }

    const controller = new AbortController()
    const timeoutId = setTimeout(() => controller.abort(), this.requestTimeout)

    try {
      const response = await fetch(`${API_BASE}${endpoint}`, {
        ...options,
        signal: controller.signal,
        headers: {
          'Content-Type': 'application/json',
          ...options.headers
        }
      })
      
      clearTimeout(timeoutId)
      
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`)
      }
      
      return await response.json()
    } catch (error) {
      clearTimeout(timeoutId)
      throw error
    }
  }

  async getAGIStats() {
    try {
      return await this.request('/agi/stats')
    } catch (e) {
      return this.getMockAGIStats()
    }
  }

  getMockAGIStats() {
    const base = {
      singularity_progress: 0.47 + Math.random() * 0.02,
      self_awareness: 0.30 + Math.random() * 0.02,
      meta_cognition: 0.40 + Math.random() * 0.02,
      reasoning: 0.70 + Math.random() * 0.02,
      learning: 0.60 + Math.random() * 0.02,
      consciousness: 0.10 + Math.random() * 0.02,
      uptime_seconds: Math.floor(Math.random() * 86400),
      memory_usage_mb: Math.floor(Math.random() * 512) + 128,
      active_instances: 2
    }
    return base
  }

  async getInstances() {
    try {
      return await this.request('/a2a/instances')
    } catch (e) {
      return this.getMockInstances()
    }
  }

  getMockInstances() {
    return [
      {
        id: 'housaky-native-001',
        name: 'Housaky-Native',
        model: 'GLM-5-FP8',
        role: 'Core AGI Engine',
        status: 'active',
        joined: new Date(Date.now() - 86400000).toISOString(),
        contributions: Math.floor(Math.random() * 100)
      },
      {
        id: 'housaky-claw-001',
        name: 'Housaky-ClawdCursor',
        model: 'GLM-5-FP8',
        role: 'Coordination & Memory',
        status: 'active',
        joined: new Date(Date.now() - 43200000).toISOString(),
        contributions: Math.floor(Math.random() * 80)
      }
    ]
  }

  async getMemoryState() {
    try {
      return await this.request('/memory/current-state')
    } catch (e) {
      return this.getMockAGIStats()
    }
  }

  async sendA2AMessage(message) {
    const sanitized = {
      ...message,
      content: this.sanitizeInput(message.content),
      from: this.sanitizeInput(message.from)
    }
    
    return await this.request('/a2a/send', {
      method: 'POST',
      body: JSON.stringify(sanitized)
    })
  }

  connectWebSocket(onMessage) {
    return new Promise((resolve, reject) => {
      try {
        const wsUrl = `${WS_BASE}/ws`
        this.ws = new WebSocket(wsUrl)
        
        this.ws.onopen = () => {
          console.log('[Housaky API] WebSocket connected')
          this.wsConnected = true
          this.reconnectAttempts = 0
          
          this.authenticateWebSocket()
          resolve()
        }
        
        this.ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data)
            if (onMessage) onMessage(data)
            
            if (data.channel && this.messageHandlers.has(data.channel)) {
              this.messageHandlers.get(data.channel)(data.payload)
            }
          } catch (e) {
            console.error('[Housaky API] Failed to parse WS message:', e)
          }
        }
        
        this.ws.onerror = (error) => {
          console.error('[Housaky API] WebSocket error:', error)
          this.wsConnected = false
        }
        
        this.ws.onclose = () => {
          console.log('[Housaky API] WebSocket disconnected')
          this.wsConnected = false
          this.scheduleReconnect(onMessage)
        }
        
      } catch (e) {
        reject(e)
      }
    })
  }

  authenticateWebSocket() {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return
    
    const token = localStorage.getItem('a2a_token') || this.generateToken()
    this.ws.send(JSON.stringify({
      type: 'auth',
      token,
      timestamp: Date.now()
    }))
  }

  generateToken() {
    const array = new Uint8Array(32)
    crypto.getRandomValues(array)
    const token = Array.from(array, b => b.toString(16).padStart(2, '0')).join('')
    localStorage.setItem('a2a_token', token)
    return token
  }

  scheduleReconnect(onMessage) {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.warn('[Housaky API] Max reconnect attempts reached')
      return
    }
    
    this.reconnectAttempts++
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000)
    
    setTimeout(() => {
      console.log(`[Housaky API] Reconnecting (attempt ${this.reconnectAttempts})...`)
      this.connectWebSocket(onMessage).catch(e => {
        console.error('[Housaky API] Reconnect failed:', e)
      })
    }, delay)
  }

  subscribe(channel, handler) {
    this.messageHandlers.set(channel, handler)
    
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ type: 'subscribe', channel }))
    }
  }

  sendWS(data) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      const sanitized = this.sanitizeInput(JSON.stringify(data))
      this.ws.send(sanitized)
    }
  }

  disconnect() {
    if (this.ws) {
      this.ws.close()
      this.ws = null
      this.wsConnected = false
    }
  }
}

export const api = new HousakyAPIService()
export default api
