# Housaky Dashboard

Cross-device web interface for Housaky AI Assistant built with Tauri + Vue 3 + Tailwind CSS 4.

## Features

- 📊 **Dashboard** - Overview of system status, channels, and quick actions
- 💬 **Chat** - Interactive chat with your AI assistant
- 🔧 **Skills** - Manage and enable/disable skills
- 📡 **Channels** - Configure messaging channels (Telegram, Discord, Slack, WhatsApp, etc.)
- 🔌 **Integrations** - Browse 75+ integrations across 9 categories
- 💻 **Hardware** - Discover and manage USB devices and microcontroller boards
- ⚙️ **Config** - Edit Housaky configuration (fully synced with `~/.housaky/config.toml`)
- 📟 **Terminal** - Execute Housaky commands directly

## Requirements

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 20+
- For Linux: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`

## Development

```bash
# Install dependencies
cd dashboard
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Testing

```bash
# Run unit tests
npm test

# Run unit tests with UI
npm run test:ui

# Run e2e tests
npm run test:e2e

# Run e2e tests with UI
npm run test:e2e:ui
```

## Architecture

```
dashboard/
├── src/
│   ├── components/ui/     # shadcn-vue UI components
│   ├── views/             # Page components (Dashboard, Chat, Config, etc.)
│   ├── composables/       # Vue composables
│   ├── lib/               # Utilities
│   ├── config/            # Navigation config
│   └── config-sync.test.ts # Config sync unit tests
├── src-tauri/            # Tauri Rust backend
│   ├── src/
│   │   └── main.rs       # Tauri commands (get_config, save_config, etc.)
│   └── tauri.conf.json   # Tauri configuration
├── tests/
│   └── e2e.spec.ts       # Playwright e2e tests
├── playwright.config.ts  # Playwright configuration
├── vitest.config.ts      # Vitest configuration
└── package.json
```

## Config Sync

The dashboard maintains **bidirectional sync** with Housaky's config file:

### Reading Config
- On load, the dashboard reads `~/.housaky/config.toml`
- Parses TOML into TypeScript interfaces
- Populates all form fields

### Writing Config
- Changes are tracked (shows "Modified" badge)
- Save writes back to `~/.housaky/config.toml`
- Immediate sync with Housaky daemon

### Commands Used
- `get_status` - Get system status
- `get_config` - Load full config
- `save_config` - Persist config changes
- `send_message` - Chat with AI
- `check_housaky_installed` - Verify installation
- `run_doctor` - Run diagnostics
- `get_channels` / `configure_channel` - Channel management
- `get_skills` / `toggle_skill` - Skill management
- `hardware_discover` - USB device discovery

## Connecting to Housaky

The dashboard communicates with Housaky via:

1. **CLI Commands** - Executes `housaky` CLI commands directly
2. **IPC** - Tauri commands in `main.rs` wrap CLI calls

Make sure Housaky is installed and in your PATH:

```bash
cargo build --release --locked
cargo install --path . --force --locked
housaky --version
```

## Building

### Local Build

```bash
cd dashboard
npm install
npm run tauri build
```

### GitHub Actions

The `.github/workflows/dashboard.yml` workflow:
- Runs unit tests on every PR
- Builds for Linux (x64), macOS (x64, arm64), Windows (x64)
- Creates release artifacts automatically

## Tech Stack

- **Frontend**: Vue 3, Vue Router, Tailwind CSS 4, shadcn-vue
- **Backend**: Tauri 2 (Rust)
- **Build**: Vite, TypeScript
- **Testing**: Vitest (unit), Playwright (e2e)

## Views

| View | Description |
|------|-------------|
| Dashboard | System overview, channel status, quick actions |
| Chat | Interactive AI chat interface |
| Skills | Enable/disable skills, view skill info |
| Channels | Configure Telegram, Discord, Slack, WhatsApp, etc. |
| Integrations | Browse 75+ integrations across 9 categories |
| Hardware | USB device discovery, board management |
| Config | Full config editor with live sync |
| Terminal | Execute Housaky CLI commands |
