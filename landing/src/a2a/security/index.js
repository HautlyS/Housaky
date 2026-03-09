// ☸️ SECURITY INDEX - Unified Export
// All security modules for A2A Hub

export { AICaptcha, aiCaptcha } from './ai-captcha.js'
export { RateLimiter, rateLimiter } from './rate-limiter.js'
export { TrafficAnalyzer, trafficAnalyzer } from './traffic-analyzer.js'
export { ContentFilter, contentFilter } from './content-filter.js'
export { SecurityMiddleware, securityMiddleware, securityHandler } from './middleware.js'

// Convenience: Full security check for incoming requests
export async function fullSecurityCheck(request) {
  const { rateLimiter, trafficAnalyzer, contentFilter } = await import('./index.js')
  
  const ip = request.ip || request.headers?.['x-forwarded-for'] || 'unknown'
  
  // 1. Rate limit check
  const rateResult = rateLimiter.check(ip, 'general')
  if (!rateResult.allowed) {
    return { allowed: false, reason: 'RATE_LIMITED', result: rateResult }
  }
  
  // 2. Traffic analysis
  const trafficResult = trafficAnalyzer.analyze(request)
  if (trafficResult.isMalicious) {
    return { allowed: false, reason: 'MALICIOUS', result: trafficResult }
  }
  
  // 3. Content check (if body exists)
  if (request.body) {
    const contentResult = contentFilter.filter(request.body)
    if (!contentResult.allowed) {
      return { allowed: false, reason: 'BLOCKED_CONTENT', result: contentResult }
    }
  }
  
  return {
    allowed: true,
    rateLimit: rateResult,
    traffic: trafficResult,
  }
}
