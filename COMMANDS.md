# Housaky Commands

This document provides an overview of the available commands in the `housaky` CLI.

## 1. Core Commands

These are the main commands for running the different parts of the Housaky system.

- `housaky onboard`: Initializes the workspace and configuration.
  - `--interactive`: Runs the full interactive wizard.
  - `--channels-only`: Reconfigures channels only.
  - `--api-key <KEY>`: Sets the API key in quick mode.
  - `--provider <PROVIDER>`: Sets the default provider in quick mode.
  - `--memory <BACKEND>`: Sets the memory backend in quick mode.

- `housaky agent`: Starts the AI agent loop.
  - `-m, --message <MESSAGE>`: Runs in single message mode.
  - `-p, --provider <PROVIDER>`: Specifies the provider to use.
  - `--model <MODEL>`: Specifies the model to use.
  - `-t, --temperature <TEMP>`: Sets the temperature for the model.
  - `--peripheral <PERIPHERAL>`: Attaches a hardware peripheral.

- `housaky gateway`: Starts the gateway server for webhooks and WebSockets.
  - `-p, --port <PORT>`: Sets the port to listen on.
  - `--host <HOST>`: Sets the host to bind to.

- `housaky daemon`: Manages the long-running autonomous runtime.
  - `start`: Starts the daemon.
  - `stop`: Stops the daemon.
  - `restart`: Restarts the daemon.
  - `status`: Shows the daemon status.

- `housaky run`: Starts the full AGI system with a TUI chat interface.
  - `-m, --message <MESSAGE>`: Sends an initial message.
  - `-p, --provider <PROVIDER>`: Specifies the provider to use.
  - `--model <MODEL>`: Specifies the model to use.
  - `-v, --verbose`: Enables verbose output.

- `housaky tui`: Launches the terminal user interface for AI chat.
  - `-p, --provider <PROVIDER>`: Specifies the provider to use.
  - `--model <MODEL>`: Specifies the model to use.

- `housaky dashboard`: Starts or checks the status of the Housaky Dashboard.
  - `--start`: Starts the dashboard server.
  - `--host <HOST>`: Sets the host to bind to.
  - `-p, --port <PORT>`: Sets the port to listen on.
  - `-o, --open`: Opens the dashboard in a browser.
  - `--desktop`: Launches the desktop app.

## 2. Management Commands

These commands are for managing various aspects of the Housaky system.

- `housaky service`: Manages the OS service lifecycle (e.g., `systemd`).
  - `install`: Installs the daemon service.
  - `start`: Starts the daemon service.
  - `stop`: Stops the daemon service.
  - `status`: Checks the daemon service status.
  - `uninstall`: Uninstalls the daemon service.

- `housaky channel`: Manages communication channels.
  - `list`: Lists all configured channels.
  - `start`: Starts all configured channels.
  - `doctor`: Runs health checks for channels.
  - `add`: Adds a new channel.
  - `remove`: Removes a channel.

- `housaky skills`: Manages skills.
  - `ui`: Opens the skills marketplace TUI.
  - `list`: Lists all installed skills.
  - `install`: Installs a new skill.
  - `remove`: Removes an installed skill.
  - `convert`: Converts a Claude `SKILL.md` to a Housaky `SKILL.toml`.
  - `get`: Installs a skill by name.

- `housaky keys`: Manages API keys and providers.
  - `list`: Lists all configured API keys.
  - `add`: Adds a new API key.
  - `remove`: Removes an API key.
  - `manager`: Enters the centralized keys/provider/model manager.

- `housaky cron`: Manages scheduled tasks.
  - `list`: Lists all scheduled tasks.
  - `add`: Adds a new cron job.
  - `once`: Adds a one-shot delayed task.
  - `remove`: Removes a scheduled task.
  - `pause`: Pauses a scheduled task.
  - `resume`: Resumes a paused task.

- `housaky models`: Manages provider model catalogs.
  - `refresh`: Refreshes and caches provider models.

- `housaky hardware`: Discovers and introspects hardware.
  - `discover`: Enumerates USB devices.
  - `introspect`: Introspects a device by its path.
  - `info`: Gets chip info via USB.

- `housaky peripheral`: Manages hardware peripherals.
  - `list`: Lists configured peripherals.
  - `add`: Adds a new peripheral.
  - `flash`: Flashes Housaky firmware to an Arduino.
  - `setup-uno-q`: Sets up the Arduino Uno Q Bridge app.
  - `flash-nucleo`: Flashes Housaky firmware to a Nucleo-F401RE.

## 3. Utility Commands

- `housaky doctor`: Runs diagnostics and health checks.
  - `run`: Runs all diagnostics.
  - `fix`: Runs diagnostics and attempts to apply fixes.
  - `channels`: Runs channel health checks only.
  - `security`: Runs security audit checks only.
  - `json`: Shows results as JSON.

- `housaky status`: Shows the system status.

- `housaky config`: Opens an interactive configuration editor.
  - `-s, --section <SECTION>`: Opens a specific section.
  - `--reset`: Resets the configuration to defaults.
  - `--restore`: Restores the configuration from a backup.

- `housaky migrate`: Migrates data from other agent runtimes.
  - `openclaw`: Imports memory from an `OpenClaw` workspace.

- `housaky integrations`: Browses available integrations.
  - `info`: Shows details about a specific integration.

## 4. Housaky AGI Commands

These commands are for interacting with the core AGI functionality.

- `housaky init`: Initializes the Housaky AGI system.
- `housaky heartbeat`: Triggers a manual heartbeat cycle.
- `housaky tasks`: Shows current tasks.
- `housaky review`: Shows a state review.
- `housaky improve`: Forces a self-improvement cycle.
- `housaky connect-kowalski`: Connects to Kowalski agents.
- `housaky agi-session`: Starts an AGI mode interactive session.
- `housaky thoughts`: Shows the agent's inner monologue.
- `housaky goals`: Manages the agent's goals (`list`, `add`, `complete`).
- `housaky self-mod`: Manages self-modification parameters (`run`, `status`, `experiments`, `set`, `unset`).

## 5. GSD Orchestration Commands (`housaky gsd`)

These commands are part of the "Get-Shit-Done" workflow.

- `housaky gsd new-project`: Initializes a new GSD project.
- `housaky gsd phase`: Creates a new phase.
- `housaky gsd discuss`: Captures implementation decisions for a phase.
- `housaky gsd execute`: Plans and executes a phase.
- `housaky gsd quick`: Executes a task directly.
- `housaky gsd verify`: Verifies phase completion.
- `housaky gsd status`: Shows the current phase status.
- `housaky gsd analyze`: Analyzes task complexity.
- `housaky gsd awareness`: Shows an awareness report.

### Slash Commands (Interactive Mode)

- `/gsd:new-project`: Initializes project planning.
- `/gsd:plan-phase`: Creates an execution plan for a phase.
- `/gsd:execute-phase`: Runs the parallel execution for a phase.
- `/gsd:debug`: Starts a systematic debugging session.

## 6. Quantum Commands (`housaky quantum`)

These commands are for interacting with quantum computing services and simulators.

- `housaky quantum run-braket`: Runs a circuit on Amazon Braket.
- `housaky quantum run-simulator`: Runs a circuit on the local simulator.
- `housaky quantum device-info`: Shows information about a Braket device.
- `housaky quantum devices`: Lists all known Braket devices.
- `housaky quantum estimate-cost`: Estimates the cost for a quantum task.
- `housaky quantum transpile`: Transpiles a circuit for a target device.
- `housaky quantum tomography`: Runs quantum state tomography.
- `housaky quantum agi-bridge`: Runs the quantum AGI bridge demo.
- `housaky quantum tasks`: Lists recent Braket tasks.
- `housaky quantum benchmark`: Runs quantum advantage benchmarks.
- `housaky quantum metrics`: Shows live quantum AGI bridge metrics.

## 7. Collective Intelligence Commands (`housaky collective`)

These commands are for interacting with the global collective intelligence network.

- `housaky collective bootstrap`: Bootstraps the instance on the Moltbook network.
- `housaky collective status`: Shows the collective system status.
- `housaky collective submit`: Submits a contribution proposal.
- `housaky collective tick`: Polls for proposals and casts votes.
- `housaky collective list`: Lists all locally cached contributions.
- `housaky collective votes`: Fetches live vote counts for a contribution.
- `housaky collective search`: Searches for proposals.
- `housaky collective register`: Registers the instance as a Moltbook agent.
