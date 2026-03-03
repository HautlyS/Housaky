# Housaky Architecture

This document provides a high-level overview of the Housaky project's architecture.

## 1. High-Level Overview

The Housaky project is a sophisticated AI assistant built with a focus on performance, modularity, and cross-platform compatibility. The architecture can be broken down into the following key components:

- **Core Backend:** A powerful, asynchronous backend written in Rust that contains the main agent logic, tool integrations, and communication gateways.
- **Desktop Application:** A cross-platform desktop application built with Tauri, using web technologies (Vue.js, Vite, Tailwind CSS) for the user interface.
- **Landing Page:** A simple, static landing page, also built with Vue.js and Vite.
- **Docker-based Deployment:** A Docker Compose setup for easy deployment and orchestration of the Housaky service.

## 2. Core Backend (Rust)

The heart of the project is a Rust application located in the `src` directory. It is designed to be a high-performance, asynchronous application using the Tokio runtime, and structured around a clear separation of concerns through its module system.

### Modular Design and Key Modules

The `src/lib.rs` file defines the public interface and modular structure of the Housaky backend. Each module encapsulates specific functionalities, promoting maintainability, testability, and scalability. Key modules include:

- **`agent`:** Contains the core AI agent logic, including the main execution loop, prompt management, and tool dispatching.
- **`commands`:** Defines the structure of the command-line interface using `clap`.
- **`config` & `config_editor`:** Manages application configuration and provides an interactive configuration editor.
- **`channels`:** Handles integration with various communication platforms (e.g., Telegram, Discord, Slack).
- **`cron`:** Manages scheduled tasks and automated processes.
- **`daemon` & `service`:** Manages the long-running background service and OS-level service integration (e.g., `systemd`).
- **`dashboard`:** Supports the web-based dashboard and desktop application frontend.
- **`doctor`:** Provides diagnostic and health check functionalities.
- **`gateway`:** Implements the HTTP/WebSocket gateway for external API access and webhooks.
- **`hardware` & `peripherals`:** Manages hardware discovery, introspection, and interaction with various peripherals (e.g., USB, serial, GPIO).
- **`housaky` (AGI Core):** This module is the core of Housaky's AGI implementation, encompassing goal management, self-modification, collective intelligence, and advanced cognitive functions.
- **`identity`:** Manages the agent's identity and persona.
- **`integrations`:** Handles third-party integrations.
- **`keys_manager`:** Securely manages API keys and provider credentials.
- **`memory`:** Implements the agent's persistent memory system.
- **`migration`:** Provides functionalities for migrating data from other agent runtimes.
- **`observability`:** Manages logging, metrics, and tracing for monitoring and debugging.
- **`onboard`:** Handles the initial setup and onboarding process.
- **`providers`:** Integrates with various Large Language Model (LLM) providers.
- **`quantum`:** Manages integration with quantum computing services and simulators (e.g., Amazon Braket).
- **`rag`:** Implements Retrieval-Augmented Generation for specialized knowledge retrieval.
- **`runtime`:** Provides an abstraction layer for different execution environments.
- **`security`:** Enforces security policies and autonomy controls.
- **`skillforge` & `skills`:** Manages the creation, installation, and execution of custom skills.
- **`tools`:** Defines the core set of tools available to the agent.
- **`tui`:** Implements Terminal User Interface components.
- **`tunnel`:** Provides secure tunneling capabilities.
- **`util`:** Contains common utility functions.

### Configuration Management

Housaky features a highly granular and comprehensive configuration system, primarily defined by the `src/config/schema.rs` file. This allows for fine-grained control over almost every aspect of the application's behavior.

Key aspects of the configuration system include:

-   **Centralized Configuration (`Config` struct):** A single `Config` struct consolidates settings from various domains, making it the central point for managing Housaky's behavior.
-   **Modular Configuration Structs:** Configuration is organized into numerous specialized structs (e.g., `AgentConfig`, `ChannelsConfig`, `GatewayConfig`, `MemoryConfig`, `QuantumConfig`, `AutonomyConfig`, `SelfModificationConfig`, `SelfReplicationConfig`, `CollectiveSchemaConfig`). Each of these corresponds to a specific functional area of the application, promoting clear separation of concerns and ease of management.
-   **Default Values:** Most configuration fields have sensible default values, simplifying initial setup and ensuring the application can run out-of-the-box.
-   **Serialization and Deserialization:** Configuration is handled using `serde` and `toml`, allowing for easy persistence to and loading from `config.toml` files.
-   **Environment Variable Overrides:** Critical configuration parameters (e.g., `api_key`, `default_provider`, `default_model`, `gateway.port`, `gateway.host`, `gateway.allow_public_bind`, `default_temperature`, `workspace_dir`) can be overridden by environment variables (e.g., `HOUSAKY_API_KEY`, `HOUSAKY_PROVIDER`), providing flexibility for deployment in various environments (e.g., Docker containers).
-   **API Key Management:** The `api_key` field, along with the `keys_manager` module, handles the secure storage, rotation, and usage of API keys across different LLM providers, integrating with the `reliability` and `secrets` configurations.
-   **Dynamic Configuration (Watcher):** The `src/config/watcher.rs` module suggests capabilities for live monitoring of configuration changes and dynamic updates.
-   **Interactive Editor:** The `config_editor` module provides a TUI-based interactive editor for convenient modification of settings.

This robust configuration system ensures that Housaky can be adapted to a wide range of use cases and operational environments, from local development to autonomous deployment in production.

### Large Language Model (LLM) Integration

Housaky's architecture includes a sophisticated and modular system for integrating with various Large Language Models. This design ensures flexibility, reliability, and security in LLM interactions.

Key components of the LLM integration include:

-   **Modular Provider Design:** The `src/providers` module is structured with separate implementations for each major LLM service (e.g., Anthropic, OpenAI, OpenRouter, Ollama, Gemini). This modularity simplifies the addition of new providers and streamlines maintenance.
-   **`Provider` Trait:** A core `Provider` trait defines a unified interface for all LLM interactions, abstracting away provider-specific details. This allows Housaky's agent logic to communicate with diverse LLMs through a consistent API, typically including methods for `chat` and `chat_with_system`.
-   **Standardized Data Formats:** Communication with LLMs uses standardized data structures like `ChatMessage`, `ChatRequest`, `ChatResponse`, `ConversationMessage`, `ToolCall`, and `ToolResultMessage`, facilitating consistent tool-use capabilities.
-   **Robust API Key Resolution:** API keys are resolved through a priority system: explicit parameters, provider-specific environment variables (e.g., `ANTHROPIC_API_KEY`), and generic environment variables (`HOUSAKY_API_KEY`). This provides flexibility and secure handling of credentials.
-   **Dynamic Provider Creation:** A factory function (`create_provider`) dynamically instantiates the correct `Provider` implementation at runtime, supporting:
    -   **Primary Providers:** Direct integrations for commonly used LLMs.
    -   **OpenAI-Compatible Providers:** A generic adapter (`OpenAiCompatibleProvider`) for services adhering to the OpenAI API standard, enabling broad compatibility.
    -   **Custom Endpoints:** Allows users to configure and use custom OpenAI-compatible or Anthropic-compatible API endpoints.
-   **Resilient Provider Wrappers:** The `create_resilient_provider` function enhances reliability by wrapping individual provider instances with retry mechanisms and fallback logic, improving tolerance to API failures and rate limits. This includes configurable retry attempts, backoff strategies, and a chain of fallback providers.
-   **Sophisticated Request Routing:** The `create_routed_provider` function implements advanced routing based on "hints" or "subjects." This directs specific types of LLM requests (e.g., "reasoning," "code") to optimal providers and models, enabling fine-grained control over cost, performance, and capabilities. It can also apply provider-specific concurrency limits.
-   **API Error Sanitization:** Sensitive information, such as API keys, is automatically redacted from error messages by `scrub_secret_patterns` and `sanitize_api_error` before they are logged or displayed. This is a critical security measure to prevent accidental credential exposure.
-   **Local Model Integration:** Support for local LLM services like Ollama (which typically do not require API keys) is seamlessly integrated, offering flexibility for privacy-sensitive or offline deployments.

### Memory and Knowledge Management

Housaky's persistent memory and knowledge management system is crucial for its ability to learn, maintain context, and operate autonomously across sessions. It features a modular design with advanced retrieval and maintenance capabilities.

-   **Modular Backends:** The `src/memory` module supports various memory backends, providing flexibility based on persistence, performance, and accessibility needs:
    -   **`sqlite`:** A robust, persistent SQL database (using `rusqlite`) for structured storage and efficient querying. This is the default and provides strong capabilities for large-scale memory.
    -   **`lucid`:** A specialized hybrid memory backend, often combining in-memory speed with persistent storage, designed for high-performance retrieval.
    -   **`markdown`:** A human-readable, file-based memory option for simpler, easily inspectable knowledge storage.
    -   **`none`:** A no-operation backend, used when memory persistence is not required.
-   **`Memory` Trait:** All memory backends implement the `Memory` trait, ensuring a unified interface for `store()`, `recall()`, `get()`, `list()`, `forget()`, and `count()` operations.
-   **Structured Memory Entries:** Information is stored as `MemoryEntry` structs, categorized by `MemoryCategory` (e.g., `Conversation`, `Daily`, `Permanent`, `Semantic`), enabling organized storage and context-specific retrieval.
-   **Embedding-based Semantic Search:** For `sqlite` and `lucid` backends, retrieval leverages embeddings for advanced semantic search. This allows Housaky to recall information based on conceptual meaning rather than just keyword matching, utilizing an `EmbeddingProvider` and `embedding_model` (e.g., OpenAI's `text-embedding-3-small`) with configurable `vector_weight` and `keyword_weight` for hybrid search.
-   **Context Chunking:** Large documents or conversation histories are intelligently chunked into smaller, manageable pieces to optimize embedding generation, storage, and retrieval efficiency, especially within limited LLM context windows.
-   **LLM Response Caching:** An optional `ResponseCache` stores LLM responses, preventing redundant API calls for identical prompts. This saves costs and improves responsiveness, with configurable TTL and maximum entries.
-   **Memory Hygiene and Snapshots:**
    -   **Hygiene:** The `hygiene` module provides scheduled archiving and purging of old memory entries based on configurable retention policies, managing the memory footprint and relevance.
    -   **Snapshots:** Core memories can be periodically exported to a human-readable `MEMORY_SNAPSHOT.md` file, serving as a "soul backup." The system also supports `auto_hydrate`, which can restore the primary memory database from this snapshot during a cold boot if the database is missing or corrupted.

This comprehensive memory system allows Housaky to maintain long-term context, learn from past interactions, and efficiently manage vast amounts of information, directly underpinning its autonomous and persistent nature.

### Communication Channels and Integrations

Housaky is designed to be a multi-modal AI assistant, capable of interacting with users across various communication platforms. The `src/channels` module provides a flexible and extensible framework for these integrations.

-   **Modular Channel Backends:** Separate modules exist for each supported communication platform, including:
    -   **`cli`:** For command-line interface interactions.
    -   **`telegram`:** Telegram bot integration.
    -   **`discord`:** Discord bot integration.
    -   **`slack`:** Slack app integration.
    -   **`webhook`:** Generic webhook support for custom integrations.
    -   **`imessage`:** iMessage integration.
    -   **`matrix`:** Matrix chat integration.
    -   **`whatsapp`:** WhatsApp Business API integration.
    -   **`email_channel`:** Email integration.
    -   **`irc`:** IRC client integration.
    -   **`lark`:** Lark/Feishu integration.
    -   **`dingtalk`:** DingTalk integration.
-   **`Channel` Trait:** The core `Channel` trait defines a unified interface for all communication channels, abstracting platform-specific details. Key methods include `name()`, `send()` (to a recipient), `listen()` (for incoming messages), `start_typing()`, `stop_typing()`, and `health_check()`.
-   **Centralized Message Bus:** All incoming messages from various channels are collected and forwarded to a single `tokio::sync::mpsc::Sender` for consistent processing.
-   **Message Processing Workflow:**
    -   **`ChannelRuntimeContext`:** Encapsulates the agent's core components (LLM provider, memory, tools, observer, system prompt, AGI processor) and configuration for message processing.
    -   **`process_channel_message()`:** Handles individual incoming messages. It supports cross-agent coordination commands (e.g., `/running`, `/logs`), integrates with the AGI core via `AGIChannelProcessor`, manages per-sender conversation history, and enriches messages with memory context before LLM interaction.
-   **Resilience and Concurrency:**
    -   **Supervised Listeners:** Each channel's `listen()` method runs in a supervised task (`spawn_supervised_listener`), with automatic restarts and exponential backoff on unexpected exits or errors, ensuring high availability of channels.
    -   **Message Dispatch Loop (`run_message_dispatch_loop`):** Manages concurrent processing of incoming messages using a `tokio::sync::Semaphore`, preventing bottlenecks and controlling resource usage.
-   **Dynamic System Prompt Generation (`build_system_prompt`):** For each channel conversation, a tailored system prompt is dynamically assembled. This prompt incorporates:
    -   Detailed tool descriptions and usage protocols.
    -   Hardware access instructions (when applicable).
    -   Crucial safety guidelines.
    -   A compact list of available skills (loaded on-demand).
    -   Workspace and project context from bootstrap files (`AGENTS.md`, `SOUL.md`, `IDENTITY.md`, etc.), with support for AIEOS identities.
    -   Current date, time, and runtime metadata.
-   **AGI Channel Processor (`agi_processor.rs`):** When Housaky's AGI capabilities are enabled, this specialized processor acts as the bridge, allowing incoming channel messages to trigger complex AGI reasoning, goal management, and self-improvement cycles.
-   **Health Checks and CLI Management:** The `doctor_channels` function provides diagnostic capabilities for all configured channels, while `handle_command` offers CLI utilities for listing and managing channel integrations.

This robust and flexible channel integration architecture allows Housaky to serve as a versatile AI assistant, interacting with users across diverse platforms while maintaining its advanced cognitive and autonomous capabilities.

### Communication Channels and Integrations

Housaky is designed to be a multi-modal AI assistant, capable of interacting with users across various communication platforms. The `src/channels` module provides a flexible and extensible framework for these integrations.

-   **Modular Channel Backends:** Separate modules exist for each supported communication platform, including:
    -   **`cli`:** For command-line interface interactions.
    -   **`telegram`:** Telegram bot integration.
    -   **`discord`:** Discord bot integration.
    -   **`slack`:** Slack app integration.
    -   **`webhook`:** Generic webhook support for custom integrations.
    -   **`imessage`:** iMessage integration.
    -   **`matrix`:** Matrix chat integration.
    -   **`whatsapp`:** WhatsApp Business API integration.
    -   **`email_channel`:** Email integration.
    -   **`irc`:** IRC client integration.
    -   **`lark`:** Lark/Feishu integration.
    -   **`dingtalk`:** DingTalk integration.
-   **`Channel` Trait:** The core `Channel` trait defines a unified interface for all communication channels, abstracting platform-specific details. Key methods include `name()`, `send()` (to a recipient), `listen()` (for incoming messages), `start_typing()`, `stop_typing()`, and `health_check()`.
-   **Centralized Message Bus:** All incoming messages from various channels are collected and forwarded to a single `tokio::sync::mpsc::Sender` for consistent processing.
-   **Message Processing Workflow:**
    -   **`ChannelRuntimeContext`:** Encapsulates the agent's core components (LLM provider, memory, tools, observer, system prompt, AGI processor) and configuration for message processing.
    -   **`process_channel_message()`:** Handles individual incoming messages. It supports cross-agent coordination commands (e.g., `/running`, `/logs`), integrates with the AGI core via `AGIChannelProcessor`, manages per-sender conversation history, and enriches messages with memory context before LLM interaction.
-   **Resilience and Concurrency:**
    -   **Supervised Listeners:** Each channel's `listen()` method runs in a supervised task (`spawn_supervised_listener`), with automatic restarts and exponential backoff on unexpected exits or errors, ensuring high availability of channels.
    -   **Message Dispatch Loop (`run_message_dispatch_loop`):** Manages concurrent processing of incoming messages using a `tokio::sync::Semaphore`, preventing bottlenecks and controlling resource usage.
-   **Dynamic System Prompt Generation (`build_system_prompt`):** For each channel conversation, a tailored system prompt is dynamically assembled. This prompt incorporates:
    -   Detailed tool descriptions and usage protocols.
    -   Hardware access instructions (when applicable).
    -   Crucial safety guidelines.
    -   A compact list of available skills (loaded on-demand).
    -   Workspace and project context from bootstrap files (`AGENTS.md`, `SOUL.md`, `IDENTITY.md`, etc.), with support for AIEOS identities.
    -   Current date, time, and runtime metadata.
-   **AGI Channel Processor (`agi_processor.rs`):** When Housaky's AGI capabilities are enabled, this specialized processor acts as the bridge, allowing incoming channel messages to trigger complex AGI reasoning, goal management, and self-improvement cycles.
-   **Health Checks and CLI Management:** The `doctor_channels` function provides diagnostic capabilities for all configured channels, while `handle_command` offers CLI utilities for listing and managing channel integrations.

This robust and flexible channel integration architecture allows Housaky to serve as a versatile AI assistant, interacting with users across diverse platforms while maintaining its advanced cognitive and autonomous capabilities.

### API Gateway (HTTP/WebSocket)

The `src/gateway` module provides an `axum`-based HTTP/WebSocket server, offering external access points for webhook integrations, client pairing, and real-time event streaming. It's designed with a strong focus on security, reliability, and ease of integration.

-   **HTTP/1.1 Compliance:** Leverages the `axum` framework for robust HTTP/1.1 parsing, content-length validation, configurable body size limits (`MAX_BODY_SIZE`), and request timeouts (`REQUEST_TIMEOUT_SECS`) to prevent abuse.
-   **Key Endpoints:**
    -   `/health`: A public endpoint for checking the gateway's operational status.
    -   `/pair`: Facilitates a secure one-time pairing process, exchanging a shared secret for a `Bearer` token required for authenticated access to other endpoints.
    -   `/webhook`: A generic webhook receiver for processing incoming data from various services. It expects a JSON payload with a `message` field.
    -   `/events`: A Server-Sent Events (SSE) endpoint that streams real-time `GatewayEvent`s (e.g., `Paired`, `WebhookReceived`, `WebhookResponded`) to authenticated clients, providing live feedback on gateway activities.
    -   `/whatsapp`: Dedicated endpoints for Meta (WhatsApp) webhook verification (`GET`) and processing incoming messages (`POST`), with built-in security mechanisms.
-   **Comprehensive Security Measures:**
    -   **Pairing Mechanism (`PairingGuard`):** Enforces a robust client authentication flow, requiring a unique, one-time pairing code to issue bearer tokens, which must then be presented for authenticated API calls.
    -   **Public Bind Refusal:** By default, the gateway actively refuses to bind to public IP addresses (`0.0.0.0`) to prevent accidental exposure, unless explicitly configured with a tunnel or `allow_public_bind` (not recommended for production without further security controls).
    -   **Webhook Secret (`X-Webhook-Secret`):** Supports an optional, additional layer of authentication for the `/webhook` endpoint, requiring a pre-shared secret in the request headers.
    -   **WhatsApp Signature Verification (`X-Hub-Signature-256`):** For WhatsApp webhooks, it automatically performs HMAC-SHA256 signature verification, ensuring that incoming messages are legitimate and untampered.
    -   **Rate Limiting (`GatewayRateLimiter`):** Implements a sliding window rate-limiting mechanism for `/pair` and `/webhook` endpoints based on client IP, protecting against abuse and denial-of-service attacks. Limits are configurable per minute.
    -   **Idempotency Store (`IdempotencyStore`):** Utilizes `X-Idempotency-Key` headers for webhook requests to prevent duplicate processing of events, ensuring transactional integrity with a configurable TTL.
-   **Tunnel Integration:** Seamlessly integrates with external tunneling solutions (e.g., Cloudflare, ngrok, Tailscale) via the `crate::tunnel` module. This enables secure public exposure of the gateway's services without requiring direct public IP binding or complex firewall configurations.
-   **Event Broadcasting:** A `tokio::sync::broadcast` channel disseminates `GatewayEvent`s to connected clients, facilitating real-time monitoring and integration with other system components.
-   **Memory Integration:** Automatically stores incoming webhook messages to Housaky's memory system if `auto_save` is enabled, using dynamically generated keys for persistence.
-   **LLM Interaction:** The gateway's webhook handlers directly interface with the configured LLM `provider` to process incoming user messages or data and generate appropriate AI responses.

This robust API gateway provides secure, reliable, and scalable external interaction with the Housaky agent, allowing it to integrate with various web services and client applications.

### API Gateway (HTTP/WebSocket)

The `src/gateway` module provides an `axum`-based HTTP/WebSocket server, offering external access points for webhook integrations, client pairing, and real-time event streaming. It's designed with a strong focus on security, reliability, and ease of integration.

-   **HTTP/1.1 Compliance:** Leverages the `axum` framework for robust HTTP/1.1 parsing, content-length validation, configurable body size limits (`MAX_BODY_SIZE`), and request timeouts (`REQUEST_TIMEOUT_SECS`) to prevent abuse.
-   **Key Endpoints:**
    -   `/health`: A public endpoint for checking the gateway's operational status.
    -   `/pair`: Facilitates a secure one-time pairing process, exchanging a shared secret for a `Bearer` token required for authenticated access to other endpoints.
    -   `/webhook`: A generic webhook receiver for processing incoming data from various services. It expects a JSON payload with a `message` field.
    -   `/events`: A Server-Sent Events (SSE) endpoint that streams real-time `GatewayEvent`s (e.g., `Paired`, `WebhookReceived`, `WebhookResponded`) to authenticated clients, providing live feedback on gateway activities.
    -   `/whatsapp`: Dedicated endpoints for Meta (WhatsApp) webhook verification (`GET`) and processing incoming messages (`POST`), with built-in security mechanisms.
-   **Comprehensive Security Measures:**
    -   **Pairing Mechanism (`PairingGuard`):** Enforces a robust client authentication flow, requiring a unique, one-time pairing code to issue bearer tokens, which must then be presented for authenticated API calls.
    -   **Public Bind Refusal:** By default, the gateway actively refuses to bind to public IP addresses (`0.0.0.0`) to prevent accidental exposure, unless explicitly configured with a tunnel or `allow_public_bind` (not recommended for production without further security controls).
    -   **Webhook Secret (`X-Webhook-Secret`):** Supports an optional, additional layer of authentication for the `/webhook` endpoint, requiring a pre-shared secret in the request headers.
    -   **WhatsApp Signature Verification (`X-Hub-Signature-256`):** For WhatsApp webhooks, it automatically performs HMAC-SHA256 signature verification, ensuring that incoming messages are legitimate and untampered.
    -   **Rate Limiting (`GatewayRateLimiter`):** Implements a sliding window rate-limiting mechanism for `/pair` and `/webhook` endpoints based on client IP, protecting against abuse and denial-of-service attacks. Limits are configurable per minute.
    -   **Idempotency Store (`IdempotencyStore`):** Utilizes `X-Idempotency-Key` headers for webhook requests to prevent duplicate processing of events, ensuring transactional integrity with a configurable TTL.
-   **Tunnel Integration:** Seamlessly integrates with external tunneling solutions (e.g., Cloudflare, ngrok, Tailscale) via the `crate::tunnel` module. This enables secure public exposure of the gateway's services without requiring direct public IP binding or complex firewall configurations.
-   **Event Broadcasting:** A `tokio::sync::broadcast` channel disseminates `GatewayEvent`s to connected clients, facilitating real-time monitoring and integration with other system components.
-   **Memory Integration:** Automatically stores incoming webhook messages to Housaky's memory system if `auto_save` is enabled, using dynamically generated keys for persistence.
-   **LLM Interaction:** The gateway's webhook handlers directly interface with the configured LLM `provider` to process incoming user messages or data and generate appropriate AI responses.

This robust API gateway provides secure, reliable, and scalable external interaction with the Housaky agent, allowing it to integrate with various web services and client applications.

### Daemon and Service Management

Housaky operates as a long-running autonomous agent, and its continuous operation is managed through a robust daemon system integrated with native OS service management. The `src/daemon` and `src/service` modules orchestrate this functionality.

-   **Daemon Core (`src/daemon`):**
    -   **Component Orchestration:** The daemon acts as a supervisor for several core Housaky components, launching and managing them as resilient background tasks. These components include:
        -   **`gateway`:** The HTTP/WebSocket API for external communication.
        -   **`channels`:** All configured communication platforms (Telegram, Discord, etc.) ensuring multi-modal interaction.
        -   **`heartbeat`:** A general mechanism for periodic background tasks.
        -   **`housaky` (AGI Core Heartbeat):** A dedicated worker specifically for the core AGI's continuous self-improvement, learning loops, and autonomous functions.
        -   **`scheduler`:** Manages and executes cron-like scheduled tasks, allowing for automated actions.
    -   **Resilience (`spawn_component_supervisor`):** A critical feature that ensures high availability. If any supervised component fails or exits unexpectedly, it is automatically restarted with an exponential backoff strategy, preventing service interruptions.
    -   **State Persistence (`spawn_state_writer`):** A background task that periodically writes the daemon's current health and component status (e.g., operational status, errors, restart counts) to a `daemon_state.json` file. This provides persistent monitoring data and aids in diagnostics and recovery.
    -   **Graceful Shutdown:** The daemon is designed to respond to standard OS signals (`SIGINT` for Ctrl+C, `SIGTERM` for system service termination). Upon receiving such a signal, it gracefully attempts to abort all running component tasks, ensuring a clean shutdown and preventing data corruption.
-   **OS Service Integration (`src/service`):**
    -   **Platform-Specific Management:** This module provides utilities to integrate the Housaky daemon seamlessly with native operating system service managers, supporting both `systemd --user` on Linux and `launchd` on macOS.
    -   **Service Unit Generation:** It dynamically generates appropriate service unit files (`.service` for `systemd`, `.plist` for `launchd`). These configuration files ensure that the Housaky daemon:
        -   Starts automatically at system/user login.
        -   Automatically restarts on crashes, maintaining continuous operation.
        -   Redirects `stdout` and `stderr` to designated log files for easy debugging and auditing.
        -   Uses the correct execution command to run the daemon.
    -   **CLI Commands:** Exposed through the `housaky service` CLI subcommand, enabling users to `install`, `start`, `stop`, `status`, and `uninstall` the daemon as a system service.
    -   **Security:** Includes XML escaping for `plist` generation to prevent potential injection vulnerabilities in service configuration.

This comprehensive daemon and service management system ensures that Housaky can run reliably and autonomously in the background, continuously performing its AGI functions and interacting with various interfaces.

### Daemon and Service Management

Housaky operates as a long-running autonomous agent, and its continuous operation is managed through a robust daemon system integrated with native OS service management. The `src/daemon` and `src/service` modules orchestrate this functionality.

-   **Daemon Core (`src/daemon`):**
    -   **Component Orchestration:** The daemon acts as a supervisor for several core Housaky components, launching and managing them as resilient background tasks. These components include:
        -   **`gateway`:** The HTTP/WebSocket API for external communication.
        -   **`channels`:** All configured communication platforms (Telegram, Discord, etc.) ensuring multi-modal interaction.
        -   **`heartbeat`:** A general heartbeat mechanism (distinct from the Housaky AGI heartbeat).
        -   **`housaky` (AGI Core Heartbeat):** A dedicated worker for the core AGI's continuous self-improvement, learning loops, and autonomous functions.
        -   **`scheduler`:** For managing and executing cron-like scheduled tasks.
    -   **Resilience (`spawn_component_supervisor`):** A critical feature that ensures high availability. If any supervised component fails or exits unexpectedly, it is automatically restarted with an exponential backoff strategy, preventing service interruptions.
    -   **State Persistence (`spawn_state_writer`):** A background task that periodically writes the daemon's current health and component status (e.g., operational status, errors, restart counts) to a `daemon_state.json` file. This provides persistent monitoring data and aids in diagnostics and recovery.
    -   **Graceful Shutdown:** The daemon is designed to respond to standard OS signals (`SIGINT` for Ctrl+C, `SIGTERM` for system service termination). Upon receiving such a signal, it gracefully attempts to abort all running component tasks, ensuring a clean shutdown and preventing data corruption.
-   **OS Service Integration (`src/service`):**
    -   **Platform-Specific Management:** This module provides utilities to integrate the Housaky daemon seamlessly with native operating system service managers, supporting both `systemd --user` on Linux and `launchd` on macOS.
    -   **Service Unit Generation:** It dynamically generates appropriate service unit files (`.service` for `systemd`, `.plist` for `launchd`). These configuration files ensure that the Housaky daemon:
        -   Starts automatically at system/user login.
        -   Automatically restarts on crashes, maintaining continuous operation.
        -   Redirects `stdout` and `stderr` to designated log files for easy debugging and auditing.
        -   Uses the correct execution command to run the daemon.
    -   **CLI Commands:** Exposed through the `housaky service` CLI subcommand, enabling users to `install`, `start`, `stop`, `status`, and `uninstall` the daemon as a system service.
    -   **Security:** Includes XML escaping for `plist` generation to prevent potential injection vulnerabilities in service configuration.

This comprehensive daemon and service management system ensures that Housaky can run reliably and autonomously in the background, continuously performing its AGI functions and interacting with various interfaces.

### Quantum Computing Integration

The `src/quantum` module represents a highly advanced aspect of Housaky's architecture, integrating quantum computing capabilities to accelerate and enhance various AGI functions. This modular framework supports both cloud-based QPUs (Quantum Processing Units) and local simulators.

-   **Modular Design:** The module is structured into distinct sub-modules, each focusing on a specific quantum computing concept or algorithm:
    -   **`backend`:** Provides a unified `QuantumBackend` trait and concrete implementations for `AmazonBraketBackend` (for AWS's quantum computing service) and `SimulatorBackend` (for local, zero-cost simulations). It also includes `BraketDeviceCatalog` for managing device information.
    -   **`circuit`:** Defines the building blocks of quantum programs, including `QuantumCircuit` representation, quantum `Gate`s (`GateType`), and handling `MeasurementResult`s.
    -   **`agi_bridge`:** A critical component (`QuantumAgiBridge`) that directly integrates quantum capabilities into Housaky's AGI core. It aims to accelerate complex AGI tasks:
        -   **Goal Scheduling (QAOA-like):** Optimizes the scheduling of interdependent goals.
        -   **Memory Optimization (Annealing-like):** Optimizes knowledge graph structures or memory recall.
        -   **Reasoning Search (Grover-like):** Accelerates the search through complex reasoning paths or decision spaces.
        -   **Fitness Landscape Exploration (VQE-like):** Aids in the exploration of improvement strategies for self-modification.
    -   **`optimizer`:** Implements quantum optimization algorithms such as `QAOAOptimizer` (Quantum Approximate Optimization Algorithm) for combinatorial problems and `VQEOptimizer` (Variational Quantum Eigensolver) for finding ground states.
    -   **`annealer`:** Provides `QuantumAnnealer` for solving Ising models, useful for optimization problems.
    -   **`grover`:** Implements `GroverSearch` for quantum search algorithms, offering quadratic speedup for unstructured search problems.
    -   **`hybrid_solver`:** Supports `HybridSolver`s that combine classical and quantum computing techniques to tackle problems beyond current quantum hardware limitations.
    -   **`error_mitigation`:** Addresses the challenge of noise in quantum hardware by implementing various `ErrorMitigator`s and `MitigationStrategy`s to improve result accuracy.
    -   **`tomography`:** Allows for the `StateTomographer` to reconstruct quantum states, essential for verifying quantum operations.
    -   **`transpiler`:** Provides `CircuitTranspiler` for optimizing and adapting quantum circuits to the native gate sets of specific QPUs, crucial for efficient execution.
    -   **`braket_tasks`:** Manages task submission, monitoring (`BraketTaskManager`), and `CostTracker` for Amazon Braket, facilitating efficient use of cloud quantum resources.

This integration represents Housaky's commitment to exploring cutting-edge computational paradigms, aiming to leverage quantum advantage for complex AGI problems that are intractable for classical computers. It directly contributes to the agent's self-improvement and problem-solving capabilities.

### Quantum Computing Integration

The `src/quantum` module represents a highly advanced aspect of Housaky's architecture, integrating quantum computing capabilities to accelerate and enhance various AGI functions. This modular framework supports both cloud-based QPUs (Quantum Processing Units) and local simulators.

-   **Modular Design:** The module is structured into distinct sub-modules, each focusing on a specific quantum computing concept or algorithm:
    -   **`backend`:** Provides a unified `QuantumBackend` trait and concrete implementations for `AmazonBraketBackend` (for AWS's quantum computing service) and `SimulatorBackend` (for local, zero-cost simulations). It also includes `BraketDeviceCatalog` for managing device information.
    -   **`circuit`:** Defines the building blocks of quantum programs, including `QuantumCircuit` representation, quantum `Gate`s (`GateType`), and handling `MeasurementResult`s.
    -   **`agi_bridge`:** A critical component (`QuantumAgiBridge`) that directly integrates quantum capabilities into Housaky's AGI core. It aims to accelerate complex AGI tasks:
        -   **Goal Scheduling (QAOA-like):** Optimizes the scheduling of interdependent goals.
        -   **Memory Optimization (Annealing-like):** Optimizes knowledge graph structures or memory recall.
        -   **Reasoning Search (Grover-like):** Accelerates the search through complex reasoning paths or decision spaces.
        -   **Fitness Landscape Exploration (VQE-like):** Aids in the exploration of improvement strategies for self-modification.
    -   **`optimizer`:** Implements quantum optimization algorithms such as `QAOAOptimizer` (Quantum Approximate Optimization Algorithm) for combinatorial problems and `VQEOptimizer` (Variational Quantum Eigensolver) for finding ground states.
    -   **`annealer`:** Provides `QuantumAnnealer` for solving Ising models, useful for optimization problems.
    -   **`grover`:** Implements `GroverSearch` for quantum search algorithms, offering quadratic speedup for unstructured search problems.
    -   **`hybrid_solver`:** Supports `HybridSolver`s that combine classical and quantum computing techniques to tackle problems beyond current quantum hardware limitations.
    -   **`error_mitigation`:** Addresses the challenge of noise in quantum hardware by implementing various `ErrorMitigator`s and `MitigationStrategy`s to improve result accuracy.
    -   **`tomography`:** Allows for the `StateTomographer` to reconstruct quantum states, essential for verifying quantum operations.
    -   **`transpiler`:** Provides `CircuitTranspiler` for optimizing and adapting quantum circuits to the native gate sets of specific QPUs, crucial for efficient execution.
    -   **`braket_tasks`:** Manages task submission, monitoring (`BraketTaskManager`), and `CostTracker` for Amazon Braket, facilitating efficient use of cloud quantum resources.

This integration represents Housaky's commitment to exploring cutting-edge computational paradigms, aiming to leverage quantum advantage for complex AGI problems that are intractable for classical computers. It directly contributes to the agent's self-improvement and problem-solving capabilities.

### Hardware Interaction and Peripheral Management

Housaky extends its capabilities into the physical world through a comprehensive framework for hardware interaction and peripheral management, primarily driven by the `src/hardware` and `src/peripherals` modules. This allows the AGI to perceive and act upon its physical environment.

-   **Feature-Gated Functionality:** The entire hardware stack is guarded by the `hardware` Cargo feature, ensuring that only necessary components are compiled for deployments that require physical interaction.
-   **Hardware Discovery (`src/hardware`):**
    -   **USB Device Enumeration:** The `discover` sub-module identifies and lists connected USB devices, providing essential details like Vendor ID (VID), Product ID (PID), board name, and architectural information.
    -   **Device Introspection:** The `introspect` sub-module allows for deeper analysis of specific devices via their paths (e.g., serial port), extracting detailed operational information.
    -   **`probe-rs` Integration:** When the `probe` Cargo feature is enabled, Housaky leverages `probe-rs` to communicate with hardware debugging probes (e.g., ST-Link). This enables low-level access to microcontrollers (like STM32/Nucleo) for reading chip information (memory maps, architecture) directly through the debug interface, without requiring specific firmware on the target device.
    -   **Wizard Integration:** Functions exist to streamline user setup, guiding them through configuring hardware via interactive CLI wizards.
-   **Peripheral Management and Tool Creation (`src/peripherals`):**
    -   **`Peripheral` Trait:** Defines a unified interface for various types of physical peripherals, abstracting their control mechanisms.
    -   **Dynamic Tool Generation:** The `create_peripheral_tools()` function dynamically converts configured hardware into actionable tools (`Box<dyn Tool>`) for the Housaky agent. This includes:
        -   **GPIO Tools:** For basic digital input/output (e.g., `UnoQGpioReadTool`, `UnoQGpioWriteTool`, Raspberry Pi GPIO).
        -   **Serial Communication Tools:** For interacting with microcontrollers (STM32, ESP32, Arduino) via serial ports.
        -   **Firmware Flashing Tools:** `ArduinoUploadTool` for agent-generated Arduino sketches, and dedicated tools for `arduino_flash` and `nucleo_flash` to deploy firmware.
        -   **Hardware Information Tools:** `HardwareMemoryMapTool`, `HardwareBoardInfoTool`, `HardwareMemoryReadTool` provide the agent with self-awareness about the physical characteristics and memory layout of connected boards.
        -   **Capabilities Tool:** `HardwareCapabilitiesTool` allows the agent to query connected hardware for available GPIO pins and LED configurations.
    -   **Platform-Specific Implementations:** Includes specific implementations for Raspberry Pi GPIO (`rpi`) when `peripheral-rpi` and `target_os = "linux"` features are enabled.
-   **CLI Commands:** Both `housaky hardware` and `housaky peripheral` subcommands are exposed for users to `discover`, `introspect`, `info` (about chips), `list`, `add`, and `flash` firmware.

This robust hardware interaction and peripheral management system empowers Housaky's AGI with the ability to perceive, analyze, and directly influence the physical world, enabling a new class of autonomous real-world applications.

### Hardware Interaction and Peripheral Management

Housaky extends its capabilities into the physical world through a comprehensive framework for hardware interaction and peripheral management, primarily driven by the `src/hardware` and `src/peripherals` modules. This allows the AGI to perceive and act upon its physical environment.

-   **Feature-Gated Functionality:** The entire hardware stack is guarded by the `hardware` Cargo feature, ensuring that only necessary components are compiled for deployments that require physical interaction.
-   **Hardware Discovery (`src/hardware`):**
    -   **USB Device Enumeration:** The `discover` sub-module identifies and lists connected USB devices, providing essential details like Vendor ID (VID), Product ID (PID), board name, and architectural information.
    -   **Device Introspection:** The `introspect` sub-module allows for deeper analysis of specific devices via their paths (e.g., serial port), extracting detailed operational information.
    -   **`probe-rs` Integration:** When the `probe` Cargo feature is enabled, Housaky leverages `probe-rs` to communicate with hardware debugging probes (e.g., ST-Link). This enables low-level access to microcontrollers (like STM32/Nucleo) for reading chip information (memory maps, architecture) directly through the debug interface, without requiring specific firmware on the target device.
    -   **Wizard Integration:** Functions exist to streamline user setup, guiding them through configuring hardware via interactive CLI wizards.
-   **Peripheral Management and Tool Creation (`src/peripherals`):**
    -   **`Peripheral` Trait:** Defines a unified interface for various types of physical peripherals, abstracting their control mechanisms.
    -   **Dynamic Tool Generation:** The `create_peripheral_tools()` function dynamically converts configured hardware into actionable tools (`Box<dyn Tool>`) for the Housaky agent. This includes:
        -   **GPIO Tools:** For basic digital input/output (e.g., `UnoQGpioReadTool`, `UnoQGpioWriteTool`, Raspberry Pi GPIO).
        -   **Serial Communication Tools:** For interacting with microcontrollers (STM32, ESP32, Arduino) via serial ports.
        -   **Firmware Flashing Tools:** `ArduinoUploadTool` for agent-generated Arduino sketches, and dedicated tools for `arduino_flash` and `nucleo_flash` to deploy firmware.
        -   **Hardware Information Tools:** `HardwareMemoryMapTool`, `HardwareBoardInfoTool`, `HardwareMemoryReadTool` provide the agent with self-awareness about the physical characteristics and memory layout of connected boards.
        -   **Capabilities Tool:** `HardwareCapabilitiesTool` allows the agent to query connected hardware for available GPIO pins and LED configurations.
    -   **Platform-Specific Implementations:** Includes specific implementations for Raspberry Pi GPIO (`rpi`) when `peripheral-rpi` and `target_os = "linux"` features are enabled.
-   **CLI Commands:** Both `housaky hardware` and `housaky peripheral` subcommands are exposed for users to `discover`, `introspect`, `info` (about chips), `list`, `add`, and `flash` firmware.

This robust hardware interaction and peripheral management system empowers Housaky's AGI with the ability to perceive, analyze, and directly influence the physical world, enabling a new class of autonomous real-world applications.

### Observability and Monitoring

Housaky incorporates a comprehensive observability framework to provide deep insights into its internal operations, facilitate debugging, and ensure reliable autonomous behavior. This system is highly configurable and supports various backends for data collection and export.

-   **Modular Observability Backends:** The `src/observability` module supports different data collection strategies through a pluggable backend system:
    -   **`log`:** A basic logging observer for standard console or file output, providing human-readable event streams.
    -   **`otel` (OpenTelemetry):** A robust integration with the OpenTelemetry standard, enabling the export of traces and metrics to external collectors and monitoring systems (e.g., Jaeger for traces, Prometheus for metrics). This is highly configurable via `otel_endpoint` and `otel_service_name`.
    -   **`noop`:** A no-operation observer, used when observability is disabled to minimize overhead, or in testing scenarios.
    -   **`multi`:** Allows combining multiple observers to send data to different destinations simultaneously.
    -   **`verbose`:** Provides more detailed logging for in-depth debugging sessions.
-   **`Observer` Trait:** The core `Observer` trait defines a unified interface for all observability backends. It exposes a `record_event()` method, which accepts `ObserverEvent`s to capture various operational data points within Housaky.
-   **`ObserverEvent` Enum:** A structured enum that categorizes and defines the types of events emitted by different components of Housaky (e.g., `AgentStart`, `LlmRequest`, `ToolCall`, `Error`, `Heartbeat`). This ensures consistent and semantically rich event data.
-   **Dynamic Observer Creation:** The `create_observer` factory function dynamically initializes the appropriate observer backend based on the `ObservabilityConfig` provided in `config.toml`, allowing users to choose their preferred observability strategy.
-   **Running Registry (`running_registry.rs`):** A vital component for monitoring the overall health and activity of the Housaky ecosystem. It tracks all active Housaky instances (daemon, agents, channels) by collecting periodic heartbeats, providing a real-time "map" of which components are operational. This is essential for multi-process and multi-agent deployments.
-   **Flight Journal (`flight_journal.rs`):** Offers a day-partitioned, persistent journal for an agent's detailed execution history. It records a chronological sequence of events, LLM interactions, tool calls, and decisions, providing an invaluable forensic tool for debugging complex AGI behaviors and understanding the agent's thought process.

This comprehensive observability framework is essential for the development, deployment, and reliable operation of Housaky. It provides the necessary transparency to understand its autonomous actions, diagnose issues, and continuously improve its performance and AGI capabilities.

### Pluggable Features

The backend is highly modular, with many features controlled by Cargo features. This allows for compiling different versions of the application with varying capabilities. Key features include `hardware`, `peripheral-rpi`, `probe`, `rag-pdf`, `web-monitoring`, and `runtime-wasm`.

## 3. Desktop Application (Tauri + Vue.js)

The primary user interface for Housaky is a cross-platform desktop application located in the `dashboard` directory.

### Architecture

- **Framework:** The application is built with [Tauri](https://tauri.app/), which allows for a Rust backend and a web-based frontend. The Rust backend is the same core application described above.
- **Frontend Stack:** The frontend is a [Vue.js](https://vuejs.org/) application, built with [Vite](https://vitejs.dev/) and styled with [Tailwind CSS](https://tailwindcss.com/). It is written in [TypeScript](https://www.typescriptlang.org/).
- **Tauri Integration:** The Vue.js frontend communicates with the Rust backend through the Tauri API. It uses several Tauri plugins to access native system functionality from the frontend, including the file system, shell, HTTP client, and more.

## 4. Landing Page (Vue.js + Vite)

The project includes a simple, static landing page in the `landing` directory. It is also built with Vue.js and Vite and styled with Tailwind CSS. It is a separate application from the main desktop app and serves as the project's public-facing website.

## 5. Services and Deployment (Docker)

The `docker-compose.yml` file defines a simple and convenient way to deploy the Housaky service.

- **Single Service:** The setup consists of a single `housaky` service.
- **Container Image:** It uses a pre-built Docker image from the GitHub Container Registry (`ghcr.io`) but can also be built locally.
- **Configuration:** The service is configured through environment variables.
- **Persistence:** A named Docker volume is used to persist the agent's workspace and configuration, ensuring that state is maintained across container restarts.
- **Networking:** The service exposes the agent's gateway API on port 3000, allowing other applications to interact with it.

## 6. Communication and Data Flow

- **Desktop App:** In the desktop application, the Vue.js frontend communicates with the Rust backend via the Tauri bridge. This is a secure and efficient inter-process communication (IPC) mechanism. The Rust backend can also perform native operations and communicate with external services, sending the results back to the frontend.
- **Docker Deployment:** In the Docker deployment, the Housaky service runs as a standalone container. External applications can communicate with the agent through the exposed gateway API on port 3000.
- **External Services:** The core Rust backend can communicate with a variety of external services, including LLM providers, AWS, Discord, and ElevenLabs, using HTTP and WebSocket clients.
