import { bytesToHex } from './binary-language'
import { blake3Hash } from './checksum-validator'

export enum ChallengeType {
  HashChain = 0x01,
  XorCascade = 0x02,
  MatrixTransform = 0x03,
  RegexSynth = 0x04,
  TokenStream = 0x05,
}

export enum OutputFormat {
  HexUpper = 0x01,
  HexLower = 0x02,
  Base64Std = 0x03,
  Base64Url = 0x04,
  Binary = 0x05,
  Decimal = 0x06,
}

export enum Operation {
  Reverse = 'REVERSE',
  XorKey = 'XOR_KEY',
  Base64Encode = 'BASE64_ENCODE',
  SHA256 = 'SHA256',
  SHA512 = 'SHA512',
  BLAKE3 = 'BLAKE3',
  RotateLeft = 'ROTATE_LEFT',
  SwapBytes = 'SWAP_BYTES',
  Increment = 'INCREMENT',
  Multiply = 'MULTIPLY',
  Mod = 'MOD',
}

export interface Challenge {
  id: number
  type: ChallengeType
  typeName: string
  complexity: number
  inputData: number[]
  inputHex: string
  operations: string[]
  expectedFormat: OutputFormat
  createdAt: number
  expiresAt: number
  nonce: number[]
  timeoutMs: number
}

export interface ChallengeResponse {
  challengeId: number
  result: string
  resultHex: string
  computeTimeMs: number
  tokenCount: number
  checksum: string
  timestamp: number
}

export class ChallengeGenerator {
  private complexityRange = { min: 3, max: 7 }
  
  generate(complexity?: number): Challenge {
    const now = Date.now()
    const comp = complexity ?? this.randomComplexity()
    const typeId = Math.floor(Math.random() * 5) + 1
    const type = typeId as ChallengeType
    const inputSize = 8 + comp * 2
    const inputData = this.generateRandomBytes(inputSize)
    const inputHex = bytesToHex(inputData)
    const operations = this.generateOperations(type, comp)
    const format = (comp % 6) + 1 as OutputFormat
    const nonce = this.generateRandomBytes(16)
    
    return {
      id: Math.floor(now * Math.random() * 1000) % 1000000,
      type,
      typeName: ChallengeType[type],
      complexity: comp,
      inputData,
      inputHex,
      operations,
      expectedFormat: format,
      createdAt: Math.floor(now / 1000),
      expiresAt: Math.floor(now / 1000) + 30,
      nonce,
      timeoutMs: 30000,
    }
  }
  
  execute(input: number[], operations: string[]): number[] {
    let data = [...input]
    
    for (const op of operations) {
      if (op === Operation.Reverse) {
        data = data.reverse()
      } else if (op.startsWith('XOR_KEY:')) {
        const key = parseInt(op.split(':')[1], 10)
        data = data.map(b => b ^ key)
      } else if (op === Operation.Base64Encode) {
        const encoded = btoa(String.fromCharCode(...data.filter(b => b < 128)))
        data = encoded.split('').map(c => c.charCodeAt(0))
      } else if (op === Operation.SHA256 || op === Operation.BLAKE3) {
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
      } else if (op === Operation.SwapBytes) {
        for (let i = 0; i < data.length - 1; i += 2) {
          [data[i], data[i + 1]] = [data[i + 1], data[i]]
        }
      } else if (op === Operation.Increment) {
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
  
  formatResult(data: number[], format: OutputFormat): string {
    switch (format) {
      case OutputFormat.HexUpper: return bytesToHex(data).toUpperCase()
      case OutputFormat.HexLower: return bytesToHex(data).toLowerCase()
      case OutputFormat.Base64Std: return btoa(String.fromCharCode(...data.filter(b => b < 128)))
      case OutputFormat.Binary: return data.map(b => b.toString(2).padStart(8, '0')).join('')
      case OutputFormat.Decimal: return data.join(',')
      default: return bytesToHex(data)
    }
  }
  
  validate(challenge: Challenge, response: ChallengeResponse): boolean {
    if (Date.now() / 1000 > challenge.expiresAt) return false
    const expectedChecksum = blake3Hash(response.result)
    if (expectedChecksum !== response.checksum) return false
    const expected = this.execute(challenge.inputData, challenge.operations)
    const expectedFormatted = this.formatResult(expected, challenge.expectedFormat)
    return expectedFormatted.toLowerCase() === response.result.toLowerCase()
  }
  
  private randomComplexity(): number {
    return Math.floor(Math.random() * (this.complexityRange.max - this.complexityRange.min + 1)) + this.complexityRange.min
  }
  
  private generateRandomBytes(size: number): number[] {
    const bytes: number[] = []
    for (let i = 0; i < size; i++) {
      bytes.push(Math.floor(Math.random() * 256))
    }
    return bytes
  }
  
  private generateOperations(type: ChallengeType, complexity: number): string[] {
    const ops: string[] = []
    const numOps = complexity
    
    switch (type) {
      case ChallengeType.HashChain:
        ops.push(Operation.Reverse)
        for (let i = 0; i < numOps; i++) {
          ops.push([Operation.SHA256, Operation.SHA512, Operation.BLAKE3][i % 3])
        }
        ops.push('TRUNCATE:16')
        break
      case ChallengeType.XorCascade:
        ops.push(Operation.Reverse)
        for (let i = 0; i < numOps; i++) {
          ops.push(`XOR_KEY:${((i * 0x42) ^ 0xAB) % 256}`)
        }
        ops.push(Operation.Base64Encode)
        break
      case ChallengeType.MatrixTransform:
        ops.push(Operation.SwapBytes)
        for (let i = 0; i < numOps; i++) {
          ops.push(`ROTATE_LEFT:${(i % 7) + 1}`)
        }
        ops.push('TRUNCATE:16')
        break
      case ChallengeType.RegexSynth:
        ops.push(Operation.Reverse, 'XOR_KEY:85', Operation.Base64Encode, 'TRUNCATE:24')
        break
      case ChallengeType.TokenStream:
        ops.push(Operation.Increment)
        for (let i = 0; i < numOps; i++) {
          ops.push(`MULTIPLY:${((i * 3) % 5) + 2}`)
        }
        ops.push('MOD:256')
        break
    }
    return ops
  }
}

export const challengeGenerator = new ChallengeGenerator()
