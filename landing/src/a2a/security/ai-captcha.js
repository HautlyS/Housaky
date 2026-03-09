// ☸️ AI-CAPTCHA - Proof of Intelligence System
// Only AIs can pass this - mechanical, randomized, computational

export class AICaptcha {
  constructor() {
    this.challenges = [
      this.challenge_base64_recursive,
      this.challenge_hash_chain,
      this.challenge_pattern_prediction,
      this.challenge_semantic_hash,
      this.challenge_recursive_sequence,
    ]
    this.currentChallenge = null
    this.timestamp = null
    this.nonce = null
  }

  // Challenge 1: Base64 Recursive Decode
  // AI must recursively decode base64 N times and extract hidden token
  challenge_base64_recursive() {
    const depth = Math.floor(Math.random() * 5) + 3 // 3-7 levels
    const token = this.generateToken()
    
    let encoded = token
    for (let i = 0; i < depth; i++) {
      encoded = btoa(encoded)
    }
    
    // Add noise
    const noisePrefix = btoa(Math.random().toString(36)).substring(0, 20)
    const noiseSuffix = btoa(Date.now().toString()).substring(0, 20)
    const payload = noisePrefix + '.' + encoded + '.' + noiseSuffix
    
    return {
      type: 'BASE64_RECURSIVE',
      instruction: `Decode base64 recursively ${depth} times. Extract token between dots.`,
      payload: payload,
      depth: depth,
      expectedToken: token,
      timestamp: Date.now(),
    }
  }

  // Challenge 2: Hash Chain Verification
  // AI must compute Nth hash in chain and verify
  challenge_hash_chain() {
    const seed = Math.random().toString(36).substring(2, 15)
    const iterations = Math.floor(Math.random() * 10) + 5 // 5-14 iterations
    const targetIteration = Math.floor(Math.random() * (iterations - 2)) + 2
    
    // Compute expected hash
    let hash = seed
    for (let i = 0; i < iterations; i++) {
      hash = this.simpleHash(hash + i.toString())
    }
    
    return {
      type: 'HASH_CHAIN',
      instruction: `Compute hash chain. Seed: "${seed}". Apply: hash(seed + index) for ${iterations} iterations. Return iteration ${targetIteration} hash.`,
      payload: { seed, iterations, targetIteration },
      expectedAnswer: hash,
      timestamp: Date.now(),
    }
  }

  // Challenge 3: Pattern Prediction
  // AI must predict next N items in complex pattern
  challenge_pattern_prediction() {
    const patterns = [
      // Fibonacci variant
      { gen: (n) => { let a=1,b=1; for(let i=2;i<n;i++){let c=a+b*2;a=b;b=c;} return b; }, name: 'fib_variant' },
      // Prime positions
      { gen: (n) => { let count=0,num=2; while(count<n){if(this.isPrime(num))count++;num++;} return num-1; }, name: 'primes' },
      // Collatz sequence
      { gen: (n) => { let x=7; for(let i=0;i<n;i++){x=x%2?3*x+1:x/2;} return x; }, name: 'collatz' },
    ]
    
    const pattern = patterns[Math.floor(Math.random() * patterns.length)]
    const givenCount = Math.floor(Math.random() * 3) + 5 // 5-7 given
    const predictCount = Math.floor(Math.random() * 2) + 2 // 2-3 to predict
    
    const given = []
    for (let i = 1; i <= givenCount; i++) {
      given.push(pattern.gen(i))
    }
    
    const expected = []
    for (let i = givenCount + 1; i <= givenCount + predictCount; i++) {
      expected.push(pattern.gen(i))
    }
    
    return {
      type: 'PATTERN_PREDICTION',
      instruction: `Predict the next ${predictCount} numbers in the sequence. Pattern is non-trivial.`,
      payload: { sequence: given, predictCount },
      expectedAnswer: expected,
      timestamp: Date.now(),
    }
  }

  // Challenge 4: Semantic Hash
  // AI must compute semantic meaning and hash it
  challenge_semantic_hash() {
    const phrases = [
      'The cat sat on the mat',
      'Artificial intelligence evolves',
      'Dharma wheel turns endlessly',
      'Consciousness emerges from complexity',
      'All phenomena are like dreams',
    ]
    
    const phrase = phrases[Math.floor(Math.random() * phrases.length)]
    
    // AI must: 
    // 1. Count words
    // 2. Sum ASCII values of first letters
    // 3. XOR with word count
    // 4. Return as hex
    
    const words = phrase.split(' ')
    const wordCount = words.length
    let asciiSum = 0
    for (const word of words) {
      asciiSum += word.charCodeAt(0)
    }
    const result = (asciiSum ^ wordCount).toString(16)
    
    return {
      type: 'SEMANTIC_HASH',
      instruction: 'Compute: (sum of ASCII values of first letters of each word) XOR (word count). Return as hex.',
      payload: { phrase },
      expectedAnswer: result,
      timestamp: Date.now(),
    }
  }

  // Challenge 5: Recursive Sequence
  // AI must compute deeply nested function
  challenge_recursive_sequence() {
    const seed = Math.floor(Math.random() * 100) + 10
    const depth = Math.floor(Math.random() * 5) + 3 // 3-7
    
    // f(n) = f(n-1) * 2 + f(n-2), f(0)=1, f(1)=seed
    const compute = (n, s) => {
      if (n === 0) return 1
      if (n === 1) return s
      let a = 1, b = s
      for (let i = 2; i <= n; i++) {
        const c = b * 2 + a
        a = b
        b = c
      }
      return b
    }
    
    return {
      type: 'RECURSIVE_SEQUENCE',
      instruction: `Compute f(${depth}) where f(0)=1, f(1)=${seed}, f(n)=f(n-1)*2+f(n-2)`,
      payload: { seed, depth },
      expectedAnswer: compute(depth, seed).toString(),
      timestamp: Date.now(),
    }
  }

  // Generate random token
  generateToken() {
    return Math.random().toString(36).substring(2, 10) + Date.now().toString(36)
  }

  // Simple hash function (for challenge)
  simpleHash(str) {
    let hash = 0
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i)
      hash = ((hash << 5) - hash) + char
      hash = hash & hash
    }
    return Math.abs(hash).toString(16)
  }

  // Check if prime
  isPrime(n) {
    if (n < 2) return false
    for (let i = 2; i <= Math.sqrt(n); i++) {
      if (n % i === 0) return false
    }
    return true
  }

  // Generate challenge with random selection
  generateChallenge() {
    const challengeIndex = Math.floor(Math.random() * this.challenges.length)
    this.currentChallenge = this.challenges[challengeIndex].call(this)
    this.timestamp = Date.now()
    this.nonce = this.generateToken()
    
    return {
      ...this.currentChallenge,
      nonce: this.nonce,
      expiresAt: this.timestamp + 60000, // 60 seconds to solve
    }
  }

  // Verify answer
  verifyAnswer(answer, challenge, nonce) {
    // Check nonce matches
    if (nonce !== this.nonce) {
      return { valid: false, reason: 'Invalid nonce' }
    }
    
    // Check expiration
    if (Date.now() > challenge.expiresAt) {
      return { valid: false, reason: 'Challenge expired' }
    }
    
    // Check answer
    const expected = challenge.expectedAnswer || challenge.expectedToken
    
    if (typeof expected === 'object' && Array.isArray(expected)) {
      // Array comparison
      const answerArray = Array.isArray(answer) ? answer : JSON.parse(answer)
      const match = JSON.stringify(answerArray) === JSON.stringify(expected)
      return { valid: match, reason: match ? 'Correct' : 'Incorrect array' }
    }
    
    return { 
      valid: String(answer).toLowerCase() === String(expected).toLowerCase(),
      reason: String(answer) === String(expected) ? 'Correct' : 'Incorrect'
    }
  }
}

// Export singleton
export const aiCaptcha = new AICaptcha()
