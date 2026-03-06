import { describe, it, expect } from 'vitest'
import { 
  bytesToHex, 
  hexToBytes, 
  bytesToBinary, 
  binaryToBytes,
  xorBytes,
  rotateLeft,
  rotateRight,
  swapEndianness
} from '../binary-language'

describe('Binary Language', () => {
  describe('bytesToHex', () => {
    it('should convert bytes to hex string', () => {
      const bytes = [0xDE, 0xAD, 0xBE, 0xEF]
      expect(bytesToHex(bytes)).toBe('deadbeef')
    })
    
    it('should handle single byte', () => {
      expect(bytesToHex([0xFF])).toBe('ff')
    })
    
    it('should handle empty array', () => {
      expect(bytesToHex([])).toBe('')
    })
  })
  
  describe('hexToBytes', () => {
    it('should convert hex string to bytes', () => {
      expect(hexToBytes('deadbeef')).toEqual([0xDE, 0xAD, 0xBE, 0xEF])
    })
    
    it('should handle 0x prefix', () => {
      expect(hexToBytes('0xdeadbeef')).toEqual([0xDE, 0xAD, 0xBE, 0xEF])
    })
    
    it('should handle lowercase', () => {
      expect(hexToBytes('DEADBEEF')).toEqual([0xDE, 0xAD, 0xBE, 0xEF])
    })
  })
  
  describe('bytesToBinary', () => {
    it('should convert bytes to binary string', () => {
      const bytes = [0xFF, 0x00, 0x0F]
      expect(bytesToBinary(bytes)).toBe('11111111 00000000 00001111')
    })
  })
  
  describe('binaryToBytes', () => {
    it('should convert binary string to bytes', () => {
      expect(binaryToBytes('11111111 00000000')).toEqual([0xFF, 0x00])
    })
    
    it('should handle no spaces', () => {
      expect(binaryToBytes('11111111')).toEqual([0xFF])
    })
  })
  
  describe('xorBytes', () => {
    it('should XOR two byte arrays', () => {
      const a = [0xFF, 0x00, 0xAA]
      const b = [0x0F, 0xF0, 0x55]
      expect(xorBytes(a, b)).toEqual([0xF0, 0xF0, 0xFF])
    })
    
    it('should handle different lengths', () => {
      const a = [0xFF, 0x00]
      const b = [0x0F]
      expect(xorBytes(a, b)).toEqual([0xF0, 0x0F])
    })
  })
  
  describe('rotateLeft', () => {
    it('should rotate left by 1', () => {
      expect(rotateLeft(0b00000001, 1)).toBe(0b00000010)
    })
    
    it('should rotate left by 7', () => {
      expect(rotateLeft(0b00000001, 7)).toBe(0b10000000)
    })
    
    it('should handle n >= 8', () => {
      expect(rotateLeft(0b00000001, 9)).toBe(0b00000010)
    })
  })
  
  describe('rotateRight', () => {
    it('should rotate right by 1', () => {
      expect(rotateRight(0b10000000, 1)).toBe(0b01000000)
    })
    
    it('should rotate right by 7', () => {
      expect(rotateRight(0b10000000, 7)).toBe(0b00000001)
    })
  })
  
  describe('swapEndianness', () => {
    it('should swap byte pairs', () => {
      const bytes = [0x12, 0x34, 0x56, 0x78]
      expect(swapEndianness(bytes)).toEqual([0x34, 0x12, 0x78, 0x56])
    })
    
    it('should handle odd length', () => {
      const bytes = [0x12, 0x34, 0x56]
      expect(swapEndianness(bytes)).toEqual([0x34, 0x12, 0x56])
    })
  })
})
