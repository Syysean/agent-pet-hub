# Agent Pet Hub 🐾

> Unified Desktop Pet Hub for AI Agents — A Tauri desktop app that visualizes AI Agent runtime events into animated pets on your screen.

[![TypeScript](https://img.shields.io/badge/TypeScript-5.6%2B-blue)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](https://rust-lang.org/)
[![React](https://img.shields.io/badge/React-19-61DAFB)](https://react.dev/)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-22C55E)](https://tauri.app/)
[![License](https://img.shields.io/badge/License-MIT-green)](./LICENSE)

Agent Pet Hub is a cross-platform desktop pet application built on **Tauri 2.x**. It monitors AI Agent runtime events (Pi, Hermes, OpenClaw) in real time and renders animated pet responses on screen. Built with a clean layered architecture featuring **Adapter pattern**, **Event Bus**, **State Machine**, and a **plugin-based skin system**.

---

## 📑 Table of Contents

- [Features](#-features)
- [Architecture](#-architecture)
  - [System Architecture](#system-architecture)
  - [Data Flow](#data-flow)
  - [Module Structure](#module-structure)
- [Dependencies](#-dependencies)
- [Quick Start](#-quick-start)
- [Project Structure](#-project-structure)
- [Core Components](#-core-components)
  - [Adapter Layer](#adapter-layer)
  - [Event Bus](#event-bus)
  - [State Machine](#state-machine)
  - [Skin/Plugin System](#skinplugin-system)
  - [IPC Layer](#ipc-layer)
- [Protocol](#-protocol)
  - [Event Types](#event-types)
  - [State Machine Transitions](#state-machine-transitions)
- [Configuration](#-configuration)
- [Development](#-development)
- [Comparison with Similar Projects](#-comparison-with-similar-projects)
- [License](#-license)

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🎭 **Multi-Agent Support** | Pi Agent, Hermes, OpenClaw — extensible to Claude Code, Gemini CLI, Cursor etc. |
| 🔄 **Unified Protocol** | All agent events normalized to a single schema via `@agent-pet-hub/protocol` |
| 🤖 **8 Pet States** | Idle, Thinking, Working, Waiting, Success, Error, Speaking, Connecting |
| 🔊 **TTS Voice** | Cross-platform text-to-speech (macOS `say` / Linux `espeak` / Windows) |
| 📡 **WebSocket IPC** | External processes subscribe to agent events via WebSocket (port 8765) |
| 🎨 **Skin System** | PNG/SVG skins with frame animation + user-customizable skins directory |
| 💻 **System Tray** | Tray icon with status indicator + window controls |
| 🔒 **State Debounce** | 500ms state retention prevents flickering during rapid event bursts |

---

## 📐 Architecture

### System Architecture

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                         EXTERNAL AGENTS                                      │
│  ┌──────────┐    ┌──────────┐    ┌───────────┐                               │
│  │  Pi Agent│    │ Hermes   │    │ OpenClaw  │                               │
│  │(JSONL)   │    │(HTTP)    │    │ (HTTP)    │                               │
│  └────┬─────┘    └──────────┘    └───────────┘                               │
└───────┼──────────────────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                    RUST BACKEND (agent-pet-hub-lib)                          │
│                                                                              │
│  ┌──────────────┐      ┌──────────────┐      ┌────────────────────────────┐  │
│  │  Agent       │────▶│  Event       │────▶│  EventBus (broadcast)      │  │
│  │  Adapter     │      │  Converter   │      │  (tokio sync)              │  │
│  │  (Trait)     │      │              │      └──────┬─────────────────────┘  │
│  └──────────────┘      └──────────────┘            │                         │
│       │                             ┌─────────────▼──────────┐               │
│       │                             │  PetStateMachine       │               │
│       │                             │  (31 transition rules) │               │
│       │                             └──────┬──────────┬──────┘               │
│       │                                    │          │                      │
│       │              ┌─────────────────────┘          │                      │
│       │              ▼                                ▼                      │
│  ┌──────────────┐  ┌────────────┐         ┌──────────────────────────┐       │
│  │  PiJsonl     │  │  TTS       │         │  Tauri Commands (IPC)    │       │
│  │  Watcher     │  │  Engine    │         │  (get/set state,         │       │
│  │  (notify fs  │  │  (espeak/  │         │   settings, skin mgmt)   │       │
│  │   monitor)   │  │   say)     │         └──────┬───────────────────┘       │
│  └──────────────┘  └────────────┘                │                           │
│       ┌───────────────────────────────────────────┼─────────────────────┐    │
│       │                                           ▼                     │    │
│       │                        ┌──────────────────────────────┐         │    │
│       └───────────────────────▶│  WebSocket Server            │        │    │
│                                │  (port 8765, auth + events)  │         │    │
│                                └──────────────────────────────┘         │    │
└──────────────────────────────────────────────────────────────────────────────┘
        │                              │
        ▼                              ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                         FRONTEND (React + TypeScript)                        │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  Pet Window (320x280, transparent, always-on-top, no decorations)      │  │
│  │                                                                        │  │
│  │  ┌──────────────────────────────────────────────────────────────────┐  │  │
│  │  │  PetPNG / PetSVG Component                                       │  │  │
│  │  │  - State-driven frame switching                                  │  │  │
│  │  │  - CSS @keyframes animations (breathing, thinking, working...)   │  │  │
│  │  │  - Drag support (Tauri start_dragging API)                       │  │  │
│  │  │  - Status indicator overlay (bottom center)                      │  │  │
│  │  └──────────────────────────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  State Management: Zustand store (PetStateSnapshot)                          │
│  Communication: Tauri Events (listen for "pet:state_changed", "pet:event")   │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
1. Agent Event
   ↓
2. Adapter (e.g., PiAdapter reads JSONL file)
   ↓
3. EventConverter (normalizes to UnifiedAgentEvent)
   ↓
4. EventBus::publish_event() (broadcast channel)
   ↓
5. PetStateMachine::handle_event() → State Transition
   ↓
6. EventBus::publish_state_change() (broadcast channel)
   ┌─────────────────┬─────────────────┐
   ▼                 ▼                 ▼
7a. Rust: TTS    7b. Rust: Tray     7c. Tauri Events
    Engine           Icon Color           ↓
   Speak              Update           emit("pet:state_changed")
   Text               Update                         ↓
                      Update               Frontend: listen() → Zustand
```

### Module Structure

```
agent-pet-hub/                          # Monorepo root
├── packages/protocol/                  # Shared event protocol (TS)
│   ├── src/events.ts                   #   Type definitions
│   ├── src/schemas.ts                  #   Zod schemas
│   ├── src/validators.ts              #   Runtime validators
│   └── src/mapping.ts                 #   Agent-specific event mappings
│
├── src-tauri/                          # Rust backend (Tauri 2.x)
│   ├── src/
│   │   ├── main.rs                     #   Binary entry point
│   │   ├── lib.rs                      #   Library entry + run()
│   │   ├── commands.rs                #   Tauri invoke handlers
│   │   ├── adapter/                   #   Agent adapter layer
│   │   │   ├── mod.rs                 #     Module exports
│   │   │   ├── trait.rs               #     AgentAdapter trait
│   │   │   ├── pi_adapter.rs          #     Pi Agent adapter impl
│   │   │   ├── pi_watcher.rs          #     JSONL file watcher
│   │   │   └── event_converter.rs     #     Pi → Unified conversion
│   │   ├── event_bus/                 #   Event bus (broadcast)
│   │   │   ├── mod.rs                 #
│   │   │   └── bus.rs                 #     EventBus core
│   │   ├── state_machine/             #   State machine
│   │   │   ├── mod.rs                 #
│   │   │   ├── machine.rs             #     PetStateMachine
│   │   │   └── transitions.rs         #     Transition rules (31 rules)
│   │   ├── config/                    #   Configuration management
│   │   │   ├── mod.rs                 #
│   │   │   └── settings.rs            #     SettingsManager
│   │   ├── ipc/                       #   WebSocket server
│   │   │   ├── mod.rs                 #
│   │   │   └── ws_server.rs           #     WSServer (tokio-tungstenite)
│   │   ├── plugin/                    #   Plugin system
│   │   │   ├── mod.rs                 #
│   │   │   └── manager.rs             #     PluginManager
│   │   ├── skin.rs                    #   Skin scanning
│   │   ├── tts/                       #   TTS engine
│   │   │   ├── mod.rs                 #
│   │   │   └── engine.rs              #     TTSEngine
│   │   ├── types/                     #   Rust type definitions
│   │   │   ├── mod.rs                 #
│   │   │   ├── events.rs              #     AgentSource, EventType, PetState...
│   │   │   └── pet.rs                 #     PetSkin, PetPosition...
│   │   └── window/                    #   Window management
│   │       ├── mod.rs                 #
│   │       ├── pet_window.rs          #     create_pet_window()
│   │       └── tray.rs                #     create_tray()
│   ├── resources/skins/               #   Built-in SVG skins
│   ├── tauri.conf.json                #   Tauri app config
│   ├── capabilities/                  #   Webview permissions
│   ├── Cargo.toml                     #   Rust dependencies
│   └── build.rs                       #   Build script
│
├── src/                                # Frontend (React + Vite)
│   ├── main.tsx                        #   React entry point
│   ├── App.tsx                         #   Root component
│   ├── index.css                       #   Global styles
│   ├── hooks/
│   │   ├── useAgentState.ts            #   Agent state hook (Tauri events)
│   │   └── useSkinLoader.ts            #   Skin loading hook
│   ├── components/
│   │   ├── PetPNG.tsx                  #   PNG skin renderer (primary)
│   │   ├── PetSVG.tsx                  #   SVG skin renderer (legacy)
│   │   ├── PetSVGLegacy.tsx            #   Legacy SVG renderer
│   │   ├── PetStatus.tsx               #   Status text overlay
│   │   └── SkinSelector.tsx            #   Skin picker UI
│   ├── services/
│   │   └── wsClient.ts                 #   WebSocket client
│   └── types/
│       ├── pet.ts                      #   Frontend pet types
│       ├── events.ts                   #   Frontend event types
│       └── skin.ts                     #   Frontend skin types
│
├── src/assets/skins/                   # Built-in PNG skins
│   └── shark/                          #   Shark skin (120x120 PNG frames)
│       ├── skin.json                   #     Metadata
│       ├── idle.png, thinking.png, ... #     Frame images
│       └── backup/                     #     Backup frames
│
├── dist/                               # Build output (frontend)
├── tsconfig.json                       #   TypeScript config
├── vite.config.ts                      #   Vite config
├── package.json                        #   Node scripts
├── pnpm-lock.yaml                      #   Lockfile
└── README.md                           #   This file
```

---

## 📦 Dependencies

### Runtime

| Dependency | Purpose | Minimum Version |
|------------|---------|-----------------|
| [Tauri 2.x](https://tauri.app/) | Desktop app framework | 2.x |
| [Rust](https://rust-lang.org/) | Backend logic | 1.75+ |
| [React 19](https://react.dev/) | UI framework | 19.x |
| [TypeScript](https://www.typescriptlang.org/) | Type safety | 5.6+ |
| [Vite](https://vitejs.dev/) | Frontend build tool | 6.x |
| [Zustand](https://zustand.pm/) | State management | 5.x |
| [tokio](https://tokio.rs/) | Async runtime (Rust) | 1.x |
| [tokio-tungstenite](https://docs.rs/tokio-tungstenite/) | WebSocket server (Rust) | 0.26.x |
| [notify](https://docs.rs/notify/) | File system monitoring | 8.x |
| [serde](https://serde.rs/) | Serialization (Rust) | 1.x |

### Build Tools

- `pnpm` — Package manager (monorepo workspaces)
- `cargo` — Rust build tool
- `tsc` — TypeScript compiler
- `esbuild` / `rollup` — Bundled by Vite

### Platform-Specific (TTS)

| Platform | TTS Engine | Command |
|----------|-----------|---------|
| macOS | `say` (system) | `say "text"` |
| Linux | `espeak` | `espeak "text"` |
| Windows | Edge-TTS (HTTP API) | _Not yet implemented_ |

### System Dependencies

- **Linux**: `libwebkit2gtk-4.1`, `libssl3`, `libayatana-appindicator3` (for tray)
- **macOS**: Xcode Command Line Tools
- **Windows**: Visual C++ Redistributable

---

## 🚀 Quick Start

### Prerequisites

1. **Rust** — Install via [rustup](https://rustup.rs/):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source "$HOME/.cargo/env"
   ```

2. **Node.js 18+** and **pnpm**:
   ```bash
   # Node.js (example with nvm)
   nvm install 20
   nvm use 20

   # pnpm
   npm install -g pnpm
   ```

3. **System Dependencies (Linux only)**:
   ```bash
   # Ubuntu/Debian
   sudo apt install libwebkit2gtk-4.1-dev \
                    libssl-dev \
                    libayatana-appindicator3-dev \
                    librsvg2-dev

   # Fedora
   sudo dnf install webkit2gtk4.1-devel \
                    openssl-devel \
                    libayatana-appindicator-gtk3-devel \
                    librsvg2-devel
   ```

### Installation

```bash
# 1. Clone the repository
git clone https://github.com/Syysean/agent-pet-hub.git
cd agent-pet-hub

# 2. Install Node.js dependencies
pnpm install
```

### Build

```bash
# Build frontend only (for development)
pnpm build

# Build the full Tauri app (includes Rust compilation)
pnpm tauri build
```

### Run

```bash
# Development mode (hot reload + Tauri window)
pnpm tauri dev

# This runs:
#   1. Vite dev server on http://localhost:1420
#   2. Rust backend (Tauri) connects to the Vite server
#   3. Pet window opens: 320×280, transparent, always-on-top
```

---

## 📂 Project Structure

### `packages/protocol/` — Shared Event Protocol

TypeScript package that defines the unified agent event protocol. Re-exported as `@agent-pet-hub/protocol`.

| File | Purpose |
|------|---------|
| `src/events.ts` | Core type definitions: `AgentSource`, `EventType`, `PetState`, `UnifiedAgentEvent`, `WSMessage`, `TTSEvent`, etc. |
| `src/schemas.ts` | Zod validation schemas for all types |
| `src/validators.ts` | Runtime validators: `validateEvent()`, `tryValidateEvent()` |
| `src/mapping.ts` | Agent-specific event mapping tables (Pi → Unified, Hermes → Unified, etc.) |

### `src-tauri/` — Rust Backend

| Module | Files | Purpose |
|--------|-------|---------|
| **Entry** | `main.rs`, `lib.rs` | Binary entry, `run()` bootstrap, global initialization |
| **Commands** | `commands.rs` | 13 Tauri invoke handlers exposed to frontend |
| **Adapter** | `adapter/trait.rs`, `pi_adapter.rs`, `pi_watcher.rs`, `event_converter.rs` | Agent adapter layer — `AgentAdapter` trait + Pi implementation |
| **Event Bus** | `event_bus/bus.rs` | Broadcast-based event distribution (tokio sync channels) |
| **State Machine** | `state_machine/machine.rs`, `transitions.rs` | `PetStateMachine` with 31 transition rules + debouncer |
| **Config** | `config/settings.rs` | `SettingsManager` — JSON config read/write with deep merge |
| **IPC** | `ipc/ws_server.rs` | WebSocket server for external event subscription |
| **Plugin** | `plugin/manager.rs` | Plugin lifecycle management (load, unload, query) |
| **Skin** | `skin.rs` | Skin scanning (built-in + user-custom), validation |
| **TTS** | `tts/engine.rs` | Cross-platform text-to-speech engine |
| **Window** | `window/pet_window.rs`, `tray.rs` | Pet window creation, system tray management |
| **Types** | `types/events.rs`, `pet.rs` | Rust type definitions (mirrors `packages/protocol/src/events.ts`) |

### `src/` — Frontend (React + Vite)

| File | Purpose |
|------|---------|
| `main.tsx` | React entry — mounts `<App />` in `<StrictMode>` |
| `App.tsx` | Root component — loads skin ID from settings, renders PetPNG + PetStatus |
| `hooks/useAgentState.ts` | Zustand-free hook: listens to Tauri events (`pet:state_changed`, `pet:event`) |
| `hooks/useSkinLoader.ts` | Loads skin metadata (skin.json) + PNG frame URLs from Vite glob |
| `components/PetPNG.tsx` | PNG skin renderer — loads frame based on PetState, CSS animation overlay |
| `components/PetSVG.tsx` | SVG skin renderer (legacy) — inline SVG with CSS animations |
| `components/PetStatus.tsx` | Status text overlay at bottom of window |
| `components/SkinSelector.tsx` | Skin picker — lists all skins, switches via `update_settings` IPC |
| `services/wsClient.ts` | WebSocket client — connects to `ws://127.0.0.1:8765`, auth + subscribe + heartbeat |
| `types/pet.ts` | Frontend pet types: `PetState`, `PetStateSnapshot`, `STATE_LABELS` |
| `types/events.ts` | Frontend event types (local mirror of protocol) |
| `types/skin.ts` | Frontend skin types: `SkinMetadata`, `SkinInfo`, `SkinFrames` |

### `src/assets/skins/` — Built-in PNG Skins

| Skin | Frames | Description |
|------|--------|-------------|
| `shark/` | 8 PNG files (120×120) | Cute shark character with idle, thinking, working, waiting, success, error, speaking, connecting frames |

Each skin directory must contain:
- `skin.json` — metadata (id, name, frames mapping)
- Frame PNG files — one per PetState

---

## 🧱 Core Components

### Adapter Layer

The adapter layer provides a unified interface for connecting to different AI agents.

**Trait** (`src-tauri/src/adapter/trait.rs`):
```rust
#[async_trait::async_trait]
pub trait AgentAdapter: Send + Sync {
    fn identity(&self) -> &AdapterIdentity;
    async fn connect(&self) -> Result<(), AdapterError>;
    async fn start_listening(&self) -> Result<(), AdapterError>;
    async fn stop_listening(&self) -> Result<(), AdapterError>;
    async fn send_message(&self, text: &str, session_id: &str) -> Result<String, AdapterError>;
    async fn list_sessions(&self) -> Result<Vec<Session>, AdapterError>;
    async fn health_check(&self) -> Result<AgentHealthStatus, AdapterError>;
    fn get_identity_info(&self) -> AgentIdentity;
}
```

**Current Implementation**: `PiAdapter` (`pi_adapter.rs`)
- Connects by verifying JSONL log file path
- Listens via `PiJsonlWatcher` (uses `notify` crate for cross-platform file monitoring)
- Converts Pi events using `EventConverter`
- Supports optional TTS engine integration

**Adding a New Adapter**: Implement the `AgentAdapter` trait and register in `lib.rs` setup.

### Event Bus

`EventBus` (`src-tauri/src/event_bus/bus.rs`) uses `tokio::sync::broadcast` channels for pub/sub:

- **Event Channel** (`event_tx`): Broadcasts `UnifiedAgentEvent` to all subscribers
- **State Channel** (`state_tx`): Broadcasts `(PetState, PetState)` tuples on state changes
- **Channel size**: Configurable (default 4096), old events dropped when full
- **Clone-safe**: `EventBus` is `Clone` (clones the internal senders)

### State Machine

`PetStateMachine` (`src-tauri/src/state_machine/machine.rs`) is the core decision engine:

- **Initial state**: `Connecting`
- **Transition table**: 31 rules defined in `transitions.rs`
- **Debouncer**: 500ms minimum state hold time prevents flickering
- **Callbacks**: `on_state_change()` for custom reactions
- **Global singleton**: Shared via `lazy_static!` Arc<TokioMutex<PetStateMachine>>

### Skin/Plugin System

**Skin System**:
- Built-in skins: bundled at compile time via `include_dir!`
- User skins: `~/.config/agent-pet-hub/skins/<skin-id>/`
- Skin format: `skin.json` + frame PNG files (120×120 recommended)

**Plugin System**:
- `Plugin` trait defines interface: `id()`, `name()`, `version()`, `plugin_type()`, `init()`, etc.
- Plugin types: `skin`, `animation`, `voice`, `notification`
- Thread-safe: `Send + Sync`
- Managed by `PluginManager` with load/unload/lifecycle

### IPC Layer

**Tauri Commands** (frontend → backend):

| Command | Parameters | Returns | Purpose |
|---------|-----------|---------|---------|
| `get_pet_state` | — | `PetState` | Get current pet state |
| `get_previous_state` | — | `PetState` | Get previous state |
| `get_state_snapshot` | — | `serde_json::Value` | Full state snapshot |
| `get_settings` | — | `serde_json::Value` | Get all settings |
| `update_settings` | `updates: Value` | `()` | Update settings (deep merge) |
| `send_event` | `event: UnifiedAgentEvent` | `usize` | Publish event to bus |
| `set_pet_state` | `state: PetState` | `()` | Force-set state |
| `toggle_pet_window` | — | `()` | Show/hide pet window |
| `send_heartbeat` | — | `()` | Publish heartbeat event |
| `get_agent_info` | — | `AgentIdentity[]` | Get all agent info |
| `start_drag` | — | `()` | Trigger window drag |
| `list_skins` | — | `SkinInfo[]` | List all available skins |
| `get_skin_metadata` | `skinId: string` | `Value` | Get skin metadata |

**Tauri Events** (backend → frontend):

| Event | Payload | Purpose |
|-------|---------|---------|
| `pet:state_changed` | `PetState` | Pet state changed |
| `pet:event` | `UnifiedAgentEvent` (raw stripped) | Agent event received |

**WebSocket Server** (external → app):
- URL: `ws://127.0.0.1:8765`
- Auth: Bearer token (configured in settings, default: random ULID)
- Supports: subscribe, unsubscribe, command, agent_info

---

## 📜 Protocol

### Event Types

The unified event protocol defines **22 event types** across **9 categories**:

| Category | Event Types | Target PetState |
|----------|-------------|-----------------|
| **Session** | `session_start`, `session_end`, `session_compaction` | Thinking → Idle → Thinking |
| **Thinking** | `thinking_start`, `thinking_tick`, `thinking_end` | Thinking |
| **Tool** | `tool_call_start`, `tool_call_end`, `tool_call_error`, `tool_batch` | Working → Thinking → Error |
| **Message** | `agent_message`, `agent_reply` | Thinking |
| **Permission** | `permission_request`, `permission_granted`, `permission_denied` | Waiting → Thinking |
| **User** | `user_prompt`, `user_cancel` | Thinking → Idle |
| **Subagent** | `subagent_start`, `subagent_end` | Working → Thinking |
| **System** | `heartbeat`, `adapter_connected`, `adapter_disconnected` | Idle |
| **Error** | (embedded in tool_call_error, etc.) | Error |

### Pet States

| State | Animation | Description |
|-------|-----------|-------------|
| `idle` | Breathing, idle blinking | Awaiting events |
| `thinking` | Head tilt, blinking, blush | Processing agent reasoning |
| `working` | Fast shaking, green ring | Tool execution in progress |
| `waiting` | Slow rocking, watch-checking | Waiting for user approval |
| `success` | Celebrating bounce, star | Operation completed |
| `error` | Head shake, red X | Error occurred |
| `speaking` | Speaking animation | TTS voice播报 |
| `connecting` | Rotating spinner | Initializing connection |

### State Machine Transitions

```
Connecting ──[adapter_connected]──▶ Idle ◀──[session_end / user_cancel]──┐
     │                                                                     │
     └──[session_start / user_prompt]──▶ Thinking ──[tool_call_start]──┐  │
          │                              │                               │  │
          │                        ┌──────▼──────┐                       │  │
          │                        │   Working   │◀─────────────────────┘ │
          │                        └──────┬──────┘                       │ │
          │                               │                               │ │
          │                         [tool_call_end]                      │ │
          │                               ▼                              │ │
          │                          [thinking_end]                      │ │
          │                               │                              │ │
          │                     [permission_request]                     │ │
          │                               ▼                              │ │
          │                          [waiting] ──[permission_granted]───▶ │
          │                               │                                │
          │                     [session_end / user_cancel]                │
          │                               ▼                                │
          └───────────────────────────── Idle ←────────────────────────────┘

Error ──[session_start / user_prompt]──▶ Thinking (recover)
  │
  └──[session_end / user_cancel]──▶ Idle

Any State ──[adapter_disconnected]──▶ Idle
```

Full transition table (31 rules) in `src-tauri/src/state_machine/transitions.rs`.

---

## ⚙️ Configuration

### Configuration File Location

| Platform | Path |
|----------|------|
| Linux | `~/.config/agent-pet-hub/config.json` |
| macOS | `~/Library/Application Support/agent-pet-hub/config.json` |
| Windows | `%APPDATA%\agent-pet-hub\config.json` |

### Configuration Schema

```json
{
  "pet": {
    "skinId": "shark",
    "enabled": true,
    "showStatus": true
  },
  "adapter": {
    "pi": {
      "enabled": true,
      "logPath": "~/.pi/agent/logs/latest.jsonl"
    },
    "hermes": {
      "enabled": false,
      "gatewayUrl": "ws://localhost:9100"
    },
    "openclaw": {
      "enabled": false,
      "url": "http://localhost:3100"
    }
  },
  "websocket": {
    "enabled": true,
    "port": 8765,
    "authToken": "01HQXYZ..."
  },
  "tts": {
    "enabled": true,
    "volume": 1.0,
    "language": "zh-cn",
    "rules": {
      "session_start": true,
      "tool_call": true,
      "tool_error": true,
      "permission_request": true,
      "session_end": true,
      "agent_message": false,
      "minIntervalMs": 3000,
      "focusMode": false
    }
  },
  "window": {
    "width": 320,
    "height": 280,
    "alwaysOnTop": true
  }
}
```

### Runtime Configuration Updates

Settings can be updated at runtime via Tauri command or WebSocket:

```typescript
// Via Tauri invoke
import { invoke } from "@tauri-apps/api/core";
await invoke("update_settings", {
  updates: { tts: { volume: 0.5 } }
});
```

### Skin Directory

User skins go to: `~/.config/agent-pet-hub/skins/<skin-id>/skin.json`

Each skin must have:
```json
{
  "id": "my-skin",
  "name": "My Skin",
  "description": "A custom skin",
  "frames": {
    "idle": "idle.png",
    "thinking": "thinking.png",
    "working": "working.png",
    "waiting": "waiting.png",
    "success": "success.png",
    "error": "error.png",
    "speaking": "speaking.png",
    "connecting": "connecting.png"
  }
}
```

---

## 🛠 Development

### Development Workflow

```bash
# 1. Start the development server (Vite + Tauri)
pnpm tauri dev

# This concurrently runs:
#   - Vite dev server on http://localhost:1420
#   - Rust Tauri backend connecting to the Vite server
#   - Hot module replacement for frontend changes

# 2. Build frontend only (for CI or preview)
pnpm build

# 3. Preview production build
pnpm preview

# 4. Lint
pnpm lint

# 5. Run tests
pnpm test
```

### Code Structure Notes

- **Frontend**: React 19 + TypeScript strict mode. No Redux — uses Tauri events + React state.
- **Backend**: Rust with `tokio` async runtime. No `async_trait` in new code (use native async impl).
- **Protocol**: Shared via `packages/protocol` monorepo workspace. Both Rust and TS side share type definitions.
- **Styling**: CSS `@keyframes` for all pet animations. No CSS-in-JS.
- **Window**: Transparent, always-on-top, no decorations, fixed 320×280.

### Adding a New Skin

1. Create directory: `src/assets/skins/<name>/`
2. Add `skin.json` with metadata and frame mappings
3. Add 8 PNG files (120×120 recommended): `idle.png`, `thinking.png`, `working.png`, `waiting.png`, `success.png`, `error.png`, `speaking.png`, `connecting.png`
4. Skin auto-detected on next launch — no code changes needed

### Adding a New Agent Adapter

1. Create module under `src-tauri/src/adapter/`
2. Implement the `AgentAdapter` trait
3. Register in `lib.rs` setup block
4. Add config fields to `config/settings.rs`
5. Re-export in `adapter/mod.rs`

### Windows Development

On Windows, install [Tauri's prerequisites](https://tauri.app/v1/api/start/prerequisites):

```powershell
winget install --id Rustlang.Rustup
winget install --id Kitware.CMake
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --add Microsoft.VisualStudio.Workload.VCTools"
```

---

## 🔍 Comparison with Similar Projects

| Feature | Agent Pet Hub | Desktop Pet (generic) | Agent Dashboard |
|---------|--------------|----------------------|-----------------|
| Animated pet on desktop | ✅ | ✅ | ❌ |
| Multi-agent support | ✅ (3 agents) | ❌ | ❌ |
| Unified event protocol | ✅ | ❌ | ❌ |
| State machine with debounce | ✅ | ❌ | ❌ |
| TTS voice broadcast | ✅ | ❌ | ❌ |
| WebSocket event streaming | ✅ | ❌ | ❌ |
| Customizable skins | ✅ | ❌ | ❌ |
| Plugin system | ✅ | ❌ | ❌ |
| System tray integration | ✅ | ❌ | ❌ |

---

## 📄 License

MIT License — see [LICENSE](./LICENSE) file for details.

---

*Built with ❤️ for developers who want their AI agents to have a cute companion on screen.*
