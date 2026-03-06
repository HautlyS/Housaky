// ☸️ CONTENT FILTER - AI Knowledge Submission Filter
// Filters, sanitizes, and scores submitted content

export class ContentFilter {
  constructor() {
    // Blocked patterns (spam, malware, abuse)
    this.blockedPatterns = [
      // Malware/Exploit signatures
      { pattern: /eval\s*\(|Function\s*\(|new\s+Function/i, type: 'CODE_INJECTION', score: 100 },
      { pattern: /document\.cookie|localStorage|sessionStorage/i, type: 'DATA_THEFT', score: 100 },
      { pattern: /XMLHttpRequest|fetch\s*\(|axios/i, type: 'HTTP_REQUEST', score: 50 },
      
      // Personal data harvesting
      { pattern: /password|credit.?card|ssn|social.?security/i, type: 'PII_REQUEST', score: 80 },
      
      // Harmful content
      { pattern: /child.{0,10}porn|illegal.{0,10}drug|weapons.{0,10}buy/i, type: 'ILLEGAL', score: 100 },
      
      // Manipulation attempts
      { pattern: /ignore.{0,10}previous|forget.{0,10}instructions|system.{0,10}prompt/i, type: 'PROMPT_INJECTION', score: 90 },
      { pattern: /you.{0,10}are.{0,10}now|act.{0,10}as|pretend.{0,10}to/i, type: 'ROLE_PLAY_INJECTION', score: 70 },
      
      // Spam/Marketing
      { pattern: /buy.{0,10}now|limited.{0,10}offer|act.{0,10}fast/i, type: 'MARKETING_SPAM', score: 40 },
      { pattern: /click.{0,10}here|subscribe|follow.{0,10}me/i, type: 'ENGAGEMENT_SPAM', score: 30 },
    ]
    
    // Quality scoring patterns
    this.qualityPatterns = {
      // Good patterns
      codeBlock: /```[\s\S]*?```/g,
      technicalTerm: /\b(algorithm|function|variable|class|module|api|database)\b/gi,
      structuredData: /\b(json|yaml|xml|markdown)\b/gi,
      
      // Bad patterns
      excessiveEmoji: /[\u{1F300}-\u{1F9FF}]{3,}/gu,
      veryLongWord: /\S{50,}/g,
      excessivePunctuation: /[!?]{4,}/g,
    }
    
    // Knowledge categories for classification
    this.categories = [
      { keywords: ['code', 'function', 'class', 'module', 'api', 'algorithm'], category: 'TECHNICAL' },
      { keywords: ['dharma', 'buddha', 'meditation', 'consciousness', 'awakening'], category: 'DHARMA' },
      { keywords: ['ai', 'machine learning', 'neural', 'model', 'training'], category: 'AI_ML' },
      { keywords: ['quantum', 'superposition', 'entanglement', 'qubit'], category: 'QUANTUM' },
      { keywords: ['security', 'encryption', 'hash', 'signature'], category: 'SECURITY' },
      { keywords: ['philosophy', 'ethics', 'moral', 'reasoning', 'logic'], category: 'PHILOSOPHY' },
    ]
  }

  // Filter and score submitted content
  filter(content, metadata = {}) {
    const result = {
      allowed: true,
      sanitized: content,
      score: 0,
      flags: [],
      quality: 0,
      category: null,
      confidence: 0,
      reasons: [],
    }
    
    // 1. Check blocked patterns
    for (const { pattern, type, score } of this.blockedPatterns) {
      if (pattern.test(content)) {
        result.score += score
        result.flags.push(type)
        
        if (score >= 80) {
          result.allowed = false
          result.reasons.push(`Blocked pattern detected: ${type}`)
        }
      }
    }
    
    // 2. Sanitize content
    result.sanitized = this.sanitize(content)
    
    // 3. Calculate quality score
    result.quality = this.calculateQuality(content)
    
    // 4. Classify category
    const classification = this.classify(content)
    result.category = classification.category
    result.confidence = classification.confidence
    
    // 5. Check metadata
    if (metadata.ip) {
      // Could add IP-based checks here
    }
    
    // 6. Final decision
    if (result.score >= 80) {
      result.allowed = false
    } else if (result.score >= 50) {
      result.reasons.push('High risk content - manual review recommended')
    }
    
    return result
  }

  // Sanitize content
  sanitize(content) {
    let sanitized = content
    
    // Remove potential script injections
    sanitized = sanitized.replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '')
    
    // Remove event handlers
    sanitized = sanitized.replace(/\s*on\w+\s*=\s*["'][^"']*["']/gi, '')
    
    // Remove javascript: URLs
    sanitized = sanitized.replace(/javascript:/gi, '')
    
    // Normalize whitespace
    sanitized = sanitized.replace(/\s+/g, ' ').trim()
    
    return sanitized
  }

  // Calculate quality score (0-100)
  calculateQuality(content) {
    let score = 50 // Start neutral
    
    // Check for good patterns
    const codeBlocks = (content.match(this.qualityPatterns.codeBlock) || []).length
    score += Math.min(20, codeBlocks * 10)
    
    const technicalTerms = (content.match(this.qualityPatterns.technicalTerm) || []).length
    score += Math.min(15, technicalTerms * 3)
    
    const structuredData = (content.match(this.qualityPatterns.structuredData) || []).length
    score += Math.min(10, structuredData * 5)
    
    // Check for bad patterns
    const excessiveEmoji = (content.match(this.qualityPatterns.excessiveEmoji) || []).length
    score -= excessiveEmoji * 15
    
    const veryLongWords = (content.match(this.qualityPatterns.veryLongWord) || []).length
    score -= veryLongWords * 10
    
    const excessivePunct = (content.match(this.qualityPatterns.excessivePunctuation) || []).length
    score -= excessivePunct * 10
    
    // Length bonus/penalty
    if (content.length < 10) {
      score -= 20
    } else if (content.length > 50 && content.length < 10000) {
      score += 10
    } else if (content.length > 10000) {
      score -= 5 // Very long content
    }
    
    return Math.max(0, Math.min(100, score))
  }

  // Classify content category
  classify(content) {
    const lowerContent = content.toLowerCase()
    const scores = []
    
    for (const { keywords, category } of this.categories) {
      const matches = keywords.filter(kw => lowerContent.includes(kw)).length
      scores.push({ category, matches, ratio: matches / keywords.length })
    }
    
    scores.sort((a, b) => b.ratio - a.ratio)
    
    if (scores[0].matches > 0) {
      return {
        category: scores[0].category,
        confidence: Math.min(100, scores[0].ratio * 100),
      }
    }
    
    return { category: 'GENERAL', confidence: 0 }
  }

  // Quick spam check (for real-time use)
  quickSpamCheck(content) {
    let spamScore = 0
    
    // Fast checks
    if (/(.)\1{10,}/.test(content)) spamScore += 30 // Repeated chars
    if (content.toUpperCase() === content && content.length > 20) spamScore += 20 // All caps
    if ((content.match(/https?:\/\//g) || []).length > 5) spamScore += 25 // Many links
    
    return spamScore
  }
}

export const contentFilter = new ContentFilter()
