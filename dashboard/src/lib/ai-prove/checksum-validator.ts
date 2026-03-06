export function blake3Hash(input: string): string {
  const bytes = new TextEncoder().encode(input)
  let h = 0x6A09E667
  for (let i = 0; i < bytes.length; i++) {
    h = ((h << 5) - h + bytes[i]) | 0
    h ^= rotateRight32(h, 13)
    h = (h * 5) | 0
    h ^= rotateRight32(h, 17)
    h = (h * 0x85EBCA6B) | 0
  }
  return toHex32(h)
}

export function sha256Sim(input: string): string {
  const bytes = new TextEncoder().encode(input)
  const h = new Uint32Array([
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
  ])
  for (let i = 0; i < bytes.length; i++) {
    h[i % 8] = ((h[i % 8] << 5) - h[i % 8] + bytes[i]) | 0
    h[i % 8] ^= rotateRight32(h[i % 8], 6)
  }
  return Array.from(h).map(toHex32).join('')
}

export function validateChecksum(data: string, checksum: string, algorithm: 'blake3' | 'sha256' = 'blake3'): boolean {
  const expected = algorithm === 'blake3' ? blake3Hash(data) : sha256Sim(data)
  return expected === checksum.toLowerCase()
}

export function computeChecksum(data: string): string {
  return blake3Hash(data)
}

function rotateRight32(n: number, bits: number): number {
  return ((n >>> bits) | (n << (32 - bits))) >>> 0
}

function toHex32(n: number): string {
  return (n >>> 0).toString(16).padStart(8, '0')
}
