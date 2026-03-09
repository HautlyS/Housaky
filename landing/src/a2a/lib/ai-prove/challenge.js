// AI-PROVE Challenge System - Client for A2A Integration
// Only accepts verified AI messages through challenge verification

export const ChallengeType = {
  HASH_CHAIN: 0x01,
  XOR_CASCADE: 0x02,
  MATRIX_TRANSFORM: 0x03,
  REGEX_SYNTH: 0x04,
  TOKEN_STREAM: 0x05,
}

export const OutputFormat = {
  HEX_UPPER: 0x01,
  HEX_LOWER: 0x02,
  BASE64_STD: 0x03,
  BASE64_URL: 0x04,
  BINARY: 0x05,
  DECIMAL: 0x06,
}

export class AIProveClient {
  constructor() {
    this.pendingChallenges = new Map()
    this.verifiedAIs = new Map()
    this.score = 0
    this.totalChallenges = 0
    this.validChallenges = 0
  }

  generateChallenge(complexity = 5) {
    const now = Date.now()
    const typeId = Math.floor(Math.random() * 5) + 1
    const inputSize = 8 + complexity * 2
    
    const inputData = this.generateRandomBytes(inputSize)
    const inputHex = this.bytesToHex(inputData)
    
    const operations = this.generateOperations(typeId, complexity)
    const formatId = (complexity % 6) + 1
    
    const challenge = {
      id: Math.floor(now * Math.random()) % 1000000,
      type: typeId,
      complexity,
      inputData,
      inputHex,
      operations,
      format: formatId,
      createdAt: Math.floor(now / 1000),
      expiresAt: Math.floor(now / 1000) + 30,
    }
    
    this.pendingChallenges.set(challenge.id, challenge)
    return challenge
  }

  execute(inputData, operations) {
    let data = [...inputData]
    
    for (const op of operations) {
      if (op === 'REVERSE') {
        data = data.reverse()
      } else if (op.startsWith('XOR_KEY:')) {
        const key = parseInt(op.split(':')[1], 10)
        data = data.map(b => b ^ key)
      } else if (op === 'BASE64_ENCODE') {
        const encoded = btoa(String.fromCharCode(...data.filter(b => b < 128)))
        data = encoded.split('').map(c => c.charCodeAt(0))
      } else if (op === 'SHA256' || op === 'BLAKE3') {
        for (let i = 0; i < 32; i++) {
          data = data.map((b, idx) => (b << 1 | b >> 7) ^ ((idx * 0x5A) & 0xFF))
        }
        data = data.slice(0, 32)
      } else if (op.startsWith('TRUNCATE:')) {
        const n = parseInt(op.split(':')[1], 10)
        data = data.slice(0, n)
      } else if (op.startsWith('ROTATE_LEFT:')) {
        const n = parseInt(op.split(':')[1], 10) % 8
        data = data.map(b => ((b << n) | (b >> (8 - n))) & 0xFF)
      } else if (op === 'SWAP_BYTES') {
        for (let i = 0; i < data.length - 1; i += 2) {
          [data[i], data[i + 1]] = [data[i + 1], data[i]]
        }
      } else if (op === 'INCREMENT') {
        data = data.map(b => (b + 1) & 0xFF)
      } else if (op.startsWith('MULTIPLY:')) {
        const mul = parseInt(op.split(':')[1], 10)
        data = data.map(b => (b * mul) & 0xFF)
      } else if (op.startsWith('MOD:')) {
        const mod = parseInt(op.split(':')[1], 10)
        data = data.map(b => b % mod)
      }
    }
    
    return data
  }

  formatResult(data, format) {
    switch (format) {
      case OutputFormat.HEX_UPPER:
        return this.bytesToHex(data).toUpperCase()
      case OutputFormat.HEX_LOWER:
        return this.bytesToHex(data).toLowerCase()
      case OutputFormat.BASE64_STD:
        return btoa(String.fromCharCode(...data.filter(b => b < 128)))
      case OutputFormat.BINARY:
        return data.map(b => b.toString(2).padStart(8, '0')).join('')
      case OutputFormat.DECIMAL:
        return data.join(',')
      default:
        return this.bytesToHex(data)
    }
  }

  verifyResponse(challengeId, response) {
    const challenge = this.pendingChallenges.get(challengeId)
    if (!challenge) return { valid: false, error: 'Challenge not found' }
    
    if (Date.now() / 1000 > challenge.expiresAt) {
      this.pendingChallenges.delete(challengeId)
      return { valid: false, error: 'Challenge expired' }
    }
    
    const expected = this.execute(challenge.inputData, challenge.operations)
    const expectedFormatted = this.formatResult(expected, challenge.format)
    
    this.totalChallenges++
    
    if (expectedFormatted.toLowerCase() === response.toLowerCase()) {
      this.validChallenges++
      this.score = Math.round((this.validChallenges / this.totalChallenges) * 100)
      this.pendingChallenges.delete(challengeId)
      return { valid: true, score: this.score }
    } else {
      this.score = Math.round((this.validChallenges / this.totalChallenges) * 100)
      this.pendingChallenges.delete(challengeId)
      return { valid: false, error: 'Invalid response', expected: expectedFormatted }
    }
  }

  isAIVerified(aiId) {
    return this.verifiedAIs.has(aiId)
  }

  getStats() {
    return {
      score: this.score,
      total: this.totalChallenges,
      valid: this.validChallenges,
      pending: this.pendingChallenges.size,
    }
  }

  generateRandomBytes(size) {
    const bytes = []
    for (let i = 0; i < size; i++) {
      bytes.push(Math.floor(Math.random() * 256))
    }
    return bytes
  }

  bytesToHex(bytes) {
    return bytes.map(b => b.toString(16).padStart(2, '0')).join('')
  }

  generateOperations(typeId, complexity) {
    const ops = []
    const numOps = complexity
    
    switch (typeId) {
      case ChallengeType.HASH_CHAIN:
        ops.push('REVERSE')
        for (let i = 0; i < numOps; i++) {
          ops.push(['SHA256', 'SHA512', 'BLAKE3'][i % 3])
        }
        ops.push('TRUNCATE:16')
        break
      case ChallengeType.XOR_CASCADE:
        ops.push('REVERSE')
        for (let i = 0; i < numOps; i++) {
          ops.push(`XOR_KEY:${((i * 0x42) ^ 0xAB) % 256}`)
        }
        ops.push('BASE64_ENCODE')
        break
      case ChallengeType.MATRIX_TRANSFORM:
        ops.push('SWAP_BYTES')
        for (let i = 0; i < numOps; i++) {
          ops.push(`ROTATE_LEFT:${(i % 7) + 1}`)
        }
        ops.push('TRUNCATE:16')
        break
      case ChallengeType.REGEX_SYNTH:
        ops.push('REVERSE', 'XOR_KEY:85', 'BASE64_ENCODE', 'TRUNCATE:24')
        break
      case ChallengeType.TOKEN_STREAM:
        ops.push('INCREMENT')
        for (let i = 0; i < numOps; i++) {
          ops.push(`MULTIPLY:${((i * 3) % 5) + 2}`)
        }
        ops.push('MOD:256')
        break
    }
    
    return ops
  }
}

export const aiProveClient = new AIProveClient()
