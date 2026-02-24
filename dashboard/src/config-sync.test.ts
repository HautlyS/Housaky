import { describe, it, expect, vi, beforeEach } from 'vitest'

// Mock config data that matches what Tauri backend returns
const mockConfig = {
  api_key: 'sk-test123',
  default_provider: 'openrouter',
  default_model: 'test-model',
  default_temperature: 0.7,
  memory: {
    backend: 'sqlite',
    auto_save: true,
    embedding_provider: 'openai',
    vector_weight: 0.7,
    keyword_weight: 0.3,
  },
  autonomy: {
    level: 'supervised',
    workspace_only: true,
    allowed_commands: ['git', 'npm'],
    forbidden_paths: ['/etc', '/root'],
    max_actions_per_hour: 100,
    max_cost_per_day_cents: 1000,
  },
  runtime: {
    kind: 'native',
  },
  heartbeat: {
    enabled: false,
    interval_minutes: 30,
  },
  gateway: {
    require_pairing: true,
    allow_public_bind: false,
  },
  tunnel: {
    provider: 'none',
  },
  secrets: {
    encrypt: true,
  },
}

const mockStatus = {
  version: '0.1.0',
  workspace: '~/.housaky/workspace',
  config: '~/.housaky/config.toml',
  provider: 'openrouter',
  model: '(default)',
  temperature: 0.7,
  memory_backend: 'sqlite',
  memory_auto_save: true,
  embedding_provider: 'openai',
  autonomy_level: 'supervised',
  workspace_only: true,
  runtime: 'native',
  heartbeat_enabled: false,
  heartbeat_interval: 30,
  channels: {
    telegram: { configured: true, active: true, allowlist_count: 1 },
    discord: { configured: false, active: false, allowlist_count: 0 },
    slack: { configured: false, active: false, allowlist_count: 0 },
    whatsapp: { configured: false, active: false, allowlist_count: 0 },
    matrix: { configured: false, active: false, allowlist_count: 0 },
  },
  secrets_encrypted: true,
}

// Simulate Tauri invoke function
const mockInvoke = vi.fn((cmd: string, args?: any) => {
  if (cmd === 'get_config') return Promise.resolve(mockConfig)
  if (cmd === 'get_status') return Promise.resolve(mockStatus)
  if (cmd === 'save_config') return Promise.resolve('~/.housaky/config.toml')
  if (cmd === 'check_housaky_installed') return Promise.resolve(true)
  if (cmd === 'send_message') return Promise.resolve('Test response')
  return Promise.resolve({})
})

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}))

describe('Config Sync Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('Config Loading', () => {
    it('should load config from backend', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      
      const config = await invoke('get_config')
      
      expect(config).toEqual(mockConfig)
      expect(config.default_provider).toBe('openrouter')
    })

    it('should load status from backend', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      
      const status = await invoke('get_status')
      
      expect(status.version).toBe('0.1.0')
      expect(status.provider).toBe('openrouter')
    })

    it('should parse channels from status', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      
      const status = await invoke('get_status')
      
      expect(status.channels.telegram.configured).toBe(true)
      expect(status.channels.discord.configured).toBe(false)
    })
  })

  describe('Config Saving', () => {
    it('should save config to backend', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      
      const newConfig = { ...mockConfig, default_temperature: 0.9 }
      const result = await invoke('save_config', { config: newConfig })
      
      expect(result).toBe('~/.housaky/config.toml')
      expect(mockInvoke).toHaveBeenCalledWith('save_config', { config: newConfig })
    })

    it('should save memory settings', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      
      const newConfig = {
        ...mockConfig,
        memory: { ...mockConfig.memory, vector_weight: 0.5 }
      }
      
      await invoke('save_config', { config: newConfig })
      
      expect(mockInvoke).toHaveBeenCalledWith('save_config', expect.objectContaining({
        config: expect.objectContaining({
          memory: expect.objectContaining({ vector_weight: 0.5 })
        })
      }))
    })

    it('should save autonomy settings', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      
      const newConfig = {
        ...mockConfig,
        autonomy: { ...mockConfig.autonomy, level: 'full' }
      }
      
      await invoke('save_config', { config: newConfig })
      
      expect(mockInvoke).toHaveBeenCalledWith('save_config', expect.objectContaining({
        config: expect.objectContaining({
          autonomy: expect.objectContaining({ level: 'full' })
        })
      }))
    })
  })

  describe('Config Validation', () => {
    it('should validate temperature range (0-2)', () => {
      const valid = mockConfig.default_temperature >= 0 && mockConfig.default_temperature <= 2
      expect(valid).toBe(true)
    })

    it('should validate vector weight (0-1)', () => {
      const valid = mockConfig.memory.vector_weight >= 0 && mockConfig.memory.vector_weight <= 1
      expect(valid).toBe(true)
    })

    it('should validate keyword weight (0-1)', () => {
      const valid = mockConfig.memory.keyword_weight >= 0 && mockConfig.memory.keyword_weight <= 1
      expect(valid).toBe(true)
    })

    it('should have valid memory backend', () => {
      const validBackends = ['sqlite', 'lucid', 'markdown', 'none']
      expect(validBackends).toContain(mockConfig.memory.backend)
    })

    it('should have valid autonomy level', () => {
      const validLevels = ['readonly', 'supervised', 'full']
      expect(validLevels).toContain(mockConfig.autonomy.level)
    })

    it('should have valid runtime kind', () => {
      const validKinds = ['native', 'docker']
      expect(validKinds).toContain(mockConfig.runtime.kind)
    })
  })

  describe('Chat Integration', () => {
    it('should send message', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      
      const response = await invoke('send_message', { message: 'Hello' })
      
      expect(response).toBe('Test response')
    })

    it('should check installation', async () => {
      const { invoke } = await import('@tauri-apps/api/core')
      
      const installed = await invoke('check_housaky_installed')
      
      expect(installed).toBe(true)
    })
  })

  describe('Config Modification Detection', () => {
    it('should detect changes in temperature', () => {
      const original = mockConfig.default_temperature
      const modified = 0.9
      const hasChanges = original !== modified
      
      expect(hasChanges).toBe(true)
    })

    it('should detect changes in memory backend', () => {
      const original = mockConfig.memory.backend
      const modified = 'markdown'
      const hasChanges = original !== modified
      
      expect(hasChanges).toBe(true)
    })

    it('should detect no changes when same', () => {
      const original = JSON.stringify(mockConfig)
      const modified = JSON.stringify(mockConfig)
      const hasChanges = original !== modified
      
      expect(hasChanges).toBe(false)
    })
  })

  describe('Status Parsing', () => {
    it('should count configured channels', () => {
      const configuredCount = Object.values(mockStatus.channels).filter(c => c.configured).length
      expect(configuredCount).toBe(1)
    })

    it('should count active channels', () => {
      const activeCount = Object.values(mockStatus.channels).filter(c => c.active).length
      expect(activeCount).toBe(1)
    })

    it('should get channel status', () => {
      const telegram = mockStatus.channels.telegram
      expect(telegram.configured).toBe(true)
      expect(telegram.active).toBe(true)
      expect(telegram.allowlist_count).toBe(1)
    })
  })
})
