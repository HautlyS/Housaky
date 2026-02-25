# Housaky vs OpenClaw: Análise Comparativa

## Visão Geral

| Aspecto | Housaky | OpenClaw |
|---------|---------|----------|
| **Linguagem** | Rust (100%) | TypeScript/Node.js |
| **Tamanho Binário** | ~3.4MB | ~28MB (dist) |
| **RAM** | <5MB | >1GB |
| **Startup** | <10ms | >500s |
| **Custo Hardware** | $10 (qualquer hardware) | Mac Mini $599 |
| **Extensibilidade** | Traits (Rust) | Hooks + Plugins (TypeScript) |

---

## 1. HOOKS - Feature Ausente no Housaky

### O que é o Sistema de Hooks do OpenClaw

O OpenClaw possui um sistema robusto de hooks que permite interceptar e estender eventos do agente:

#### Tipos de Eventos Suportados

```typescript
// src/hooks/internal-hooks.ts
export type InternalHookEventType = "command" | "session" | "agent" | "gateway" | "message";
```

#### Hooks Bundled (Incluídos)

1. **session-memory** - Salva contexto da sessão quando `/new` ou `/reset` é executado
2. **boot-md** - Carrega arquivos bootstrap extras no startup
3. **bootstrap-extra-files** - Adiciona arquivos extras ao bootstrap
4. **command-logger** - Log de comandos executados
5. **llm-slug-generator** - Gera slugs descritivos via LLM

#### Hooks de Eventos

- `command:*` - Eventos de comando
- `command:new` - Comando /new executado
- `command:reset` - Comando /reset executado
- `session:*` - Eventos de sessão
- `session:start` - Sessão iniciada
- `session:end` - Sessão finalizada
- `agent:*` - Eventos de agente
- `agent:bootstrap` - Agente sendo inicializado
- `gateway:*` - Eventos do gateway
- `gateway:startup` - Gateway iniciando
- `message:*` - Mensagens
- `message:received` - Mensagem recebida
- `message:sent` - Mensagem enviada

### Implementação Recomendada para Housaky

```rust
// src/hooks/mod.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub event_type: HookEventType,
    pub action: String,
    pub session_key: String,
    pub context: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HookEventType {
    Command,
    Session,
    Agent,
    Gateway,
    Message,
}

pub type HookHandler = Arc<dyn Fn(HookEvent) -> BoxFuture<'static, Result<(), HookError>> + Send + Sync>;

#[derive(Debug)]
pub struct HookError {
    pub message: String,
    pub event_type: HookEventType,
    pub action: String,
}

pub struct HookRegistry {
    handlers: Arc<RwLock<HashMap<String, Vec<HookHandler>>>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, event_key: &str, handler: HookHandler) {
        let mut handlers = self.handlers.write().await;
        handlers.entry(event_key.to_string()).or_default().push(handler);
    }

    pub async fn trigger(&self, event: HookEvent) -> Result<Vec<String>, HookError> {
        let mut all_messages = Vec::new();
        
        let handlers = self.handlers.read().await;
        
        // Get type handlers
        let type_key = format!("{:?}", event.event_type).to_lowercase();
        let type_handlers = handlers.get(&type_key).cloned().unwrap_or_default();
        
        // Get specific handlers
        let specific_key = format!("{}:{}", type_key, event.action);
        let specific_handlers = handlers.get(&specific_key).cloned().unwrap_or_default();
        
        // Execute all handlers
        for handler in type_handlers.into_iter().chain(specific_handlers) {
            match handler(event.clone()).await {
                Ok(messages) => all_messages.extend(messages),
                Err(e) => {
                    tracing::error!("Hook error [{}:{}]: {}", event.event_type.as_str(), event.action, e.message);
                }
            }
        }
        
        Ok(all_messages)
    }
}

// Example hook implementations
pub mod builtins {
    use super::*;
    
    pub fn session_memory_handler() -> HookHandler {
        Arc::new(move |event: HookEvent| {
            Box::pin(async move {
                if event.event_type == HookEventType::Command && 
                   (event.action == "new" || event.action == "reset") {
                    // Save session to memory
                    tracing::debug!("Session memory hook triggered for {:?}", event.action);
                }
                Ok(vec![])
            })
        })
    }
}
```

### Configuração do Hook

```toml
# ~/.housaky/config.toml

[hooks]
enabled = true

[[hooks.bundled]]
name = "session-memory"
enabled = true

[[hooks.bundled]]
name = "boot-md"
enabled = true
```

---

## 2. PLUGINS / EXTENSIONS

### Sistema de Plugins do OpenClaw

O OpenClaw possui um sistema de plugins robusto:

```
extensions/
├── telegram/          # Channel plugin
├── discord/           # Channel plugin  
├── slack/             # Channel plugin
├── whatsapp/          # Channel plugin
├── signal/            # Channel plugin
├── matrix/            # Channel plugin
├── msteams/           # Channel plugin
├── zalo/              # Channel plugin
├── nostr/             # Channel plugin
├── bluebubbles/       # iMessage via BlueBubbles
├── mattermost/        # Channel plugin
├── tlon/              # Channel plugin
├── line/              # Channel plugin
├── lobster/           # Tool/plugin
├── copilot-proxy/     # Proxy plugin
└── memory-core/       # Memory plugin
```

#### Estrutura de um Plugin

```json
// extensions/telegram/openclaw.plugin.json
{
  "id": "telegram",
  "name": "Telegram",
  "version": "1.0.0",
  "description": "Telegram channel support",
  "channel": true,
  "permissions": ["webhooks"]
}
```

### Implementação Recomendada para Housaky

```rust
// src/plugins/mod.rs

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub channel: Option<bool>,
    pub permissions: Option<Vec<String>>,
    pub hooks: Option<Vec<String>>,
}

pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn init(&self) -> Result<(), PluginError>;
    fn register_hooks(&self, registry: &HookRegistry) -> Result<(), PluginError>;
}

pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, Box<dyn Plugin>>>,
}

impl PluginRegistry {
    pub async fn load_plugin(&self, path: PathBuf) -> Result<(), PluginError> {
        // Load plugin manifest
        let manifest_path = path.join("openclaw.plugin.json");
        let manifest: PluginManifest = serde_json::from_str(&tokio::fs::read_to_string(manifest_path).await?)?;
        
        // Load dynamic library or WASM module
        // Register plugin
        Ok(())
    }
}
```

---

## 3. CANAIS (CHANNELS)

### Canais do OpenClaw

| Canal | Status | Housaky |
|-------|--------|---------|
| WhatsApp | ✅ Core | ✅ |
| Telegram | ✅ Core | ✅ |
| Discord | ✅ Core | ✅ |
| Slack | ✅ Core | ✅ |
| Google Chat | ✅ Core | ❌ |
| Signal | ✅ Core | ❌ |
| iMessage (BlueBubbles) | ✅ Core | ❌ |
| iMessage (legacy) | ✅ Core | ❌ |
| Microsoft Teams | ✅ Extension | ❌ |
| Matrix | ✅ Extension | ❌ |
| Zalo | ✅ Extension | ❌ |
| WebChat | ✅ Core | ❌ |
| LINE | ✅ Extension | ❌ |
| Nostr | ✅ Extension | ❌ |
| Mattermost | ✅ Extension | ❌ |
| Tlon | ✅ Extension | ❌ |

### Features de Canal do OpenClaw

1. **DM Policy** - `pairing`, `open`, `closed`
2. **Group Routing** - Mention gating, reply tags
3. **Allowlist** - Controle de acesso por usuário
4. **Typing Indicators**
5. **Presence**
6. **Media Pipeline** - Images, audio, video

---

## 4. SKILLS

### Skills do OpenClaw (~60+ skills)

```
skills/
├── trello/           # Trello integration
├── github/           # GitHub operations
├── slack/            # Slack integration
├── discord/          # Discord integration
├── notion/           # Notion integration
├── obsidian/         # Obsidian notes
├── apple-notes/      # Apple Notes
├── apple-reminders/  # Apple Reminders
├── spotify-player/   # Spotify control
├── weather/          # Weather info
├── 1password/        # 1Password integration
├── openai-whisper/  # Whisper transcription
├── openai-image-gen/ # DALL-E image generation
├── canvas/           # Canvas visual workspace
├── coding-agent/     # Coding assistance
├── skill-creator/    # Create new skills
├── session-logs/     # Session logging
├── model-usage/      # Usage tracking
├── healthcheck/      # Health monitoring
└── ...
```

### Housaky Skills (Existente)

O Housaky já possui sistema de skills em `src/skills/`:

- Claude Code integration
- TOML-based SKILL.md

### Transpile Recomendado

Converter skills TypeScript → Rust:

1. **Simple CLI wrappers** → Facilidade alta
2. **API integrations** → Medium (async/reqwest)
3. **Complex UI interactions** → Baixa prioridade

---

## 5. BROWSER CONTROL

### OpenClaw Browser

- Dedicated Chrome/Chromium via CDP
- Browser profiles
- Snapshots, actions, uploads
- Sandbox integration

### Housaky Browser

Já possui implementação básica em `src/tools/browser.rs`:

```rust
// Browser tool implementation exists
// Needs: enhanced CDP support, profile management
```

### Melhorias Recomendadas

1. **Browser Profiles** - Suporte a múltiplos perfis
2. **Cookie Storage** - Persistência de cookies
3. **NoVNC Integration** - Browser remoto
4. **Browser Extensions** - Extensões do Chrome

---

## 6. VOICE / TALK MODE

### OpenClaw

- **Voice Wake** - Wake word detection
- **Talk Mode** - Continuous conversation
- **ElevenLabs** - Voice synthesis
- **STT** - Speech to text
- **macOS/iOS/Android nodes** - Voice em dispositivos

### Housaky

**Ausente** - Não há implementação de voz

### Implementação Recomendada

```rust
// src/voice/mod.rs

pub mod wake_word {
    pub trait WakeWordDetector: Send + Sync {
        fn detect(&self, audio_data: &[i16]) -> bool;
    }
}

pub mod stt {
    pub trait SpeechToText: Send + Sync {
        async fn transcribe(&self, audio: &[u8]) -> Result<String, VoiceError>;
    }
}

pub mod tts {
    pub trait TextToSpeech: Send + Sync {
        async fn speak(&self, text: &str) -> Result<Vec<u8>, VoiceError>;
    }
}
```

**Transpile para Rust:**
- **STT**: Whisper.cpp bindings (já existe partial)
- **TTS**: Coqui TTS, ElevenLabs API wrapper
- **Wake Word**: Preciso, snowboy ou vosk

---

## 7. MACOS / IOS / ANDROID APPS

### OpenClaw Apps

- **macOS App** - Menu bar, Voice Wake, Talk Mode
- **iOS Node** - Canvas, Camera, Voice
- **Android Node** - Canvas, Camera, Screen

### Housaky

- **Dashboard Tauri** - Já existe (`src/dashboard/`)

---

## 8. CANVAS / A2UI

### OpenClaw Canvas

- Agent-driven visual workspace
- A2UI (Agent-to-User Interface) protocol
- Push/reset, eval, snapshot

### Housaky

**Ausente** - Não há implementação de canvas visual

---

## 9. SUBAGENTS

### OpenClaw Subagents

- Session-based isolation
- `sessions_list`, `sessions_history`, `sessions_send`
- `sessions_spawn` - Spawn new agents
- Announce queue for inter-agent communication

### Housaky AGI

Já possui子系统 em `src/housaky/`:
- Goal engine
- Meta-cognition
- Self-improvement
- Working memory

### Transpile Recomendado

Subagent spawning → Implementar como spawn de tasks assíncronas

---

## 10. CRON / SCHEDULING

### OpenClaw Cron

- Cron jobs
- Wakeups
- Webhook delivery
- Gmail Pub/Sub integration

### Housaky Cron

Já existe em `src/housaky/` e `housaky cron`:
```bash
housaky cron list
housaky cron add
housaky cron remove
```

---

## 11. SANDBOXING

### OpenClaw Sandboxing

- Docker-based
- Tool policy per session
- Filesystem scoping
- Environment sanitization

### Housaky Sandboxing

Já existe em `src/security/`:
- Docker
- Bubblewrap
- Firejail
- Landlock

---

## 12. GATEWAY WEBSOCKET

### OpenClaw Gateway

- WebSocket control plane
- ws://127.0.0.1:18789
- RPC-based agent runtime
- Tool streaming
- Block streaming

### Housaky Gateway

Já existe em `src/web_interface/`:

```rust
// Gateway endpoint types
- /health
- /pair
- /webhook
- /telegram
- /discord
- /slack
```

---

## 13. PRESENCE & TYPING

### OpenClaw

- Presence indicators
- Typing indicators
- Usage tracking

### Housaky

**Parcial** - Necessita implementação

---

## 14. AUTH PROFILES

### OpenClaw Auth Profiles

- OAuth vs API keys
- Model failover
- Auth rotation

### Housaky

**Ausente** - Não há auth profiles

---

## 15. TRANSPILE PRIORITY

### Alta Prioridade (Fácil + Alto Impacto)

1. **Hooks System** - Essencial para extensibilidade
2. **Plugin System** - Extensibilidade de canais
3. **Auth Profiles** - Multi-provider support

### Média Prioridade

4. **Voice (STT/TTS)** - Feature diferenciada
5. **Browser Profiles** - Melhora UX
6. **Presence/Typing** - Melhora UX

### Baixa Prioridade (Complexo)

7. **Canvas/A2UI** - Complexo, baixo ROI para embedded
8. **macOS/iOS/Android Apps** - Já existe dashboard
9. **Subagents** - Já existe AGI system

---

## 16. IMPLEMENTAÇÃO RECOMENDADA: HOOKS

### Passo 1: Definir Traits

```rust
// src/hooks/traits.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait Hook: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn events(&self) -> Vec<&str>;
    
    async fn handle(&self, event: HookEvent) -> Result<HookResult, HookError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub event_type: String,
    pub action: String,
    pub session_key: String,
    pub context: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    pub messages: Vec<String>,
    pub should_continue: bool,
}
```

### Passo 2: Registrar Hooks

```rust
// src/hooks/registry.rs

pub struct HookRegistry {
    hooks: RwLock<HashMap<String, Vec<Arc<dyn Hook>>>>,
}

impl HookRegistry {
    pub async fn register(&self, hook: Arc<dyn Hook>) {
        for event in hook.events() {
            self.hooks
                .write()
                .await
                .entry(event.to_string())
                .or_default()
                .push(hook.clone());
        }
    }

    pub async fn trigger(&self, event: HookEvent) -> Vec<String> {
        let handlers = self.hooks.read().await;
        let mut messages = Vec::new();
        
        // Trigger type handlers
        if let Some(hooks) = handlers.get(&event.event_type) {
            for hook in hooks {
                match hook.handle(event.clone()).await {
                    Ok(result) => messages.extend(result.messages),
                    Err(e) => tracing::error!("Hook error: {}", e),
                }
            }
        }
        
        // Trigger specific handlers
        let specific_key = format!("{}:{}", event.event_type, event.action);
        if let Some(hooks) = handlers.get(&specific_key) {
            for hook in hooks {
                match hook.handle(event.clone()).await {
                    Ok(result) => messages.extend(result.messages),
                    Err(e) => tracing::error!("Hook error: {}", e),
                }
            }
        }
        
        messages
    }
}
```

### Passo 3: Hooks Bundled

```rust
// src/hooks/bundled/mod.rs

pub mod session_memory {
    use super::*;
    
    pub struct SessionMemoryHook {
        message_count: usize,
    }
    
    impl SessionMemoryHook {
        pub fn new(message_count: usize) -> Self {
            Self { message_count }
        }
    }
    
    #[async_trait]
    impl Hook for SessionMemoryHook {
        fn id(&self) -> &str { "session-memory" }
        fn name(&self) -> &str { "Session Memory" }
        fn events(&self) -> Vec<&str> { vec!["command:new", "command:reset"] }
        
        async fn handle(&self, event: HookEvent) -> Result<HookResult, HookError> {
            // Save session to memory
            Ok(HookResult { messages: vec![], should_continue: true })
        }
    }
}

pub mod boot_md {
    // Bootstrap extra MD files
}

pub mod command_logger {
    // Log commands
}
```

### Passo 4: Configuração

```toml
# config.toml

[hooks]
enabled = true

[hooks.session-memory]
enabled = true
messages = 15

[hooks.boot-md]
enabled = true
```

---

## 17. RESUMO DE IMPLEMENTAÇÃO

| Feature | Status Housaky | Status OpenClaw | Ação |
|---------|---------------|----------------|------|
| Hooks | ❌ Ausente | ✅ Completo | Implementar |
| Plugins | ❌ Ausente | ✅ Completo | Planejar |
| Voice | ❌ Ausente | ✅ Completo | Planejar |
| Canvas | ❌ Ausente | ✅ Completo | Não priorizado |
| Auth Profiles | ❌ Ausente | ✅ Completo | Implementar |
| Browser Profiles | ⚠️ Parcial | ✅ Completo | Melhorar |
| Presence/Typing | ❌ Ausente | ✅ Completo | Implementar |
| macOS App | ⚠️ Dashboard | ✅ App completo | Manter dashboard |
| Subagents | ⚠️ AGI system | ✅ Completo | Enhancer |
| Canvas/A2UI | ❌ Ausente | ✅ Completo | Não priorizado |
| Channels | ✅ ~12 canais | ✅ ~18 canais | Adicionar |
| Skills | ⚠️ Basic | ✅ ~60 skills | Expandir |
| Sandboxing | ✅ Docker/Landlock | ✅ Docker | Manter |
| Gateway | ⚠️ REST | ✅ WebSocket | Considerar WS |

---

## 18. REFERÊNCIAS

- Hooks TypeScript: `openclaw/src/hooks/internal-hooks.ts`
- Hooks Types: `openclaw/src/hooks/types.ts`
- Session Memory: `openclaw/src/hooks/bundled/session-memory/handler.ts`
- Plugins: `openclaw/extensions/*/`
- Skills: `openclaw/skills/*/SKILL.md`

---

*Análise gerada em Fevereiro 2026*
