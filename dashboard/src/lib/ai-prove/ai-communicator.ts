import { Challenge, ChallengeResponse, challengeGenerator } from './challenge-generator'
import { computeChecksum } from './checksum-validator'

export interface A2AMessage {
  id: string
  type: 'request' | 'response' | 'challenge' | 'challenge_response'
  payload: string
  metadata?: Record<string, unknown>
  challenge?: Challenge
  challengeResponse?: ChallengeResponse
  signature?: string
  timestamp: number
}

export interface AIProveConfig {
  a2aEndpoint: string
  agentId: string
  challengeRequired: boolean
  timeoutMs: number
}

export class AICommunicator {
  private config: AIProveConfig
  private pendingChallenges: Map<number, Challenge> = new Map()
  private messageHistory: A2AMessage[] = []
  
  constructor(config: AIProveConfig) {
    this.config = config
  }
  
  async sendMessage(payload: string, metadata?: Record<string, unknown>): Promise<A2AMessage> {
    const challenge = this.config.challengeRequired ? challengeGenerator.generate() : null
    
    const message: A2AMessage = {
      id: crypto.randomUUID(),
      type: challenge ? 'challenge' : 'request',
      payload,
      metadata,
      challenge: challenge ?? undefined,
      timestamp: Date.now(),
    }
    
    if (challenge) {
      this.pendingChallenges.set(challenge.id, challenge)
    }
    
    this.messageHistory.push(message)
    
    if (this.config.a2aEndpoint) {
      try {
        const response = await this.sendToA2A(message)
        return response
      } catch (error) {
        console.error('A2A communication error:', error)
      }
    }
    
    return message
  }
  
  async respondToChallenge(challengeId: number, _result: string): Promise<ChallengeResponse> {
    const challenge = this.pendingChallenges.get(challengeId)
    if (!challenge) {
      throw new Error(`Challenge ${challengeId} not found`)
    }
    
    const startTime = performance.now()
    const computedResult = challengeGenerator.execute(challenge.inputData, challenge.operations)
    const formattedResult = challengeGenerator.formatResult(computedResult, challenge.expectedFormat)
    const endTime = performance.now()
    
    const response: ChallengeResponse = {
      challengeId,
      result: formattedResult,
      resultHex: formattedResult,
      computeTimeMs: endTime - startTime,
      tokenCount: Math.ceil(formattedResult.length / 4),
      checksum: computeChecksum(formattedResult),
      timestamp: Date.now(),
    }
    
    this.pendingChallenges.delete(challengeId)
    
    const ackMessage: A2AMessage = {
      id: crypto.randomUUID(),
      type: 'challenge_response',
      payload: JSON.stringify(response),
      challengeResponse: response,
      timestamp: Date.now(),
    }
    
    this.messageHistory.push(ackMessage)
    return response
  }
  
  verifyChallengeResponse(challenge: Challenge, response: ChallengeResponse): boolean {
    return challengeGenerator.validate(challenge, response)
  }
  
  getMessageHistory(): A2AMessage[] {
    return [...this.messageHistory]
  }
  
  clearHistory(): void {
    this.messageHistory = []
  }
  
  private async sendToA2A(message: A2AMessage): Promise<A2AMessage> {
    const response = await fetch(this.config.a2aEndpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(message),
      signal: AbortSignal.timeout(this.config.timeoutMs),
    })
    
    if (!response.ok) {
      throw new Error(`A2A request failed: ${response.status}`)
    }
    
    return response.json()
  }
}

export function createAICommunicator(config: Partial<AIProveConfig> = {}): AICommunicator {
  const defaultConfig: AIProveConfig = {
    a2aEndpoint: config.a2aEndpoint || '/api/a2a',
    agentId: config.agentId || 'housaky',
    challengeRequired: config.challengeRequired ?? true,
    timeoutMs: config.timeoutMs || 30000,
  }
  
  return new AICommunicator(defaultConfig)
}
