# Pitfalls Research

**Domain:** Embedded AI Agent Infrastructure (Rust-based)
**Researched:** 2026-02-24
**Confidence:** MEDIUM

## Critical Pitfalls

### Pitfall 1: USB Device Enumeration Race Conditions

**What goes wrong:**
USB device discovery fails intermittently or returns stale device lists. Housaky may miss devices that are present, or report devices that have been unplugged.

**Why it happens:**
USB enumeration is asynchronous. Devices take time to settle after plug/unplug events. Without proper retry logic and state tracking, the system reads device state mid-transition.

**How to avoid:**
- Implement device refresh debouncing (wait 100-500ms after enumeration before reporting)
- Track device state changes over time, not just snapshots
- Add explicit wait states for device stabilization after hot-plug events
- Use libudev on Linux for event-driven device tracking instead of polling

**Warning signs:**
- Intermittent "device not found" errors that succeed on retry
- Device list includes devices that were unplugged minutes ago
- Flaky behavior on USB hubs or slow enumeration devices

**Phase to address:** Hardware Integration Phase - USB Discovery

---

### Pitfall 2: STM32/Nucleo Flash Corruption from Interrupted Programming

**What goes wrong:**
Firmware flashing partially completes, leaving the MCU in an unbootable state. The device becomes unrecoverable without external programmer intervention.

**Why it happens:**
- Power loss during flash operation
- USB cable disconnect mid-program
- No atomic write verification
- Missing bootloader mode handling (BOOT0 pin)

**How to avoid:**
- Implement verify-after-write with rollback on failure
- Detect bootloader mode requirements per MCU family
- Support ST-LINK recovery via probe-rs
- Add firmware checksum validation before activating new image
- Support dual-bank flash devices for atomic swap

**Warning signs:**
- "Flash successful" but device doesn't boot
- ST-LINK connection fails after programming attempt
- Device enters mysterious "no response" state

**Phase to address:** Hardware Integration Phase - STM32/Nucleo Flashing

---

### Pitfall 3: GPIO Pin Permission Hell on Linux

**What goes wrong:**
GPIO operations fail with permission denied errors even when user is in correct groups. Raspberry Pi GPIO becomes unusable without root.

**Why it happens:**
- Linux kernel moved from sysfs to libgpiod, breaking old access patterns
- `/dev/gpiochip*` permissions differ across Raspberry Pi OS versions
- Container environments lack GPIO access by default
- udev rules not properly configured

**How to avoid:**
- Detect and support both sysfs (legacy) and libgpiod (modern) backends
- Generate and install udev rules automatically during setup
- Document container GPIO passthrough requirements
- Provide clear error messages when permissions are missing

**Warning signs:**
- "Permission denied" on GPIO operations despite correct group membership
- Works as root, fails as regular user
- Different behavior on Raspberry Pi OS Bookworm vs. Bullseye

**Phase to address:** Hardware Integration Phase - Raspberry Pi GPIO

---

### Pitfall 4: Goal Engine Infinite Loop / Resource Exhaustion

**What goes wrong:**
AGI goal engine enters infinite planning loops, consuming CPU/memory until system crashes or becomes unresponsive.

**Why it happens:**
- Goal decomposition has no depth limits
- No cycle detection in goal graphs
- Cognitive modules can re-trigger same goals indefinitely
- Self-improvement loop modifies goal engine to increase goal generation

**How to avoid:**
- Implement hard limits on goal decomposition depth (e.g., max 10 levels)
- Add cycle detection using goal ID tracking
- Enforce resource budgets per goal (time, tokens, memory)
- Make self-improvement opt-in with strict boundaries
- Add automatic goal suspension after N consecutive failures

**Warning signs:**
- CPU usage spikes to 100% with no user output
- Memory grows unbounded during "thinking"
- Logs show repeated goal decomposition without resolution

**Phase to address:** AGI Capabilities Phase - Goal Engine

---

### Pitfall 5: Cognitive Module Memory Corruption

**What goes wrong:**
Cognitive modules store corrupted or inconsistent state, causing unpredictable agent behavior. The agent "forgets" learned behaviors or adopts harmful modifications.

**Why it happens:**
- Self-modification writes malformed data to memory
- Concurrent access to cognitive state without proper synchronization
- No validation of self-improved code/behavior before activation
- Migration/rollback to broken cognitive snapshots

**How to avoid:**
- Validate all cognitive modifications against schema before storage
- Maintain immutable history of cognitive changes (append-only)
- Implement cognitive state snapshots with atomic rollback
- Add "cognitive quarantine" — new behaviors require explicit activation
- Store cognitive deltas, not full overwrites

**Warning signs:**
- Agent behavior changes unexpectedly after "learning"
- Memory stores contain invalid/garbage data
- Cannot restore agent to known-good cognitive state

**Phase to address:** AGI Capabilities Phase - Cognitive Modules

---

### Pitfall 6: Sandbox Escape via Agent Tool Abuse

**What goes wrong:**
An agent escapes the Docker/sandboxed runtime and gains access to the host system, potentially exfiltrating secrets or damaging the host.

**Why it happens:**
- Insufficient syscall filtering (Docker alone is not enough)
- Host filesystem mounted incorrectly into container
- Agent has access to docker socket or privileged operations
- Tool definitions allow escape primitives (e.g., mounting volumes, escaping via /proc)

**How to avoid:**
- Use gVisor or Firecracker microVMs for production isolation
- Never mount host docker socket into agent containers
- Implement strict syscall allowlists (seccomp profile)
- Validate all tool parameters for escape patterns
- Use read-only rootfs with minimal writable mounts
- Implement network isolation (no internet for untrusted agents)

**Warning signs:**
- Agent can access files outside designated workspace
- Container has CAP_SYS_ADMIN or similar dangerous capabilities
- `/var/run/docker.sock` is accessible inside sandbox

**Phase to address:** Security Features Phase - Sandboxing

---

### Pitfall 7: Pairing Protocol Timing Attacks

**What goes wrong:**
An attacker brute-forces the 6-digit pairing code within the time window, gaining unauthorized access to the agent.

**Why it happens:**
- No rate limiting on pairing attempts
- Timing window too large (e.g., infinite)
- No account lockout after failed attempts
- Code entropy insufficient

**How to avoid:**
- Implement exponential backoff after failed pairing attempts
- Add maximum attempt limits (e.g., 10 attempts then lockout)
- Use 8+ character codes with mixed case + numbers
- Invalidate pairing codes after successful exchange
- Add tamper detection that clears pairing state after physical access

**Warning signs:**
- Logs show rapid failed pairing attempts
- No exponential backoff visible in authentication logs
- Pairing codes remain valid indefinitely

**Phase to address:** Security Features Phase - Pairing

---

### Pitfall 8: Workspace Scoping Bypass via Symlink Escape

**What goes wrong:**
Agent accesses files outside the intended workspace by creating or following symlinks that escape the configured directory boundary.

**Why it happens:**
- Path canonicalization happens after tool invocation, not before
- Symlinks created during operation point outside workspace
- Race condition between check and access (TOCTOU)
- Case-insensitive path handling on certain filesystems

**How to avoid:**
- Canonicalize paths before every file operation (resolve symlinks)
- Re-check canonical path is still within workspace after opening
- Block symlink creation in restricted directories
- Use O_NOFOLLOW when opening files
- Maintain allowlist of permitted workspace root directories

**Warning signs:**
- Agent can read /etc/passwd despite workspace_only=true
- Symlinks to parent directory work unexpectedly
- Path traversal works with `..` sequences

**Phase to address:** Security Features Phase - Workspace Scoping

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Skip firmware verification step | Faster flashing workflow | Bricked devices, support burden | Never - verify is mandatory |
| Use root GPIO without udev | Works immediately | Breaks for end users, requires root | Development only |
| Disable seccomp for debugging | Easier to diagnose issues | Accidental production deployment with disabled security | Never in final build |
| Hardcode pairing code for testing | Fast iteration | Security vulnerability in production | Never |
| Skip cognitive state validation | Faster self-improvement | Memory corruption, unpredictable behavior | Never |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| USB Serial | Opening port without waiting for OS to settle | Add 100ms delay after port enumeration |
| STM32 DFU | Not detecting bootloader mode | Check BOOT0 pin state, handle ROM bootloader |
| Raspberry Pi GPIO | Using BCM pin numbers without mapping | Validate pin mapping for Pi model |
| Docker Runtime | Running with `--privileged` for convenience | Use minimal capabilities, explicit syscall allowlist |
| Workspace Scoping | Trusting relative paths | Always canonicalize before check |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| USB polling in tight loop | High CPU, battery drain | Use event-driven libudev | Always - inefficient |
| Goal engine without limits | Memory exhaustion | Budget enforcement per goal | At ~1000 goals |
| Cognitive module full reload | Slow startup | Incremental state loading | At >10MB cognitive state |
| Serial port without read timeout | Hang on disconnected device | Set read_timeout_ms | Device cable removal |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Trusting LLM-generated code without sandbox | Remote code execution | All code runs in sandboxed container |
| No rate limiting on webhook endpoints | DoS, brute force | Implement per-IP rate limits |
| Storing secrets in plaintext config | Credential theft | Use encrypted secrets with key file |
| Pairing code never expires | Unauthorized access | Time-bounded codes, max attempts |
| No audit logging of agent actions | No incident traceability | Immutable action audit log |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Silent USB device enumeration failure | User doesn't know hardware not detected | Show clear "no devices found" with troubleshooting |
| Cryptic ST-LINK errors | User can't recover bricked board | Translate ST-LINK errors to actionable messages |
| GPIO works for dev, fails for user | "It works on my machine" | Auto-install udev rules during setup |
| Goal engine hangs without feedback | User thinks system is frozen | Show goal progress, timeout with user notification |

---

## "Looks Done But Isn't" Checklist

- [ ] **USB Discovery:** Handles device hot-plug/unplug correctly — test by rapidly plugging/unplugging
- [ ] **STM32 Flashing:** Verifies firmware checksum before activating — test by power-cycling mid-flash
- [ ] **GPIO Access:** Works for non-root users on Raspberry Pi OS — test on fresh install
- [ ] **Goal Engine:** Has hard limits on goal depth and resource usage — test with recursive goal
- [ ] **Cognitive Modules:** Validates all state changes — test with malformed memory write
- [ ] **Sandboxing:** Survives escape attempt primitives — test with known escape techniques
- [ ] **Pairing:** Rate-limits and expires codes — test with brute-force script
- [ ] **Workspace Scoping:** Blocks symlink escape — test with `ln -s /etc /workspace/etc`

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Bricked STM32 | HIGH | Manual ST-LINK erase, hardware recovery |
| GPIO permission locked out | MEDIUM | Re-run udev setup, reboot |
| Goal engine infinite loop | LOW | Kill agent process, reset goal state |
| Cognitive state corruption | HIGH | Restore from last known-good snapshot |
| Sandbox escape | CRITICAL | Redeploy from clean state, audit access |
| Workspace bypass exploited | CRITICAL | Audit what was accessed, rotate secrets |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| USB Device Enumeration Race | Hardware Integration - USB | Rapid plug/unplug test |
| STM32/Nucleo Flash Corruption | Hardware Integration - Flashing | Power interrupt mid-flash test |
| GPIO Permission Hell | Hardware Integration - GPIO | Fresh Pi user test |
| Goal Engine Infinite Loop | AGI - Goal Engine | Recursive goal stress test |
| Cognitive Module Corruption | AGI - Cognitive Modules | Malformed state injection |
| Sandbox Escape | Security - Sandboxing | Escape primitive test suite |
| Pairing Timing Attack | Security - Pairing | Brute-force attempt logging |
| Workspace Symlink Escape | Security - Workspace Scoping | Symlink escape attempt |

---

## Sources

- [Common AI Agent Development Mistakes - WildNet](https://www.wildnetedge.com/blogs/ai-agent-development-mistakes)
- [AI Agent Mistakes That Cost Developers - Arms of Old](https://armsofold.co.uk/ai-agent-mistakes)
- [ST Community - Nucleo Flash Issues](https://community.st.com/)
- [Raspberry Pi GPIO Best Practices](https://pip.raspberrypi.com/categories/685-whitepapers-app-notes/documents/RP-006553-WP/)
- [Docker Sandboxes Aren't Enough - Arcade](https://blog.arcade.dev/docker-sandboxes-arent-enough-for-agent-safety)
- [Complete Guide to Sandboxing Autonomous Agents](https://www.ikangai.com/the-complete-guide-to-sandboxing-autonomous-agents/)
- [When AI Agents Escape the Sandbox - Monday Momentum](https://www.mondaymomentum.io/p/when-ai-agents-escape-the-sandbox)
- [Probe-rs STM32 Flashing Issues](https://users.rust-lang.org/t/cant-flash-the-program-into-stm32nucleo-l476rg/)

---

*Pitfalls research for: Housaky embedded AI agent infrastructure*
*Researched: 2026-02-24*
