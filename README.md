# Agent Pet Hub 🐾

> Unified Desktop Pet Hub for AI Agents — A Tauri desktop app that visualizes AI Agent runtime events into animated pets on your screen.

[![TypeScript](https://img.shields.io/badge/TypeScript-5.6%2B-blue)](https://www.typescriptlang.org/)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/React-19-61DAFB)](https://react.dev/)
[![Tauri](https://img.shields.io/badge/Tauri-2.x-22C55E)](https://tauri.app/)
[![License](https://img.shields.io/badge/License-MIT-green)](./LICENSE)

Agent Pet Hub is a cross-platform desktop pet application built on **Tauri 2.x**. It monitors AI Agent runtime events (Pi, Hermes, OpenClaw) in real time and renders animated pet responses on screen. Built with a clean layered architecture featuring **Adapter pattern**, **Event Bus**, **State Machine**, and a **plugin-based skin system**.

---

## 📑 Table of Contents

- [Features](#features)
- [Architecture](#architecture)
  - [System Architecture](#system-architecture)
  - [Data Flow](#data-flow)
  - [Module Structure](#module-structure)
- [Dependencies](#dependencies)
- [Quick Start](#quick-start)
- [Project Structure](#project-structure)
- [Core Components](#core-components)
  - [Adapter Layer](#adapter-layer)
  - [Event Bus](#event-bus)
  - [State Machine](#state-machine)
  - [Skin/Plugin System](#skinplugin-system)
  - [IPC Layer](#ipc-layer)
- [Protocol](#protocol)
  - [Event Types](#event-types)
  - [State Machine Transitions](#state-machine-transitions)
- [Configuration](#configuration)
- [Development](#development)
- [Comparison with Similar Projects](#comparison-with-similar-projects)
- [License](#license)

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
│       │                             └──────┬────────────────┘                │
│       │                                    │ emit("pet:state_changed")       │
│       │                             ┌──────▼──────────┐                      │
│       │                             │  Tauri Events   │                      │
│       │                             │  + Commands     │                      │
│       │                             └──────┬──────────┘                      │
│       │                                    │ emit("pet:event")               │
│  ┌─────▼────────┐     ┌──────────────┐     ┌─────────────────────────────┐   │
│  │  WebSocket   │     │  TTS Engine  │     │  Plugin Manager             │   │
│  │  Server      │     │              │     │  (skin/animation/notify)    │   │
│  │  (port 8765) │     └──────────────┘     └─────────────────────────────┘   │
│  └──────────────┘                                                            │
└──────────────────────────────────────────────────────────────────────────────┘
        │ Tauri IPC (Events + Commands)
        ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                      FRONTEND (React 19 + TypeScript)                        │
│                                                                              │
│  ┌──────────────┐    ┌──────────────────────────────────────────────────┐    │
│  │  App.tsx     │    │  Components                                      │    │
│  │              │    │  ├── PetPNG.tsx  — PNG skin frame rendering      │    │
│  │  └───┬──────┘     │  ├── PetSVG.tsx   — SVG rendering (fallback)     │    │
│  │      │            │  ├── PetStatus.tsx — Status text overlay         │    │
│  │      │            │  └── SkinSelector.tsx — Skin switching UI        │    │
│  │  useAgentState  │   └──────┬─────────────────────────────────────────┘    │
│  │  (Tauri listen) │          │                                              │
│  └─────────────────┘    ┌─────▼────────────┐                                 │
│                         │  useSkinLoader    │                                │
│                         │  (Vite glob + IPC)│                                │
│                         └───────────────────┘                                │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  WebSocketClient (optional)                                         │     │
│  │  connect → auth → subscribe(eventTypes) → onEvent callback          │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
  ┌──────────────────────────────────────────────────────────────────┐
  │  Path 1: Agent → Pet Animation (Primary Flow)                    │
  └──────────────────────────────────────────────────────────────────┘

  Agent (Pi/Hermes/OpenClaw)
    │
    │  [Pi: JSONL file]  or  [Hermes/OpenClaw: HTTP API]
    ▼
  AgentAdapter (Trait)
    │  PiJsonlWatcher polls ~/.pi/agent/logs/latest.jsonl every 500ms
    ▼
  EventConverter
    │  Maps raw agent events → UnifiedAgentEvent
    │  Pi: session_start → Thinking, tool_call → Working, ...
    ▼
  EventBus.publish_event(unified_event)
    │  tokio::sync::broadcast channel
    ├──▶ PetStateMachine.handle_event()
    │     │  lookup: (current_state, event_type) → new_state
    │     │  debounce: min 500ms state hold time
    │     ▼
    │   app_handle.emit("pet:state_changed", PetState)
    │   app_handle.emit("pet:event", UnifiedAgentEvent)
    │
    ▼
  Frontend: Tauri listen("pet:state_changed")
    │
    ▼
  useAgentState hook → setPetState(newState)
    │
    ▼
  PetPNG component → frames[petState] → <img> + CSS animation
    │
    ▼
  PetStatus → STATE_LABELS[newState] → Chinese status text

  ┌──────────────────────────────────────────────────────────────────┐
  │  Path 2: WebSocket Push (Secondary Flow)                         │
  └──────────────────────────────────────────────────────────────────┘

  WebSocketClient (connect → auth → subscribe)
    │  ws://127.0.0.1:8765
    ▼
  WSServer (tokio-tungstenite)
    │  auth → subscribe([eventTypes]) → heartbeat (30s)
    ▼
  event_rx.recv() → JSON.stringify → ws.send()
    │
    ▼
  Frontend WebSocketClient.onEvent → callback
```

### Module Structure

```
agent-pet-hub/                          # Monorepo root
├── packages/protocol/                  # 📦 Shared protocol package
│   └── src/
│       ├── events.ts                   #   Type definitions (EventType, PetState, etc.)
│       ├── schemas.ts                  #   Zod runtime validation schemas
│       ├── validators.ts               #   Result<T,E> + validateEvent()
│       ├── mapping.ts                  #   Agent→Event type mapping tables
│       └── index.ts                    #   Unified re-export
│
├── src/                                # 🌐 React Frontend
│   ├── main.tsx                        #   Entry point (React StrictMode)
│   ├── App.tsx                         #   Root component (skin loading + drag)
│   ├── index.css                       #   8-state CSS animations + overlay
│   ├── components/                     #   UI components
│   │   ├── PetPNG.tsx                  #   PNG skin frame renderer
│   │   ├── PetSVG.tsx                  #   SVG skin renderer (fallback)
│   │   ├── PetSVGLegacy.tsx            #   Deprecated SVG component
│   │   ├── PetStatus.tsx               #   Status text overlay
│   │   └── SkinSelector.tsx            #   Skin switching UI
│   ├── hooks/                          #   React hooks
│   │   ├── useAgentState.ts            #   Tauri event listener → pet state
│   │   └── useSkinLoader.ts            #   Skin discovery + frame URL resolution
│   ├── services/                       #   Network services
│   │   └── wsClient.ts                 #   WebSocket client (auth+subscribe+heartbeat)
│   └── types/                          #   Frontend type definitions
│       ├── events.ts                   #   Event type mirror (protocol-aligned)
│       ├── pet.ts                      #   PetPosition, PetStateSnapshot, labels
│       └── skin.ts                     #   SkinMetadata, SkinFrames, SkinInfo
│
├── src-tauri/                          # 🦀 Rust Backend (Tauri 2.x)
│   ├── src/
│   │   ├── lib.rs                      #   App initialization + module wiring
│   │   ├── main.rs                     #   Binary entry → lib::run()
│   │   ├── commands.rs                 #   15 Tauri commands (get/set/list)
│   │   ├── skin.rs                     #   Skin discovery (builtin + user dirs)
│   │   ├── adapter/                    #   Agent adapter pattern
│   │   │   ├── trait.rs                #     AgentAdapter trait definition
│   │   │   ├── pi_adapter.rs           #     Pi agent integration
│   │   │   ├── pi_watcher.rs           #     JSONL file watcher (500ms poll)
│   │   │   └── event_converter.rs      #     Raw → UnifiedAgentEvent converter
│   │   ├── event_bus/
│   │   │   └── bus.rs                  #   tokio broadcast channel
│   │   ├── state_machine/
│   │   │   ├── machine.rs              #   StateMachine + StateDebouncer (500ms)
│   │   │   └── transitions.rs          #   31 state transition rules + tests
│   │   ├── ipc/
│   │   │   └── ws_server.rs            #   WebSocket server (auth+subscribe+push)
│   │   ├── tts/                        #   Text-to-speech engine
│   │   ├── config/                     #   SettingsManager (JSON config)
│   │   ├── window/                     #   Pet window + system tray
│   │   ├── plugin/                     #   Plugin manager (skin/animation/notify)
│   │   └── types/
│   │       └── pet.rs                  #   Rust PetState, PetConfig, PetMood
│   ├── tauri.conf.json                 #   Window config (320x280, transparent, alwaysOnTop)
│   └── Cargo.toml
│
├── src/assets/skins/                   # 🎨 Built-in skins
│   ├── default/skin.json + *.png       #   Default skin (8 frames)
│   └── shark/skin.json + *.png         #   Shark skin (7 frames)
│
├── skills/pi/                          # 🔌 Pi Agent Extension
│   └── pet-event-logger.ts             #   Extension: logs events to JSONL
│
├── docs/                               # 📝 Design documents
│   └── phase*.md
│
├── package.json                        #   Scripts, dependencies, workspace config
├── tsconfig.json                       #   TypeScript config (ES2022, strict)
├── vite.config.ts                      #   Vite config (dev port 1420)
└── pnpm-workspace.yaml                 #   pnpm workspace (packages/*)
```

---

## 📦 Dependencies

### Runtime Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| `react` | ^19.0.0 | UI framework |
| `react-dom` | ^19.0.0 | DOM renderer |
| `@tauri-apps/api` | ^2 | Tauri IPC (invoke/emit/listen) |
| `@tauri-apps/plugin-shell` | ^2 | Shell command execution (TTS) |
| `zustand` | ^5.0.0 | State management (optional) |
| `zod` | 3.23.x | Runtime validation (protocol package) |

### Development Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| `typescript` | ^5.6.0 | Type checking |
| `vite` | ^6.0.0 | Build tool + dev server |
| `vitest` | ^2.1.0 | Unit testing |
| `eslint` + `@typescript-eslint/*` | ^8.0.0 | Linting |
| `@vitejs/plugin-react` | ^4.3.0 | React Fast Refresh |

### Backend Dependencies (Rust)

| Crate | Purpose |
|-------|---------|
| `tauri` | Desktop framework |
| `tokio` | Async runtime |
| `serde` / `serde_json` | Serialization |
| `lazy_static` | Global state singletons |
| `tracing` | Structured logging |
| `lazy-regex` | JSONL line parsing |
| `tungstenite` | WebSocket server |
| `include_dir` | Embedded skin assets |
| `rodio` / `cpal` | Audio playback (TTS) |

### System Requirements

- **Node.js** 18+ / **pnpm** 8+
- **Rust** 1.75+ with `cargo-tauri`
- **Tauri CLI**: `cargo install tauri-cli --version "^2"`

---

## 🚀 Quick Start

### 1. Clone & Install

```bash
git clone https://github.com/your-org/agent-pet-hub.git
cd agent-pet-hub
pnpm install
```

### 2. Development Mode

```bash
# Start Tauri dev (Rust + Vite dev server on :1420)
pnpm tauri dev
```

This starts:
- Vite dev server on `http://localhost:1420`
- Tauri window (320×280, transparent, always-on-top)
- Pi adapter monitoring `~/.pi/agent/logs/latest.jsonl`
- WebSocket server on port `8765`

### 3. Build

```bash
# TypeScript check + production build
pnpm build

# Tauri packaged binary
pnpm tauri build
```

### 4. Run Tests

```bash
# Rust tests (state machine transition rules)
cd src-tauri && cargo test --lib
```

> Note: this workspace currently does not include frontend Vitest test files, so `pnpm test` may not find any tests.

---

## 🏗️ Core Components

### Adapter Layer

The Adapter pattern enables **zero-code integration** of new AI agents. Each agent implements the `AgentAdapter` trait:

```rust
// src-tauri/src/adapter/trait.rs
pub trait AgentAdapter: Send + Sync {
    fn identity(&self) -> AdapterIdentity;
    fn connect(&mut self) -> Result<(), AdapterError>;
    fn start_listening(&mut self, event_bus: EventBusSender);
    fn send_message(&self, message: &str) -> Result<(), AdapterError>;
    fn health_check(&self) -> bool;
}
```

| Adapter | Status | Connection Method |
|---------|--------|-------------------|
| **Pi** | ✅ Implemented | JSONL file watcher (500ms poll) |
| **Hermes** | 🔄 Planned | HTTP gateway polling |
| **OpenClaw** | 🔄 Planned | HTTP API polling |

Adding a new adapter requires:
1. Implement `AgentAdapter` trait
2. Register in `lib.rs` under `init_adapters()`
3. Enable in config

### Event Bus

The EventBus is a **tokio broadcast channel** that decouples event producers from consumers:

```rust
// src-tauri/src/event_bus/bus.rs
pub struct EventBus {
    event_tx: broadcast::Sender<UnifiedAgentEvent>,
    state_tx: broadcast::Sender<PetState>,
}
```

**Flow:**
```
AgentAdapter → EventConverter → EventBus.publish_event()
                                              ├──▶ PetStateMachine
                                              ├──▶ WebSocket Server
                                              └──▶ Frontend (Tauri Events)
```

**Benefits:**
- Zero-copy event distribution
- Any number of subscribers (state machine, WS server, plugins)
- Backpressure via broadcast channel capacity

### State Machine

A deterministic state machine with **31 transition rules** and **500ms debounce**:

```rust
// src-tauri/src/state_machine/machine.rs
pub struct PetStateMachine {
    current_state: PetState,
    previous_state: Option<PetState>,
    state_history: Vec<PetState>,
    min_state_duration: Duration,   // 500ms debounce
    callbacks: Vec<Box<dyn Fn(PetState) + Send + Sync>>,
}
```

**8 Pet States:**
| State | Chinese Label | CSS Class | Description |
|-------|--------------|-----------|-------------|
| `Idle` | 空闲 | `pet-idle` | Breathing, idle animation |
| `Thinking` | 思考中 | `pet-thinking` | Tilted head, blinking |
| `Working` | 工作中 | `pet-working` | Coding, busy |
| `Waiting` | 等待中 | `pet-waiting` | Looking at watch, waiting for permission |
| `Success` | 成功 | `pet-success` | Celebration animation |
| `Error` | 出错 | `pet-error` | Shaking head, error indication |
| `Speaking` | 语音播报 | `pet-speaking` | TTS playback |
| `Connecting` | 连接中 | `pet-connecting` | Loading/connecting |

**Key transition rules (31 total):**
```
Connecting + AdapterConnected → Idle
Connecting + SessionStart → Thinking
Connecting + UserPrompt → Thinking
Idle + SessionStart → Thinking
Idle + UserPrompt → Thinking
Thinking + ToolCallStart → Working
Thinking + ToolBatch → Working
Thinking + SubagentStart → Working
Thinking + SessionEnd → Idle
Thinking + UserCancel → Idle
Thinking + ToolCallError → Error
Thinking + PermissionRequest → Waiting
Working + ToolCallEnd → Thinking
Working + ThinkingEnd → Thinking
Working + SessionEnd → Idle
Working + UserCancel → Idle
Working + ToolCallError → Error
Working + PermissionRequest → Waiting
Waiting + PermissionGranted → Thinking
Waiting + PermissionDenied → Thinking
Waiting + SessionEnd → Idle
Waiting + UserCancel → Idle
Error + SessionEnd → Idle
Error + UserCancel → Idle
Error + SessionStart → Thinking
Error + UserPrompt → Thinking
AnyState + AdapterDisconnected → Idle
```

### Skin/Plugin System

**Skin Discovery (Rust side):**
```rust
// src-tauri/src/skin.rs — scan_all_skins()
1. scan_builtin_png_skins()      // include_dir!("../src/assets/skins")
2. scan_builtin_resource_skins() // include_dir!("resources/skins") — legacy
3. scan_directory_skins(user_dir) // ~/.config/agent-pet-hub/skins/
```

**Skin Loading (Frontend):**
```typescript
// src/hooks/useSkinLoader.ts
1. invoke("list_skins") → SkinInfo[]
2. invoke("get_skin_metadata", skinId) → SkinMetadata
3. Built-in: Vite glob import.meta.glob("*.png") → hash URLs
4. Custom: Direct file path resolution
5. Parse frames → { [PetState]: string (URL) }
```

**Skin Package Structure:**
```
skins/
  {skin-id}/
    skin.json              // { id, name, version, frames: { idle: "idle.png", ... } }
    idle.png               // Frame images (120×120 recommended)
    thinking.png
    working.png
    ...
```

**Falls Back:** `skin.json` supports frame fallbacks for missing states (e.g., shark skin maps `speaking` → `idle.png`).

### IPC Layer

**Tauri IPC (Frontend ↔ Rust):**
| Command | Direction | Description |
|---------|-----------|-------------|
| `get_pet_state` | Rust → Frontend | Current PetState + timestamp |
| `get_previous_state` | Rust → Frontend | Previous PetState |
| `get_settings` | Rust → Frontend | Full config snapshot |
| `update_settings` | Frontend → Rust | Update skinId, TTS, window settings |
| `send_event` | Frontend → Rust | Manually trigger event |
| `set_pet_state` | Frontend → Rust | Force state change (debug) |
| `toggle_pet_window` | Frontend → Rust | Show/hide pet window |
| `start_drag` | Frontend → Rust | Begin window drag |
| `list_skins` | Rust → Frontend | Discover all available skins |
| `get_skin_metadata` | Rust → Frontend | Get skin.json metadata |

**WebSocket IPC (External Process ↔ Rust):**
```
External Process          WSServer (port 8765)
    │                          │
    ├── CONNECT ─────────────▶│
    ├── AUTH(token) ────────▶ │  Verify token
    ├── SUBSCRIBE(events) ──▶ │  Register event interest
    │                          │
    │◀── EVENT(JSON) ────────┤  Push on event bus
    │◀── HEARTBEAT ──────────┤  Ping/pong every 30s
    │                         │
    ├── DISCONNECT ─────────▶│  Cleanup
```

---

## 📜 Protocol

The protocol is published as `@agent-pub/protocol` (workspace package) for type sharing between Rust and TypeScript.

### Event Types

```typescript
type EventType =
  // Session (3)
  | "session_start" | "session_end" | "session_compaction"
  // User (2)
  | "user_prompt" | "user_cancel"
  // Thinking (3)
  | "thinking_start" | "thinking_tick" | "thinking_end"
  // Tool (4)
  | "tool_call_start" | "tool_call_end" | "tool_call_error" | "tool_batch"
  // Permission (3)
  | "permission_request" | "permission_granted" | "permission_denied"
  // Message (2)
  | "agent_message" | "agent_reply"
  // Subagent (2)
  | "subagent_start" | "subagent_end"
  // System (3)
  | "heartbeat" | "adapter_connected" | "adapter_disconnected"
  // Total: 22 event types
```

### Event Schema

```typescript
interface UnifiedAgentEvent {
  id: string;              // ULID format (26 chars)
  timestamp: string;       // ISO 8601
  version: "1.0";
  source: AgentSource;     // "pi" | "hermes" | "openclaw"
  category: EventCategory; // "session" | "user" | "thinking" | "tool" | ...
  type: EventType;         // See above
  petState: PetState;      // 8 pet states
  sessionId?: string;
  toolName?: string;
  toolArgsPreview?: string;
  taskPreview?: string;
  raw: Record<string, unknown>;
  metadata?: Record<string, unknown>;
}
```

### Agent Event Mapping

Each agent's raw events are mapped via `packages/protocol/src/mapping.ts`:

```typescript
// Pi Agent mapping example
const piEventMapping: Record<string, UnifiedEventType> = {
  "session_start": "session_start",
  "user_prompt": "user_prompt",
  "text_delta": "thinking_tick",
  "tool_call": "tool_call_start",
  "tool_result": "tool_call_end",
  "tool_error": "tool_call_error",
  "turn_end": "session_end",
  "compaction": "session_compaction",
};
```

---

## ⚙️ Configuration

Config path: `~/.config/agent-pet-hub/config.json`

```json
{
  "pet": {
    "skinId": "default",
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
      "gatewayUrl": ""
    },
    "openclaw": {
      "enabled": false,
      "url": ""
    }
  },
  "websocket": {
    "enabled": true,
    "port": 8765,
    "authToken": "agent-pet-hub-default"
  },
  "tts": {
    "enabled": false,
    "volume": 0.8
  },
  "window": {
    "width": 320,
    "height": 280,
    "alwaysOnTop": true,
    "skipTaskbar": true
  }
}
```

---

## 🛠️ Development

### Adding a New Agent Adapter

1. Create `src-tauri/src/adapter/{name}_adapter.rs`
2. Implement `AgentAdapter` trait
3. Add module to `src-tauri/src/adapter/mod.rs`
4. Register in `lib.rs:42` (init_adapters)
5. Add config in `config/adapter/{name}.json`
6. Add mapping in `packages/protocol/src/mapping.ts`

### Adding a New Skin

1. Create directory `src/assets/skins/{your-skin}/`
2. Add `skin.json` with frame definitions
3. Add frame PNG/SVG images (120×120 recommended)
4. Skin auto-discovers on next restart

### Directory Structure Reference

| Directory | Relative Path | Purpose |
|-----------|--------------|---------|
| Frontend entry | `src/main.tsx` | React bootstrap |
| Root component | `src/App.tsx` | Skin loading + window drag |
| CSS animations | `src/index.css` | 8 state animations + transitions |
| WS client | `src/services/wsClient.ts` | WebSocket connection management |
| State hook | `src/hooks/useAgentState.ts` | Tauri event → React state |
| Skin hook | `src/hooks/useSkinLoader.ts` | Skin discovery + frame resolution |
| Protocol types | `packages/protocol/src/events.ts` | EventType, PetState, AgentSource |
| Protocol schemas | `packages/protocol/src/schemas.ts` | Zod validation |
| Protocol mapping | `packages/protocol/src/mapping.ts` | Agent→Event type maps |
| Rust lib entry | `src-tauri/src/lib.rs` | App init + module wiring |
| Rust commands | `src-tauri/src/commands.rs` | Tauri command handlers |
| State machine | `src-tauri/src/state_machine/` | Rules + debounce logic |
| Agent adapters | `src-tauri/src/adapter/` | Adapter trait + implementations |
| Event bus | `src-tauri/src/event_bus/` | Broadcast channel |
| WS server | `src-tauri/src/ipc/ws_server.rs` | WebSocket server |
| Skin scanner | `src-tauri/src/skin.rs` | Built-in + user skin discovery |
| Pi extension | `skills/pi/pet-event-logger.ts` | Pi Agent extension |

---

## 🆚 Comparison with Similar Projects

| Feature | **Agent Pet Hub** | macOS Desktop Pet | Linux Desktop Pet (Plasma) | OpenPet |
|---------|-------------------|-------------------|---------------------------|---------|
| **AI Agent Integration** | ✅ Native adapter pattern | ❌ Manual trigger | ⚠️ Limited | ❌ |
| **Multi-Agent Support** | ✅ 3+ agents, extensible | ❌ Single | ⚠️ Single | ⚠️ Limited |
| **Event Protocol** | ✅ Unified schema (Zod) | ❌ Custom | ❌ Custom | ⚠️ JSON |
| **Cross-Platform** | ✅ Windows/Mac/Linux (Tauri) | ✅ macOS only | ✅ Linux only | ✅ |
| **Skin System** | ✅ PNG/SVG + user skins | ⚠️ Images only | ⚠️ Themes | ⚠️ Images |
| **WebSocket IPC** | ✅ Real-time push | ❌ | ❌ | ❌ |
| **State Debounce** | ✅ 500ms anti-flicker | ❌ | ⚠️ Basic | ❌ |
| **TTS** | ✅ Cross-platform | ⚠️ macOS only | ⚠️ Varies | ❌ |
| **Plugin System** | ✅ Skin/animation/notify | ❌ | ⚠️ Widgets | ❌ |
| **Build System** | ✅ pnpm workspace | npm | cmake | cargo |
| **Type Safety** | ✅ TypeScript + Rust | JS | JS/C++ | Rust |

**Agent Pet Hub Advantages:**

1. **Unified Protocol** — Every agent's events normalize to the same schema. Add a new agent in ~100 lines of code without touching the frontend.

2. **Adapter Pattern** — The `AgentAdapter` trait is the single point of extension. The core (state machine, skin rendering, WS server) never changes when adding agents.

3. **State Debounce** — 500ms minimum state retention prevents rapid state flickering during burst events (e.g., rapid tool calls).

4. **Dual IPC** — Tauri Events for frontend communication + WebSocket for external process integration. Most competitors only offer one.

5. **Monorepo Protocol** — `@agent-pet-hub/protocol` package shares types between Rust and TypeScript via workspace packages. Zero duplication, full type safety.

6. **User Skin Directory** — Place skins in `~/.config/agent-pet-hub/skins/` and they auto-discover. No rebuild needed.

---

## 📄 License

MIT

## 🤝 Contributing

Issues and PRs welcome! Please read the design docs in `docs/` before contributing.

---

*Agent Pet Hub — Your AI Agent deserves a companion 🐾*
