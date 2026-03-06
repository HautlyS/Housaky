export function bytesToHex(bytes: number[]): string {
  return bytes.map(b => b.toString(16).padStart(2, '0')).join('')
}

export function hexToBytes(hex: string): number[] {
  const clean = hex.replace(/^0x/, '')
  const bytes: number[] = []
  for (let i = 0; i < clean.length; i += 2) {
    bytes.push(parseInt(clean.slice(i, i + 2), 16))
  }
  return bytes
}

export function base64Encode(str: string): string {
  if (typeof btoa === 'function') return btoa(str)
  return Buffer.from(str, 'binary').toString('base64')
}

export function base64Decode(str: string): string {
  if (typeof atob === 'function') return atob(str)
  return Buffer.from(str, 'base64').toString('binary')
}

export function bytesToBinary(bytes: number[]): string {
  return bytes.map(b => b.toString(2).padStart(8, '0')).join(' ')
}

export function binaryToBytes(bin: string): number[] {
  return bin.split(/\s+/).map(b => parseInt(b, 2))
}

export function xorBytes(a: number[], b: number[]): number[] {
  const len = Math.max(a.length, b.length)
  const result: number[] = []
  for (let i = 0; i < len; i++) {
    result.push((a[i % a.length] ?? 0) ^ (b[i % b.length] ?? 0))
  }
  return result
}

export function rotateLeft(byte: number, n: number): number {
  n = n % 8
  return ((byte << n) | (byte >> (8 - n))) & 0xFF
}

export function rotateRight(byte: number, n: number): number {
  n = n % 8
  return ((byte >> n) | (byte << (8 - n))) & 0xFF
}

export function swapEndianness(bytes: number[]): number[] {
  const result = [...bytes]
  for (let i = 0; i < result.length - 1; i += 2) {
    [result[i], result[i + 1]] = [result[i + 1], result[i]]
  }
  return result
}
