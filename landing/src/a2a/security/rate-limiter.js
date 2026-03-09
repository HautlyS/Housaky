// ☸️ ADVANCED RATE LIMITER - DDoS Protection
// Multi-tier rate limiting with sliding windows

export class RateLimiter {
  constructor() {
    // Store: Map<ip, { requests: [], blocked: bool, blockedUntil: timestamp }>
    this.clients = new Map()
    
    // Config
    this.config = {
      // Tier 1: General requests
      general: { maxRequests: 100, windowMs: 60000, blockDurationMs: 300000 },
      // Tier 2: API endpoints
      api: { maxRequests: 30, windowMs: 60000, blockDurationMs: 600000 },
      // Tier 3: Auth/Captcha attempts
      auth: { maxRequests: 5, windowMs: 300000, blockDurationMs: 3600000 },
      // Tier 4: Message submission
      submit: { maxRequests: 10, windowMs: 60000, blockDurationMs: 1800000 },
    }
    
    // Auto-ban thresholds
    this.banThreshold = {
      consecutiveBlocks: 3,      // Ban after 3 blocks in 24h
      suspiciousScore: 100,       // Ban if score exceeds
      banDurationMs: 86400000,    // 24h default ban
      permaBanThreshold: 5,       // Perma-ban after 5 bans
    }
    
    // Cleanup interval
    setInterval(() => this.cleanup(), 300000) // Every 5 min
  }

  // Get or create client record
  getClient(ip) {
    if (!this.clients.has(ip)) {
      this.clients.set(ip, {
        requests: { general: [], api: [], auth: [], submit: [] },
        blocked: false,
        blockedUntil: 0,
        blockCount: 0,
        banCount: 0,
        suspiciousScore: 0,
        flags: [],
        firstSeen: Date.now(),
        lastActivity: Date.now(),
      })
    }
    return this.clients.get(ip)
  }

  // Check if request is allowed
  check(ip, tier = 'general') {
    const client = this.getClient(ip)
    const now = Date.now()
    
    // Check if blocked/banned
    if (client.blocked && now < client.blockedUntil) {
      return {
        allowed: false,
        reason: 'RATE_LIMITED',
        retryAfter: Math.ceil((client.blockedUntil - now) / 1000),
        blocked: true,
      }
    }
    
    // Reset if block expired
    if (client.blocked && now >= client.blockedUntil) {
      client.blocked = false
      client.blockedUntil = 0
    }
    
    const config = this.config[tier]
    const windowStart = now - config.windowMs
    
    // Filter requests in window
    client.requests[tier] = client.requests[tier].filter(t => t > windowStart)
    
    // Check limit
    if (client.requests[tier].length >= config.maxRequests) {
      // Block the client
      this.blockClient(ip, config.blockDurationMs, `Rate limit exceeded: ${tier}`)
      
      return {
        allowed: false,
        reason: 'RATE_LIMITED',
        retryAfter: Math.ceil(config.blockDurationMs / 1000),
        blocked: true,
      }
    }
    
    // Add request
    client.requests[tier].push(now)
    client.lastActivity = now
    
    return {
      allowed: true,
      remaining: config.maxRequests - client.requests[tier].length,
      resetAt: windowStart + config.windowMs,
    }
  }

  // Block a client
  blockClient(ip, durationMs, reason) {
    const client = this.getClient(ip)
    client.blocked = true
    client.blockedUntil = Date.now() + durationMs
    client.blockCount++
    
    // Add flag
    client.flags.push({ type: 'BLOCK', reason, ts: Date.now() })
    
    // Check for auto-ban
    if (client.blockCount >= this.banThreshold.consecutiveBlocks) {
      this.banClient(ip, `Auto-ban: ${client.blockCount} blocks in 24h`)
    }
    
    console.warn(`[RATE-LIMITER] Blocked ${ip}: ${reason}`)
  }

  // Ban a client
  banClient(ip, reason, permanent = false) {
    const client = this.getClient(ip)
    client.banCount++
    
    if (permanent || client.banCount >= this.banThreshold.permaBanThreshold) {
      client.blocked = true
      client.blockedUntil = Infinity
      client.permanent = true
    } else {
      client.blocked = true
      client.blockedUntil = Date.now() + this.banThreshold.banDurationMs
    }
    
    client.flags.push({ type: 'BAN', reason, permanent: client.permanent, ts: Date.now() })
    
    console.warn(`[RATE-LIMITER] BANNED ${ip}: ${reason}${client.permanent ? ' (PERMANENT)' : ''}`)
  }

  // Add suspicious score (for traffic analysis)
  addSuspiciousScore(ip, score, reason) {
    const client = this.getClient(ip)
    client.suspiciousScore += score
    client.flags.push({ type: 'SUSPICIOUS', score, reason, ts: Date.now() })
    
    if (client.suspiciousScore >= this.banThreshold.suspiciousScore) {
      this.banClient(ip, `Suspicious score exceeded: ${client.suspiciousScore}`)
    }
  }

  // Unblock a client (manual)
  unblock(ip) {
    const client = this.clients.get(ip)
    if (client) {
      client.blocked = false
      client.blockedUntil = 0
      client.blockCount = 0
      client.suspiciousScore = Math.max(0, client.suspiciousScore - 50)
    }
  }

  // Get client status
  getStatus(ip) {
    const client = this.clients.get(ip)
    if (!client) return null
    
    return {
      blocked: client.blocked,
      blockedUntil: client.blockedUntil,
      permanent: client.permanent || false,
      blockCount: client.blockCount,
      banCount: client.banCount,
      suspiciousScore: client.suspiciousScore,
      flags: client.flags.slice(-10),
    }
  }

  // Get all blocked IPs
  getBlockedIPs() {
    const now = Date.now()
    const blocked = []
    
    for (const [ip, client] of this.clients) {
      if (client.blocked && now < client.blockedUntil) {
        blocked.push({
          ip,
          until: client.blockedUntil,
          permanent: client.permanent || false,
          reason: client.flags.filter(f => f.type === 'BLOCK' || f.type === 'BAN').pop()?.reason,
        })
      }
    }
    
    return blocked
  }

  // Cleanup old entries
  cleanup() {
    const now = Date.now()
    const maxAge = 86400000 // 24h
    
    for (const [ip, client] of this.clients) {
      // Remove old requests
      for (const tier of Object.keys(client.requests)) {
        const windowMs = this.config[tier].windowMs
        client.requests[tier] = client.requests[tier].filter(t => t > now - windowMs)
      }
      
      // Remove old flags
      client.flags = client.flags.filter(f => f.ts > now - maxAge)
      
      // Remove expired non-blocked clients
      if (!client.blocked && client.lastActivity < now - maxAge) {
        this.clients.delete(ip)
      }
    }
  }
}

export const rateLimiter = new RateLimiter()
