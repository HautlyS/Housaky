# Housaky Feature Comparison & Improvement Plan

## Executive Summary

This document compares Housaky against OpenClaw, Claude Code, and OpenCode to identify missing features and improvements.

---

## 1. Channel Comparison

### Current Housaky Channels (11)
| Channel | Implementation | Notes |
|---------|----------------|-------|
| CLI | ✅ Built-in | Terminal interface |
| Telegram | ✅ Grammy framework | Full support |
| Discord | ✅ Serenity | Full support |
| Slack | ✅ Bolt-style | Full support |
| Matrix | ✅ Matrix SDK | Full support |
| iMessage | ✅ macOS only | AppleScript-based |
| WhatsApp | ⚠️ Meta API only | **NEEDS QR sync mode** |
| Email | ✅ SMTP/IMAP | Full support |
| IRC | ✅ IRC protocol | Full support |
| Lark | ✅ Lark API | Full support |
| DingTalk | ✅ DingTalk API | Full support |

### OpenClaw Channels (22+)
| Channel | Housaky | OpenClaw | Priority |
|---------|---------|----------|----------|
| WhatsApp | Meta API only | Meta API + Baileys QR | **HIGH** |
| Signal | ❌ Missing | ✅ signal-cli | MEDIUM |
| LINE | ❌ Missing | ✅ @line/bot-sdk | MEDIUM |
| WebChat | ❌ Missing | ✅ Built-in WebSocket | **HIGH** |
| BlueBubbles | ❌ Missing | ✅ iMessage via server | LOW |
| Microsoft Teams | ❌ Missing | ✅ Bot Framework | MEDIUM |
| Mattermost | ❌ Missing | ✅ API | LOW |
| Nextcloud Talk | ❌ Missing | ✅ API | LOW |
| Nostr | ❌ Missing | ✅ Protocol | LOW |
| Twitch | ❌ Missing | ✅ IRC-based | LOW |
| Voice Call | ❌ Missing | ✅ Twilio | MEDIUM |
| Zalo | ❌ Missing | ✅ API | LOW |

---

## 2. WhatsApp Improvement Plan

### Current Implementation (Meta API)
- Uses WhatsApp Business Cloud API
- Requires Meta developer account
- Webhook-based message receiving
- Official, but complex setup

### New Implementation (QR Code Sync)
- Use Baileys-like approach (WhatsApp Web protocol)
- QR code pairing like OpenClaw
- No Meta developer account needed
- Personal WhatsApp accounts supported

### Proposed Architecture

```rust
pub enum WhatsAppMode {
    /// Official Meta WhatsApp Business API
    BusinessApi {
        access_token: String,
        phone_number_id: String,
        verify_token: String,
    },
    /// WhatsApp Web sync via QR code (like OpenClaw)
    WebSync {
        auth_dir: PathBuf,
        session_name: String,
    },
}
```

---

## 3. Missing CLI Commands

### Housaky Commands (Current)
- `onboard`, `agent`, `gateway`, `daemon`, `run`, `service`, `doctor`, `status`
- `dashboard`, `cron`, `models`, `keys`, `channel`, `integrations`, `skills`
- `migrate`, `hardware`, `peripheral`, `tui`, `config`
- AGI commands: `init`, `heartbeat`, `tasks`, `review`, `improve`, etc.

### OpenClaw Commands (Missing in Housaky)
| Command | Description | Priority |
|---------|-------------|----------|
| `logs` | Tail gateway logs | MEDIUM |
| `system` | System events, heartbeat | MEDIUM |
| `approvals` | Manage exec approvals | **HIGH** |
| `nodes` | Node pairing/commands | LOW |
| `devices` | Device pairing | MEDIUM |
| `sandbox` | Manage sandbox containers | **HIGH** |
| `qr` | Generate iOS pairing QR | LOW |
| `pairing` | Secure DM pairing | **HIGH** |
| `security audit` | Comprehensive audit | **HIGH** |
| `secrets` | Secrets management | MEDIUM |
| `message send/broadcast` | Send messages via channels | MEDIUM |
| `directory` | Lookup contact/group IDs | LOW |
| `update` | Self-update command | MEDIUM |

### Claude Code Commands (Missing in Housaky)
| Command | Description | Priority |
|---------|-------------|----------|
| `mcp` | MCP server management | **HIGH** |
| `hooks` | Hook management | **HIGH** |
| `permissions` | Permission rules | **HIGH** |
| `stats` | Usage statistics | MEDIUM |
| `session` | Session management | MEDIUM |

### OpenCode Commands (Missing in Housaky)
| Command | Description | Priority |
|---------|-------------|----------|
| `acp` | Agent Client Protocol | MEDIUM |
| `serve` | HTTP server mode | MEDIUM |
| `web` | Web interface | MEDIUM |
| `export/import` | Session data | LOW |

---

## 4. Feature Comparison Matrix

### Core Features
| Feature | Housaky | OpenClaw | Claude Code | OpenCode |
|---------|---------|----------|-------------|----------|
| **Language** | Rust | TypeScript | TypeScript | TypeScript |
| **AGI Core** | ✅ Full | ❌ | ❌ | ❌ |
| **Self-Modification** | ✅ Full | ❌ | ❌ | ❌ |
| **Quantum Computing** | ✅ Full | ❌ | ❌ | ❌ |
| **Multi-Agent** | ✅ Kowalski | Limited | Subagents | Subagents |
| **MCP Support** | ❌ | Limited | ✅ Full | ✅ Full |
| **Hooks System** | ✅ Basic | ✅ Advanced | ✅ Advanced | ❌ |
| **Plugin System** | ✅ Skills | ✅ Plugins | ✅ Plugins | Limited |
| **Sandbox Mode** | ❌ | ✅ Docker | ✅ | Limited |
| **Permission System** | Basic | Basic | ✅ Advanced | ✅ Advanced |
| **Session Forking** | ❌ | ❌ | ✅ | ✅ |
| **Vim Mode** | ❌ | ❌ | ✅ | ❌ |
| **Desktop App** | ❌ | ❌ | ❌ | ✅ Tauri |
| **Web Interface** | ❌ | ✅ WebChat | ❌ | ✅ |
| **Client/Server** | ❌ | Gateway | ❌ | ✅ |
| **ACP Protocol** | ❌ | ✅ | ❌ | ✅ |
| **Tailscale** | ❌ | ✅ | ❌ | ❌ |
| **Voice Wake** | ❌ | ✅ macOS/iOS | ❌ | ❌ |

### Memory System
| Feature | Housaky | OpenClaw | Claude Code | OpenCode |
|---------|---------|----------|-------------|----------|
| SQLite | ✅ | ❌ | ❌ | ✅ |
| Vector Search | ✅ | ✅ | ❌ | ❌ |
| LanceDB | ❌ | ✅ Plugin | ❌ | ❌ |
| Markdown | ✅ | ✅ | ✅ CLAUDE.md | ✅ |
| Semantic Search | ✅ | ✅ | ❌ | ❌ |
| Embedding Providers | Multiple | Multiple | ❌ | ❌ |

### Provider Support
| Provider | Housaky | OpenClaw | Claude Code | OpenCode |
|----------|---------|----------|-------------|----------|
| OpenRouter | ✅ | ❌ | ❌ | ✅ |
| Anthropic | ✅ | ✅ | ✅ Primary | ✅ |
| OpenAI | ✅ | ✅ | ❌ | ✅ |
| Google Gemini | ✅ | ✅ | ❌ | ✅ |
| AWS Bedrock | ❌ | ✅ | ✅ | ✅ |
| Azure OpenAI | ❌ | ✅ | ❌ | ✅ |
| Groq | ❌ | ✅ | ❌ | ✅ |
| Mistral | ❌ | ✅ | ❌ | ✅ |
| Ollama | ✅ | ✅ | ❌ | ✅ |
| Local Llama | ❌ | ✅ | ❌ | ✅ |
| GitHub Copilot | ❌ | ✅ | ❌ | ✅ |
| Modal | ✅ | ❌ | ❌ | ❌ |
| Custom OpenAI | ✅ | ✅ | ❌ | ✅ |

---

## 5. Priority Improvements

### Phase 1: Critical (Immediate)
1. **WhatsApp QR Sync Mode** - Add Baileys-style QR pairing
2. **MCP Support** - Full Model Context Protocol
3. **Sandbox Mode** - Docker isolation for commands
4. **Permission System** - Advanced allow/deny rules
5. **Hooks Enhancement** - PreToolUse, PostToolUse, etc.
6. **Security Audit Command** - Comprehensive security checks

### Phase 2: Important (Next Sprint)
1. **WebChat Channel** - Built-in WebSocket chat
2. **Signal Channel** - Via signal-cli
3. **LINE Channel** - Via @line/bot-sdk
4. **Message Commands** - `message send/broadcast`
5. **Approvals System** - Execution approval workflow
6. **Pairing System** - Secure DM pairing

### Phase 3: Enhancement (Future)
1. **Client/Server Architecture** - Remote control
2. **Desktop App** - Tauri-based
3. **Web Interface** - Browser-based access
4. **Voice Wake** - Wake word detection
5. **Tailscale Integration** - Zero-config remote
6. **Session Forking** - Branch conversations

---

## 6. Implementation Order

### Week 1-2: WhatsApp Dual-Mode
```
src/channels/whatsapp/
├── mod.rs           # Main module with mode selection
├── business_api.rs  # Existing Meta API implementation
├── web_sync.rs      # NEW: QR code sync implementation
└── types.rs         # Shared types
```

### Week 2-3: MCP Support
```
src/mcp/
├── mod.rs           # MCP module
├── server.rs        # MCP server implementation
├── client.rs        # MCP client for external servers
├── tools.rs         # Tool discovery and execution
└── prompts.rs       # Prompt integration
```

### Week 3-4: Hooks Enhancement
```
src/hooks/
├── mod.rs           # Existing
├── types.rs         # Enhanced event types
├── rules.rs         # Existing markdown rules
├── executor.rs      # Hook execution engine
└── builtins/        # Bundled hooks
    ├── session_memory.rs
    ├── command_logger.rs
    └── security_guard.rs
```

### Week 4-5: Permission System
```
src/security/
├── mod.rs           # Existing
├── permissions.rs   # NEW: Permission rules engine
├── policy.rs        # Existing security policy
└── approvals.rs     # NEW: Execution approvals
```

---

## 7. Configuration Schema Additions

### WhatsApp Configuration
```toml
[channels_config.whatsapp]
# Mode selection
mode = "web_sync"  # or "business_api"

# Business API mode (existing)
access_token = ""
phone_number_id = ""
verify_token = ""
allowed_numbers = ["*"]

# Web Sync mode (new)
[channels_config.whatsapp.web_sync]
auth_dir = "~/.housaky/whatsapp_auth"
session_name = "default"
dm_policy = "pairing"  # or "open"
group_policy = "mention"  # or "open", "allowlist"
```

### MCP Configuration
```toml
[mcp]
enabled = true

[mcp.servers.database]
type = "stdio"
command = "/path/to/mcp-server"
args = ["--config", "db.json"]

[mcp.servers.asana]
type = "sse"
url = "https://mcp.asana.com/sse"
```

### Permissions Configuration
```toml
[permissions]
# Tool permissions
ask = ["shell"]
deny = ["browser_open"]

[permissions.tools.shell]
allow = ["git", "npm", "cargo"]
deny = ["rm -rf /"]

[permissions.files]
allow = ["src/**", "tests/**"]
deny = [".env", "credentials.json"]
```

---

## 8. Summary

Housaky has unique strengths:
- Full AGI core with goal engine, reasoning, knowledge graph
- Self-modification capabilities
- Quantum computing integration
- Built in Rust for performance

Key gaps to address:
1. WhatsApp needs QR sync mode (like OpenClaw)
2. MCP protocol support (like Claude Code/OpenCode)
3. Advanced permission system (like Claude Code)
4. Sandbox/isolation mode (like OpenClaw/Claude Code)
5. More channels (Signal, LINE, WebChat)

This plan prioritizes features that would have the highest impact on user experience and feature parity with the analyzed projects.
