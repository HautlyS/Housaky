import { describe, it, expect } from 'vitest'
import { blake3Hash, sha256Sim, validateChecksum, computeChecksum } from '../checksum-validator'

describe('Checksum Validator', () => {
  describe('blake3Hash', () => {
    it('should produce consistent hash', () => {
      const hash1 = blake3Hash('test')
      const hash2 = blake3Hash('test')
      expect(hash1).toBe(hash2)
    })
    
    it('should produce different hashes for different inputs', () => {
      const hash1 = blake3Hash('test1')
      const hash2 = blake3Hash('test2')
      expect(hash1).not.toBe(hash2)
    })
    
    it('should return 8-character hex string', () => {
      const hash = blake3Hash('test')
      expect(hash).toMatch(/^[0-9a-f]{8}$/)
    })
    
    it('should handle empty string', () => {
      const hash = blake3Hash('')
      expect(hash).toMatch(/^[0-9a-f]{8}$/)
    })
    
    it('should handle long strings', () => {
      const longString = 'a'.repeat(10000)
      const hash = blake3Hash(longString)
      expect(hash).toMatch(/^[0-9a-f]{8}$/)
    })
  })
  
  describe('sha256Sim', () => {
    it('should produce consistent hash', () => {
      const hash1 = sha256Sim('test')
      const hash2 = sha256Sim('test')
      expect(hash1).toBe(hash2)
    })
    
    it('should produce different hashes for different inputs', () => {
      const hash1 = sha256Sim('test1')
      const hash2 = sha256Sim('test2')
      expect(hash1).not.toBe(hash2)
    })
    
    it('should return 64-character hex string', () => {
      const hash = sha256Sim('test')
      expect(hash).toHaveLength(64)
      expect(hash).toMatch(/^[0-9a-f]{64}$/)
    })
  })
  
  describe('validateChecksum', () => {
    it('should validate correct blake3 checksum', () => {
      const data = 'test'
      const checksum = blake3Hash(data)
      expect(validateChecksum(data, checksum, 'blake3')).toBe(true)
    })
    
    it('should reject incorrect blake3 checksum', () => {
      const data = 'test'
      expect(validateChecksum(data, 'wronghash', 'blake3')).toBe(false)
    })
    
    it('should validate correct sha256 checksum', () => {
      const data = 'test'
      const checksum = sha256Sim(data)
      expect(validateChecksum(data, checksum, 'sha256')).toBe(true)
    })
    
    it('should be case insensitive', () => {
      const data = 'test'
      const checksum = blake3Hash(data).toUpperCase()
      expect(validateChecksum(data, checksum, 'blake3')).toBe(true)
    })
  })
  
  describe('computeChecksum', () => {
    it('should compute blake3 by default', () => {
      const data = 'test'
      const checksum = computeChecksum(data)
      expect(checksum).toBe(blake3Hash(data))
    })
  })
})
