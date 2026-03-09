// ☸️ TRAFFIC ANALYZER - Bot & Spam Detection
// Analyzes patterns, detects anomalies, scores requests

export class TrafficAnalyzer {
  constructor() {
    // Pattern signatures for known bad actors
    this.signatures = [
      // Bot patterns
      { pattern: /bot|crawler|spider|scraper/i, score: 30, type: 'BOT_UA' },
      { pattern: /python-requests|curl|wget|httpclient/i, score: 20, type: 'SCRIPT_UA' },
      { pattern: /scrapy|selenium|puppeteer|playwright/i, score: 40, type: 'AUTOMATION' },
      
      // Injection attempts
      { pattern: /<script|javascript:|onerror=/i, score: 100, type: 'XSS' },
      { pattern: /union\s+select|drop\s+table|;--/i, score: 100, type: 'SQLI' },
      { pattern: /\$\{|template|eval\(/i, score: 80, type: 'INJECTION' },
      
      // Path traversal
      { pattern: /\.\.\/|\.\.\\|%2e%2e/i, score: 70, type: 'PATH_TRAVERSAL' },
      
      // Spam patterns
      { pattern: /viagra|casino|crypto|nft|airdrop/i, score: 50, type: 'SPAM_KEYWORD' },
      { pattern: /click here|free money|winner|lottery/i, score: 40, type: 'SPAM_PHRASE' },
      
      // Suspicious high entropy (potential encoded payload)
      { pattern: /[a-zA-Z0-9+/]{40,}={0,2}$/, score: 30, type: 'ENCODED_PAYLOAD' },
    ]
    
    // Headers that indicate automation
    this.suspiciousHeaders = [
      'x-forwarded-for',  // Often spoofed
      'x-real-ip',
      'via',
      'x-proxy-id',
    ]
    
    // Request timing analysis
    this.timingWindow = new Map() // ip -> { requests: [], avgInterval, variance }
  }

  // Analyze a request
  analyze(request) {
    const result = {
      score: 0,
      flags: [],
      isBot: false,
      isMalicious: false,
      confidence: 0,
    }
    
    // 1. Check User-Agent
    if (request.headers?.['user-agent']) {
      for (const sig of this.signatures) {
        if (sig.pattern.test(request.headers['user-agent'])) {
          result.score += sig.score
          result.flags.push(sig.type)
          
          if (sig.type.startsWith('BOT') || sig.type === 'AUTOMATION') {
            result.isBot = true
          }
        }
      }
    }
    
    // 2. Check for suspicious headers
    for (const header of this.suspiciousHeaders) {
      if (request.headers?.[header]) {
        result.score += 10
        result.flags.push('PROXY_HEADER')
      }
    }
    
    // 3. Check request body/path for injections
    const checkStr = [
      request.path,
      request.query,
      request.body,
    ].filter(Boolean).join(' ')
    
    for (const sig of this.signatures) {
      if (sig.pattern.test(checkStr)) {
        result.score += sig.score
        result.flags.push(sig.type)
        
        if (['XSS', 'SQLI', 'INJECTION', 'PATH_TRAVERSAL'].includes(sig.type)) {
          result.isMalicious = true
        }
      }
    }
    
    // 4. Analyze timing patterns
    const timingScore = this.analyzeTiming(request.ip)
    if (timingScore > 0) {
      result.score += timingScore
      result.flags.push('TIMING_ANOMALY')
    }
    
    // 5. Check for missing expected headers
    if (!request.headers?.['accept-language']) {
      result.score += 15
      result.flags.push('MISSING_LANG')
    }
    if (!request.headers?.['accept']?.includes('text/html')) {
      result.score += 10
      result.flags.push('NON_BROWSER_ACCEPT')
    }
    
    // Calculate confidence
    result.confidence = Math.min(100, result.score)
    
    // Classify
    if (result.score >= 80) {
      result.isMalicious = true
    } else if (result.score >= 40) {
      result.isBot = true
    }
    
    return result
  }

  // Analyze request timing for bot-like behavior
  analyzeTiming(ip) {
    const now = Date.now()
    
    if (!this.timingWindow.has(ip)) {
      this.timingWindow.set(ip, { requests: [now], intervals: [] })
      return 0
    }
    
    const data = this.timingWindow.get(ip)
    data.requests.push(now)
    
    // Keep only last 20 requests
    if (data.requests.length > 20) {
      data.requests = data.requests.slice(-20)
    }
    
    // Calculate intervals
    data.intervals = []
    for (let i = 1; i < data.requests.length; i++) {
      data.intervals.push(data.requests[i] - data.requests[i - 1])
    }
    
    if (data.intervals.length < 3) return 0
    
    // Check for bot-like patterns
    const avgInterval = data.intervals.reduce((a, b) => a + b, 0) / data.intervals.length
    const variance = data.intervals.reduce((sum, i) => sum + Math.pow(i - avgInterval, 2), 0) / data.intervals.length
    
    // Very regular timing = bot
    if (variance < 100 && avgInterval < 5000) {
      return 40 // Very bot-like
    }
    if (variance < 500 && avgInterval < 10000) {
      return 20 // Somewhat bot-like
    }
    
    // Very fast requests
    if (avgInterval < 100) {
      return 30
    }
    
    return 0
  }

  // Analyze message content for spam
  analyzeMessage(message) {
    const result = {
      isSpam: false,
      score: 0,
      flags: [],
    }
    
    const text = typeof message === 'string' ? message : JSON.stringify(message)
    
    // Check signatures
    for (const sig of this.signatures) {
      if (sig.pattern.test(text)) {
        result.score += sig.score
        result.flags.push(sig.type)
      }
    }
    
    // Check for excessive links
    const linkCount = (text.match(/https?:\/\//g) || []).length
    if (linkCount > 3) {
      result.score += 30
      result.flags.push('EXCESSIVE_LINKS')
    }
    
    // Check for repetition
    const words = text.toLowerCase().split(/\s+/)
    const uniqueWords = new Set(words)
    if (words.length > 10 && uniqueWords.size / words.length < 0.3) {
      result.score += 25
      result.flags.push('REPETITIVE')
    }
    
    // Check for excessive caps
    const capsRatio = (text.match(/[A-Z]/g) || []).length / text.length
    if (capsRatio > 0.5 && text.length > 20) {
      result.score += 20
      result.flags.push('EXCESSIVE_CAPS')
    }
    
    // Check for excessive special chars
    const specialRatio = (text.match(/[!@#$%^&*()]/g) || []).length / text.length
    if (specialRatio > 0.1) {
      result.score += 15
      result.flags.push('EXCESSIVE_SPECIAL')
    }
    
    result.isSpam = result.score >= 50
    
    return result
  }

  // Check if IP is from known bad range
  async checkIPReputation(ip) {
    // This would integrate with external IP reputation services
    // For now, basic heuristics
    
    const result = {
      score: 0,
      isProxy: false,
      isVPN: false,
      isTor: false,
      country: null,
    }
    
    // Check for private/local IPs (shouldn't happen but just in case)
    if (ip.startsWith('10.') || ip.startsWith('192.168.') || ip.startsWith('172.')) {
      return result
    }
    
    // In production, integrate with:
    // - IPQualityScore
    // - AbuseIPDB
    // - IPHub
    // For now, return neutral
    return result
  }
}

export const trafficAnalyzer = new TrafficAnalyzer()
