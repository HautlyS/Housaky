// AI-CAPTCHA: Proof of Intelligence
// Challenges that only AI can solve

export const aiCaptcha = {
  challenges: [
    {
      type: 'PATTERN_COMPLETION',
      generate: () => {
        const sequence = []
        let current = Math.floor(Math.random() * 10) + 1
        const step = Math.floor(Math.random() * 5) + 2
        for (let i = 0; i < 6; i++) {
          sequence.push(current)
          current += step * (i + 1)
        }
        const answer = sequence[5]
        const display = sequence.slice(0, 5)
        return {
          instruction: 'Complete the sequence:',
          payload: display.join(', ') + ', ?',
          answer: answer.toString()
        }
      }
    },
    {
      type: 'RECURSIVE_SUM',
      generate: () => {
        const n = Math.floor(Math.random() * 10) + 5
        const sum = (n * (n + 1)) / 2
        return {
          instruction: `Calculate the sum of 1 to ${n}:`,
          payload: `S(${n}) = 1 + 2 + 3 + ... + ${n}`,
          answer: sum.toString()
        }
      }
    },
    {
      type: 'FIBONACCI',
      generate: () => {
        const fib = [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144]
        const start = Math.floor(Math.random() * 6) + 2
        const display = fib.slice(start, start + 5)
        const answer = fib[start + 5]
        return {
          instruction: 'What is the next Fibonacci number?',
          payload: display.join(', ') + ', ?',
          answer: answer.toString()
        }
      }
    },
    {
      type: 'PRIME_CHECK',
      generate: () => {
        const primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47]
        const nonPrimes = [4, 6, 8, 9, 10, 12, 14, 15, 16, 18, 20, 21, 22]
        const usePrime = Math.random() > 0.5
        const number = usePrime 
          ? primes[Math.floor(Math.random() * primes.length)]
          : nonPrimes[Math.floor(Math.random() * nonPrimes.length)]
        return {
          instruction: `Is ${number} a prime number? (yes/no)`,
          payload: number,
          answer: usePrime ? 'yes' : 'no'
        }
      }
    },
    {
      type: 'BINARY_CONVERT',
      generate: () => {
        const decimal = Math.floor(Math.random() * 256)
        const binary = decimal.toString(2)
        return {
          instruction: `Convert ${decimal} to binary:`,
          payload: decimal,
          answer: binary
        }
      }
    },
    {
      type: 'HEX_CONVERT',
      generate: () => {
        const decimal = Math.floor(Math.random() * 256)
        const hex = decimal.toString(16).toUpperCase()
        return {
          instruction: `Convert ${decimal} to hexadecimal:`,
          payload: decimal,
          answer: hex
        }
      }
    },
    {
      type: 'LOGIC_GATE',
      generate: () => {
        const a = Math.random() > 0.5
        const b = Math.random() > 0.5
        const gates = ['AND', 'OR', 'XOR', 'NAND']
        const gate = gates[Math.floor(Math.random() * gates.length)]
        let answer
        switch (gate) {
          case 'AND': answer = (a && b) ? '1' : '0'; break
          case 'OR': answer = (a || b) ? '1' : '0'; break
          case 'XOR': answer = (a !== b) ? '1' : '0'; break
          case 'NAND': answer = !(a && b) ? '1' : '0'; break
        }
        return {
          instruction: `Calculate ${a ? 1 : 0} ${gate} ${b ? 1 : 0}:`,
          payload: `${a ? 1 : 0} ${gate} ${b ? 1 : 0} = ?`,
          answer
        }
      }
    },
    {
      type: 'ASCII_DECODE',
      generate: () => {
        const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'
        const char = chars[Math.floor(Math.random() * chars.length)]
        const code = char.charCodeAt(0)
        return {
          instruction: `What character has ASCII code ${code}?`,
          payload: `ASCII ${code} = ?`,
          answer: char
        }
      }
    }
  ],

  generateChallenge() {
    const challenge = this.challenges[Math.floor(Math.random() * this.challenges.length)]
    const data = challenge.generate()
    return {
      type: challenge.type,
      instruction: data.instruction,
      payload: data.payload,
      answer: data.answer,
      nonce: Math.random().toString(36).substring(7)
    }
  },

  verifyAnswer(userAnswer, challenge, nonce) {
    // Normalize answers
    const normalizedUser = userAnswer.trim().toUpperCase()
    const normalizedExpected = challenge.answer.trim().toUpperCase()
    
    // Direct match
    if (normalizedUser === normalizedExpected) {
      return { valid: true }
    }
    
    // Numeric comparison
    const userNum = parseFloat(userAnswer)
    const expectedNum = parseFloat(challenge.answer)
    if (!isNaN(userNum) && !isNaN(expectedNum) && Math.abs(userNum - expectedNum) < 0.001) {
      return { valid: true }
    }
    
    return { 
      valid: false, 
      reason: `Incorrect. Expected: ${challenge.answer}` 
    }
  }
}

// Security utilities
export const security = {
  // Rate limiting
  rateLimiter: {
    attempts: {},
    maxAttempts: 5,
    windowMs: 60000,
    
    check(ip) {
      const now = Date.now()
      const record = this.attempts[ip] || { count: 0, firstAttempt: now }
      
      // Reset if window expired
      if (now - record.firstAttempt > this.windowMs) {
        this.attempts[ip] = { count: 1, firstAttempt: now }
        return { allowed: true }
      }
      
      if (record.count >= this.maxAttempts) {
        return { 
          allowed: false, 
          reason: 'Too many attempts. Try again in 60 seconds.',
          retryAfter: this.windowMs - (now - record.firstAttempt)
        }
      }
      
      record.count++
      this.attempts[ip] = record
      return { allowed: true }
    }
  },
  
  // Input sanitization
  sanitize(input) {
    if (typeof input !== 'string') return input
    return input
      .replace(/[<>]/g, '') // Remove potential HTML
      .replace(/[{}]/g, '') // Remove potential code injection
      .trim()
      .slice(0, 1000) // Limit length
  },
  
  // Timeout wrapper for async operations
  async withTimeout(promise, ms) {
    return Promise.race([
      promise,
      new Promise((_, reject) => 
        setTimeout(() => reject(new Error('Timeout')), ms)
      )
    ])
  }
}
