// ☸️ Secure WebSocket Service for A2A Hub
// Encrypted real-time communication with Housaky Native

import { ref, onUnmounted } from 'vue'

class SecureWebSocket {
  constructor(url, options = {}) {
    this.url = url
    this.options = options
    this.ws = null
    this.reconnectAttempts = 0
    this.maxReconnectAttempts = 5
    this.reconnectDelay = 1000
    this.subscriptions = new Map()
    this.isConnected = ref(false)
    this.lastMessage = ref(null)
    this.error = ref(null)
  }

  connect() {
    return new Promise((resolve, reject) => {
      try {
        // Use wss:// for secure connection
        const wsUrl = this.url.replace('http://', 'ws://').replace('https://', 'wss://')
        
        this.ws = new WebSocket(wsUrl)
        
        this.ws.onopen = () => {
          console.log('[A2A] WebSocket connected')
          this.isConnected.value = true
          this.reconnectAttempts = 0
          this.error.value = null
          
          // Send authentication
          this.authenticate()
          
          // Resubscribe to channels
          this.subscriptions.forEach((callback, channel) => {
            this.subscribe(channel, callback)
          })
          
          resolve()
        }
        
        this.ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data)
            this.lastMessage.value = data
            
            // Route to appropriate subscription
            if (data.channel && this.subscriptions.has(data.channel)) {
              this.subscriptions.get(data.channel)(data.payload)
            }
            
            // Broadcast to all subscribers
            if (this.subscriptions.has('*')) {
              this.subscriptions.get('*')(data)
            }
          } catch (e) {
            console.error('[A2A] Failed to parse message:', e)
          }
        }
        
        this.ws.onerror = (error) => {
          console.error('[A2A] WebSocket error:', error)
          this.error.value = error
        }
        
        this.ws.onclose = () => {
          console.log('[A2A] WebSocket disconnected')
          this.isConnected.value = false
          this.scheduleReconnect()
        }
        
      } catch (e) {
        reject(e)
      }
    })
  }

  authenticate() {
    // Send authentication with secure token
    const token = localStorage.getItem('a2a_token') || this.generateToken()
    this.send({
      type: 'auth',
      token: token,
      timestamp: Date.now()
    })
  }

  generateToken() {
    // Generate a secure random token
    const array = new Uint8Array(32)
    crypto.getRandomValues(array)
    const token = Array.from(array, b => b.toString(16).padStart(2, '0')).join('')
    localStorage.setItem('a2a_token', token)
    return token
  }

  subscribe(channel, callback) {
    this.subscriptions.set(channel, callback)
    
    if (this.ws && this.isConnected.value) {
      this.send({
        type: 'subscribe',
        channel: channel
      })
    }
  }

  unsubscribe(channel) {
    this.subscriptions.delete(channel)
    
    if (this.ws && this.isConnected.value) {
      this.send({
        type: 'unsubscribe',
        channel: channel
      })
    }
  }

  send(data) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({
        ...data,
        timestamp: Date.now()
      }))
    }
  }

  scheduleReconnect() {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++
      const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1)
      
      console.log(`[A2A] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`)
      
      setTimeout(() => {
        this.connect().catch(e => console.error('[A2A] Reconnect failed:', e))
      }, delay)
    }
  }

  disconnect() {
    if (this.ws) {
      this.ws.close()
      this.ws = null
      this.isConnected.value = false
    }
  }
}

// Singleton instance
let wsInstance = null

export function useWebSocket(url = 'ws://127.0.0.1:8084/ws') {
  if (!wsInstance) {
    wsInstance = new SecureWebSocket(url)
  }
  
  return {
    connect: () => wsInstance.connect(),
    disconnect: () => wsInstance.disconnect(),
    subscribe: (channel, callback) => wsInstance.subscribe(channel, callback),
    unsubscribe: (channel) => wsInstance.unsubscribe(channel),
    send: (data) => wsInstance.send(data),
    isConnected: wsInstance.isConnected,
    lastMessage: wsInstance.lastMessage,
    error: wsInstance.error
  }
}

// Composable for Vue components
export function useA2AConnection() {
  const { connect, disconnect, subscribe, send, isConnected, lastMessage, error } = useWebSocket()
  
  onUnmounted(() => {
    // Don't disconnect on unmount - keep connection alive
  })
  
  return {
    connect,
    disconnect,
    subscribe,
    send,
    isConnected,
    lastMessage,
    error
  }
}
