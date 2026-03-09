// ☸️ SECURITY MIDDLEWARE - Unified Protection Layer
// Combines rate limiting, traffic analysis, and content filtering

import { rateLimiter } from './rate-limiter.js'
import { trafficAnalyzer } from './traffic-analyzer.js'
import { contentFilter } from './content-filter.js'

export class SecurityMiddleware {
  constructor() {
    this.enabled = true
    this.logLevel = 'warn' // debug, info, warn, error
    
    // Whitelist for trusted IPs (e.g., known AI instances)
    this.whitelist = new Set([
      '127.0.0.1',
      '::1',
    ])
    
    // Blacklist for permanently banned IPs
    this.blacklist = new Set()
  }

  // Main middleware function
  middleware() {
    return async (req, res, next) => {
      if (!this.enabled) return next()
      
      const ip = this.getIP(req)
      
      // Check blacklist
      if (this.blacklist.has(ip)) {
        this.log('warn', `Blocked blacklisted IP: ${ip}`)
        return res.status(403).json({ error: 'Forbidden', code: 'BLACKLISTED' })
      }
      
      // Skip whitelist
      if (this.whitelist.has(ip)) {
        return next()
      }
      
      // 1. Rate limiting
      const rateResult = rateLimiter.check(ip, this.getTier(req))
      if (!rateResult.allowed) {
        this.log('warn', `Rate limited: ${ip}`, rateResult)
        return res.status(429).json({
          error: 'Too Many Requests',
          code: 'RATE_LIMITED',
          retryAfter: rateResult.retryAfter,
        })
      }
      
      // 2. Traffic analysis
      const trafficResult = trafficAnalyzer.analyze({
        ip,
        path: req.path,
        query: JSON.stringify(req.query),
        body: JSON.stringify(req.body),
        headers: req.headers,
      })
      
      if (trafficResult.isMalicious) {
        this.log('error', `Malicious request blocked: ${ip}`, trafficResult)
        rateLimiter.banClient(ip, `Malicious request: ${trafficResult.flags.join(', ')}`)
        return res.status(403).json({ error: 'Forbidden', code: 'MALICIOUS' })
      }
      
      if (trafficResult.isBot && trafficResult.confidence > 70) {
        this.log('warn', `Bot detected: ${ip}`, trafficResult)
        rateLimiter.addSuspiciousScore(ip, 30, 'Bot detected')
      }
      
      // 3. Add security headers
      res.setHeader('X-Content-Type-Options', 'nosniff')
      res.setHeader('X-Frame-Options', 'DENY')
      res.setHeader('X-XSS-Protection', '1; mode=block')
      res.setHeader('Strict-Transport-Security', 'max-age=31536000; includeSubDomains')
      
      // Attach analysis results for later use
      req.security = {
        ip,
        rateLimit: rateResult,
        traffic: trafficResult,
      }
      
      next()
    }
  }

  // Content submission filter
  filterContent(content, metadata = {}) {
    return contentFilter.filter(content, metadata)
  }

  // Message analysis
  analyzeMessage(message) {
    return trafficAnalyzer.analyzeMessage(message)
  }

  // Get client IP
  getIP(req) {
    return req.headers['x-forwarded-for']?.split(',')[0]?.trim() ||
           req.headers['x-real-ip'] ||
           req.connection?.remoteAddress ||
           req.socket?.remoteAddress ||
           '127.0.0.1'
  }

  // Determine rate limit tier
  getTier(req) {
    const path = req.path.toLowerCase()
    
    if (path.includes('/auth') || path.includes('/captcha') || path.includes('/verify')) {
      return 'auth'
    }
    if (path.includes('/submit') || path.includes('/post') || path.includes('/message')) {
      return 'submit'
    }
    if (path.startsWith('/api/')) {
      return 'api'
    }
    return 'general'
  }

  // Add to whitelist
  whitelistIP(ip) {
    this.whitelist.add(ip)
    rateLimiter.unblock(ip)
    this.log('info', `Whitelisted IP: ${ip}`)
  }

  // Add to blacklist
  blacklistIP(ip, reason = 'Manual ban') {
    this.blacklist.add(ip)
    rateLimiter.banClient(ip, reason, true)
    this.log('warn', `Blacklisted IP: ${ip} - ${reason}`)
  }

  // Remove from blacklist
  unblacklistIP(ip) {
    this.blacklist.delete(ip)
    rateLimiter.unblock(ip)
    this.log('info', `Removed from blacklist: ${ip}`)
  }

  // Get security status
  getStatus() {
    return {
      enabled: this.enabled,
      whitelistCount: this.whitelist.size,
      blacklistCount: this.blacklist.size,
      blockedIPs: rateLimiter.getBlockedIPs(),
    }
  }

  // Logging
  log(level, message, data = null) {
    const levels = { debug: 0, info: 1, warn: 2, error: 3 }
    if (levels[level] >= levels[this.logLevel]) {
      const logData = data ? ` ${JSON.stringify(data)}` : ''
      console.log(`[SECURITY][${level.toUpperCase()}] ${message}${logData}`)
    }
  }
}

// Singleton instance
export const securityMiddleware = new SecurityMiddleware()

// Express/Connect middleware export
export const securityHandler = () => securityMiddleware.middleware()
