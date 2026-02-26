# SQLite Decentralized Encrypted Memory for Housaky AGI

> **Status**: Draft for Agent Review  
> **Created**: 2026-02-26  
> **Purpose**: Document the architecture for encrypted, decentralized SQLite memory in Housaky

---

## Executive Summary

This document outlines the architecture for implementing encrypted, decentralized memory in Housaky — a zero-overhead, fully autonomous AI assistant infrastructure written in Rust. The goal is to enable secure, peer-to-peer memory synchronization while maintaining the project's edge-first philosophy.

---

## 1. Current Architecture

### 1.1 SQLite Memory Backend

**Location**: `src/memory/sqlite.rs`

Housaky's default memory backend is SQLite with:
- **Vector storage**: Embeddings stored as BLOB with cosine similarity search
- **Keyword search**: FTS5 virtual table with BM25 scoring
- **Hybrid merge**: Weighted fusion of vector + keyword results
- **Embedding cache**: LRU-evicted cache to avoid redundant API calls
- **WAL mode**: Concurrent reads during writes, crash-safe
- **Path**: `{workspace_dir}/memory/brain.db`

**Configuration** (from `src/config/schema.rs`):
```rust
pub struct MemoryConfig {
    pub backend: String,           // "sqlite" | "lucid" | "markdown" | "none"
    pub embedding_provider: String, // "none" | "openai" | "custom:URL"
    pub embedding_model: String,
    pub vector_weight: f64,
    pub keyword_weight: f64,
    // ... more fields
}
```

### 1.2 Existing Decentralization: Federation System

**Location**: `src/housaky/federation/`

Housaky already has a **peer-to-peer federation system** for distributed cognition:

| Component | Description |
|-----------|-------------|
| `FederationTransport` | P2P communication via libp2p or HTTP/WS |
| `KnowledgeDelta` | CRDT-inspired delta sync between peers |
| `SyncMode` | Three modes: KnowledgeGraph, Beliefs, Full |
| `Trust Scoring` | Reject deltas from untrusted peers (configurable threshold) |
| `Peer Discovery` | Registration, health monitoring, latency tracking |

**Key Types**:
```rust
pub struct KnowledgeDelta {
    pub source_peer: String,
    pub timestamp: DateTime<Utc>,
    pub version: u64,
    pub additions: Vec<KnowledgeEntry>,
    pub modifications: Vec<KnowledgeEntry>,
    pub deletions: Vec<String>,
}
```

### 1.3 Collective Memory (Swarm)

**Location**: `src/housaky/swarm/collective_memory.rs`

SQLite-backed shared memory for multi-agent scenarios:
- Conflict resolution (HigherConfidenceWins, Merge, KeepExisting, TakeIncoming)
- Version tracking with conflict logging
- Vector search support

---

## 2. Encryption Options for SQLite

### 2.1 Available Solutions

| Solution | Type | Pros | Cons |
|----------|------|------|------|
| **SQLCipher** | Full-database encryption | Industry standard, well-tested, zero-config | GPL license (requires commercial license for closed-source) |
| **rusqlite with bundled SQLCipher** | Full-database encryption | Drop-in replacement, same API | Requires feature flag, GPL |
| **AES-encrypted file layer** | Transparent file encryption | No SQLite changes needed, MIT/Apache | Manual implementation, potential performance overhead |
| **OS-level encryption** | Filesystem encryption | Transparent, no code changes | Platform-specific (fscrypt, LUKS, FileVault) |
| **column-level encryption** | Application-level | Fine-grained control | Complexity, query performance impact |

### 2.2 Recommendation: SQLCipher

**Rationale**:
- **Best integration**: Direct SQLite replacement, no application logic changes
- **Well-maintained**: Active project with security audits
- **Zero overhead at rest**: Encryption/decryption transparent to queries
- **Production proven**: Used by thousands of applications

**Implementation path**:
```toml
# Cargo.toml
rusqlite = { version = "0.31", features = ["sqlcipher"] }
```

```rust
// src/memory/sqlite.rs - proposed modification
use rusqlite::Connection;

pub fn open_encrypted(path: &Path, key: &[u32]) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(&format!(
        "PRAGMA key = 'x\"{}\"';",
        key.iter().map(|b| format!("{:02x}", b)).collect::<String>()
    ))?;
    Ok(conn)
}
```

### 2.3 Key Management Strategy

For an AGI system, key management must balance security with usability:

| Scenario | Key Source | Recovery |
|----------|------------|----------|
| Single device | PBKDF2 from user password | Password reset |
| Multi-device | Derived key from master secret + device ID | Sync master key |
| Federation | Per-session ephemeral keys + long-term identity | Revocation list |
| Zero-device | Key escrow with threshold signing | Multi-party recovery |

---

## 3. Proposed Architecture: Encrypted Decentralized SQLite

### 3.1 High-Level Design

```
┌─────────────────────────────────────────────────────────────────┐
│                        Housaky Agent                            │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐ │
│  │   AGI Core  │───▶│  Memory     │───▶│  SQLite + SQLCipher │ │
│  │  (Reasoning)│    │  (Interface)│    │  (Encrypted Brain)  │ │
│  └─────────────┘    └─────────────┘    └─────────────────────┘ │
│         │                                        │              │
│         ▼                                        ▼              │
│  ┌─────────────┐                       ┌─────────────────────┐ │
│  │  Federation │◀───────────────────────│  P2P Sync (libp2p)  │ │
│  │  (P2P Mesh) │                       │  - Encrypted deltas │ │
│  └─────────────┘                       │  - Trust scoring    │ │
│                                         └─────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Encryption Layers

| Layer | What | How |
|-------|------|-----|
| **Database at rest** | Full brain.db | SQLCipher PRAGMA key |
| **P2P transport** | Delta sync packets | TLS 1.3 or Noise protocol |
| **Memory export** | Snapshots, backups | AES-256-GCM with key derived from master key |
| **Key storage** | Encryption keys | OS keyring (keychain, keyctl) or encrypted file |

### 3.3 Integration Points

**Files to modify**:
1. `src/memory/sqlite.rs` — Add SQLCipher support with feature flag
2. `src/memory/mod.rs` — Add encrypted factory option
3. `src/config/schema.rs` — Add encryption config fields
4. `src/housaky/federation/transport.rs` — Add E2E encryption for deltas
5. `src/security/` — Integrate key management

**New modules to create**:
- `src/memory/crypto.rs` — Key derivation, encryption primitives
- `src/security/keyring.rs` — OS keyring integration

### 3.4 Federation Sync with Encryption

**Current flow** (unencrypted):
```
Agent A  ──delta (JSON)──▶  Agent B
         ◀──ack──────────
```

**Proposed flow** (encrypted):
```
Agent A  ──encrypted delta (Sealed box)──▶  Agent B
              │                                │
              │    Verify peer identity        │
              │    Decrypt with session key    │
              ▼                                ▼
         ◀──encrypted ack─────────────────
```

**Delta format**:
```rust
struct EncryptedDelta {
    pub source_peer: String,
    pub nonce: [u8; 24],
    pub ciphertext: Vec<u8>,  // XChaCha20-Poly1305
    pub ephemeral_public_key: [u8; 32],  // For forward secrecy
}
```

---

## 4. Implementation Roadmap

### Phase 1: Encryption Foundation
- [ ] Add SQLCipher feature flag to rusqlite dependency
- [ ] Implement key derivation (PBKDF2/Argon2)
- [ ] Add `memory.encryption_key` config (path to key file or env var)
- [ ] Create encrypted open path in `sqlite.rs`

### Phase 2: Federation Encryption
- [ ] Add XChaCha20-Poly1305 for delta encryption
- [ ] Implement peer identity keys (Ed25519)
- [ ] Add encrypted sync mode to federation config
- [ ] Integrate with existing trust scoring

### Phase 3: Key Management
- [ ] OS keyring integration (Linux: keyctl, macOS: keychain, Windows: DPAPI)
- [ ] Key rotation support
- [ ] Federation key exchange protocol

### Phase 4: Testing & Hardening
- [ ] Fuzz testing for encryption layer
- [ ] Performance benchmarks (encryption overhead)
- [ ] Recovery procedures documentation

---

## 5. Security Considerations

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Stolen device | SQLCipher + OS keyring (biometric unlock) |
| Network interception | E2E encrypted P2P (XChaCha20-Poly1305) |
| Malicious peer | Trust scoring + signature verification |
| Key compromise | Forward secrecy via ephemeral keys |
| Memory dump | Encrypted database at rest |

### Performance Impact

- **SQLCipher**: ~5-15% write overhead, ~0-5% read overhead (depends on page size)
- **P2P encryption**: ~1-3ms per delta (negligible for sync interval of 300s)
- **Key derivation**: ~100ms on first boot (can be cached in memory)

---

## 6. Open Questions for Review

1. **License**: SQLCipher is GPL. Should we use it directly, or find an Apache2 alternative like `sqlx` with `sqlcipher` feature?

2. **Key distribution**: For federated sync, how should peers exchange session keys?
   - Option A: Diffie-Hellman per session
   - Option B: Pre-shared long-term keys
   - Option C: Hybrid (long-term identity key + ephemeral session key)

3. **Recovery**: If all devices are lost, should there be a recovery mechanism?
   - Threshold secret sharing?
   - Social recovery?
   - Accept data loss as feature?

4. **Backward compatibility**: Should encrypted mode be opt-in or the default? If default, how to handle migration from unencrypted?

5. **Memory-encrypted mode**: For extreme edge cases (RAM-constrained), consider SQLCipher's in-memory mode with encrypted persistence.

---

## 7. References

- [SQLCipher Documentation](https://www.zetetic.net/sqlcipher/)
- [rusqlite SQLCipher bindings](https://docs.rs/rusqlite/0.31.0/rusqlite/)
- [Housaky Federation System](src/housaky/federation/mod.rs)
- [SQLite FTS5 Documentation](https://www.sqlite.org/fts5.html)
- [XChaCha20-Poly1305 (RustCrypto)](https://github.com/RustCrypto/AEADs)

---

## Appendix A: Current SQLite Schema

```sql
-- Core memory table
CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    embedding BLOB,
    category TEXT NOT NULL,
    importance REAL NOT NULL DEFAULT 0.5,
    created_at TEXT NOT NULL,
    accessed_at TEXT NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    metadata_json TEXT
);

-- Full-text search
CREATE VIRTUAL TABLE memories_fts USING fts5(
    content,
    category,
    content='memories',
    content_rowid='rowid'
);

-- Triggers for FTS sync
CREATE TRIGGER memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, content, category)
    VALUES (NEW.rowid, NEW.content, NEW.category);
END;
```

---

## Appendix B: Federation Delta Schema

```rust
// Current unencrypted delta
pub struct KnowledgeDelta {
    pub source_peer: String,
    pub timestamp: DateTime<Utc>,
    pub version: u64,
    pub additions: Vec<KnowledgeEntry>,
    pub modifications: Vec<KnowledgeEntry>,
    pub deletions: Vec<String>,
}

// Proposed encrypted delta
pub struct EncryptedDelta {
    pub source_peer: String,
    pub source_peer_public_key: [u8; 32],
    pub nonce: [u8; 24],
    pub ciphertext: Vec<u8>,  // Serialized KnowledgeDelta encrypted
    pub signature: [u8; 64],  // Ed25519 signature
    pub timestamp: DateTime<Utc>,
}
```

---

*End of Document*
