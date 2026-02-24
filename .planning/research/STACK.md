# Stack Research

**Domain:** Rust Ecosystem for Hardware Integration, AGI Agents, and Security Sandboxing
**Researched:** 2026-02-24
**Confidence:** HIGH

## Recommended Stack

### Hardware Integration

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|------------------|
| **nusb** | 0.2.1 | Cross-platform USB device discovery and communication | Pure Rust, no libusb dependency, async-first, supports Linux/macOS/Windows. Active development (19 releases). Used by cross_usb for WASM support. |
| **serialport** | 4.3.0 | Serial port enumeration and communication | Mature (24 releases), supports USB-serial adapters, async via tokio-serial. Act |
| **rively maintained.ppal** | 0.22.1 | Raspberry Pi GPIO, I2C, PWM, SPI, UART | Most complete Pi HAL (36 releases), well-documented, 14K monthly downloads. |
| **gpio-cdev** | 0.6.0 | Linux GPIO character device ABI | Official Linux GPIO interface (replaces deprecated sysfs), used by rppal internally. |
| **probe-rs** | 0.31.0+ | STM32/Nucleo debugging and flashing | Industry standard for ARM/RISC-V flashing in Rust. VSCode integration, RTT support. Used by cargo-flash, cargo-embed. |
| **embedded-hal** | 1.0.0 | Hardware abstraction layer trait definitions | Rust embedded standard — provides traits (I2c, Spi, Gpio) that all embedded crates implement. Stable since Jan 2024. |
| **embassy-usb** | 0.5.1 | USB device/host stack for embedded | Async USB stack from Embassy project, integrates with embedded-hal 1.0. |

### AGI Agent Frameworks

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|------------------|
| **neuron** | Latest | Composable building blocks for AI agents (traits: Provider, Tool, ContextStrategy) | "Serde, not serde_json" philosophy — provides traits and composable blocks, not opinionated framework. Actively maintained (docs at secbear.github.io/neuron). |
| **AutoAgents** | 0.3.4 | Production-grade multi-agent framework | 354 stars, actively maintained (v0.3.3 released Feb 2026), modular design, MCP support. |
| **Anda** | 0.8.0 | AI agent framework with ICP and TEE support | 403 stars, unique blockchain/privacy features, WASM component support. |
| **Amico V2** | Latest | Embedded AI agent runtime | Specifically designed for embedded devices, platform-agnostic runtime, 38 stars. |
| **Cargo-AI** | Latest | JSON-declared lightweight agents | Compile JSON configs to Rust binaries, ultra-lightweight for embedded. |
| **asterbot** | Latest | WASM-component-based modular agent | Hyper-modular (Feb 2026), every capability is swappable WASM, from public registry. |

### Security Sandboxing

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|------------------|
| **landlock** | 0.4.4 | Linux kernel filesystem sandboxing | Rust-native Landlock bindings. No root required, works with unprivileged users. Kernel-native (stable since Linux 5.13). |
| **tabox** | 1.3.6 | Seccomp-based system call filtering | MPL-2.0 licensed, supports seccomp-bpf, resource limits (CPU, memory, wall time). Cross-platform (Linux, macOS). |
| **silicube** | Latest | IOI Isolate-based sandboxing | Production-grade, cgroup v2 support, resource limit enforcement, interactive execution support. |
| **LiteBox** | Latest | Security-focused sandboxing library | Microsoft research (Feb 2026), "north/south" interface design for minimal attack surface. Kernel and userspace. |
| **microsandbox** | 0.1.2 | Async sandbox execution | Async Rust API, designed for running untrusted code, supports multiple languages. |
| **bwrap** (bubblewrap) | System | Linux namespace sandboxing | Not a Rust crate — use via subprocess. Most mature Linux sandbox, used by Flatpak, GNOME Sandbox. |
| **firejail** | System | Linux privilege separation | Not Rust — use via subprocess. Profiles for many applications, simpler than bubblewrap for desktop. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| **cargo-embed** | Embedded debugging with RTT | Part of probe-rs ecosystem, replaces cargo-binutils for STM32. |
| **cargo-flash** | Flash firmware to devices | Standalone flashing, works without full debugging. |
| **defmt** | Efficient logging for embedded | Used by probe-rs, Embassy. Format similar to println but much smaller binary. |
| **probe-run** | Run embedded programs with RTT | Part of probe-rs toolchain, handles reset and logging. |

## Alternatives Considered

| Category | Recommended | Alternative | When to Use Alternative |
|----------|-------------|-------------|------------------------|
| USB | nusb | rusb | Need libusb compatibility or older platform support. nusb is pure Rust with async support; rusb requires libusb FFI. |
| USB | nusb | libusb-sys | When you already have libusb infrastructure. Prefer nusb for new projects. |
| Serial | serialport | tokio-serial | Need async serial I/O; serialport provides blocking. |
| GPIO | rppal | sysfs_gpio | Legacy only (deprecated). rppal uses gpio-cdev (Linux standard). |
| Flashing | probe-rs | openocd + GDB | Need wider chip support beyond ARM/RISC-V, or custom JTAG adapters. probe-rs is Rust-native with better DX. |
| AGI | neuron | autogpt (kevin-rs) | autogpt is WIP; neuron provides stable composable blocks. |
| AGI | AutoAgents | LangChain (Python) | Need Python ecosystem integration; AutoAgents is Rust-native for edge deployment. |
| Sandbox | landlock | seccomp-bpf (libseccomp) | Need finer-grained syscall control; landlock is simpler for filesystem-only restrictions. |
| Sandbox | landlock | Docker | Need full containerization with networking; landlock is for lightweight process sandboxing. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **sysfs_gpio** | Linux deprecated sysfs interface (removed in kernel 6.x) | gpio-cdev or rppal |
| **libusb** (C) | FFI overhead, not Rust-native | nusb for pure Rust, rusb if libusb required |
| **rusb** (with libusb) | Requires C library dependency, less async-friendly | nusb for modern async-first approach |
| **autogpt (kevin-rs)** | WIP, not production-ready | AutoAgents or neuron for stable AGI in Rust |
| **Python agent frameworks** (LangChain, CrewAI) | Runtime overhead, not Rust-native | AutoAgents, Anda, Amico for Rust-native |
| **Docker-only sandboxing** | Heavy for simple process isolation | landlock for lightweight filesystem sandboxing |
| **Bubblewrap (via CLI)** | Not integrated, requires subprocess management | tabox or landlock for Rust-native sandboxing |

## Stack Patterns by Variant

**If targeting Raspberry Pi GPIO:**
- Use `rppal` for GPIO/I2C/SPI/PWM
- Use `embedded-hal` traits for portable embedded code
- Use `probe-rs` for flashing Pi RP2040 boards

**If targeting USB device discovery (desktop/embedded):**
- Use `nusb` for pure Rust cross-platform USB
- Use `serialport` for USB-serial adapters
- Use `usb-resolver` for role-based device matching

**If targeting STM32/Nucleo flashing:**
- Use `probe-rs` (cargo-flash, cargo-embed)
- Use `embassy-usb` for embedded USB CDC
- Use `embedded-hal` 1.0.0 + `stm32-hal2` for firmware

**If building AGI agents:**
- Use `neuron` for composable traits (Provider, Tool, ContextStrategy)
- Use `AutoAgents` for production multi-agent systems
- Use `asterbot` if WASM component modularity is required

**If implementing sandboxing:**
- Use `landlock` for filesystem-only restrictions (no root needed)
- Use `tabox` for system call filtering + resource limits
- Use Docker subprocess for full containerization (existing housaky pattern)

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| nusb 0.2.x | tokio (async), no runtime required | Async-first but works with blocking too |
| rppal 0.22.x | embedded-hal 0.2.x, Linux only | Pi-specific, not no-std |
| probe-rs 0.31.x | DAP-compatible debuggers, VSCode | Active development, check releases |
| embedded-hal 1.0.0 | embassy-* crates, HAL implementations | Stable since Jan 2024 |
| landlock 0.4.x | Linux 5.13+ kernels | Kernel feature, not crate version dependent |
| AutoAgents 0.3.x | tokio, reqwest | Check features for MCP, filesystem, search |

## Sources

- **Context7/nusb** — Cross-platform USB library documentation, version 0.2.1
- **Context7/rppal** — Raspberry Pi HAL, version 0.22.1
- **Context7/probe-rs** — STM32/RISC-V flashing, version 0.31.0
- **Context7/embedded-hal** — Hardware abstraction traits, version 1.0.0
- **Context7/landlock** — Linux filesystem sandboxing, version 0.4.4
- **Context7/serialport** — Cross-platform serial, version 4.3.0
- **WebSearch** — AutoAgents (354 stars, Feb 2026), Anda (403 stars), Amico V2 (embedded-focused)
- **WebSearch** — neuron docs at secbear.github.io/neuron (composable agent blocks)
- **WebSearch** — LiteBox (Microsoft research, Feb 2026), silicube (IOI Isolate)
- **crates.io** — Version verification, download stats, release dates

---

*Stack research for: Housaky enhancements (hardware integration, AGI, security)*
*Researched: 2026-02-24*
