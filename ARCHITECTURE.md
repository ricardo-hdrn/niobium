# Niobium Architecture

## What Is Niobium

Niobium is a **universal rich UI runtime for CLI AI agents**, exposed via MCP. CLI agents (Claude Code, Codex, Gemini CLI, custom agents) have no UI — they're stuck in the terminal. Niobium gives them one.

Beyond UI, Niobium is an **environment-aware runtime** — it can capture and stream environmental inputs (voice, clipboard, files, sensors) as events, making agents reactive to the world around them.

## The Problem

```
CLI Agent today:
  "I need the user to pick a date range"
  → prints: "Please enter start date (YYYY-MM-DD):"
  → user types into terminal
  → error-prone, ugly, limited

With Niobium:
  → MCP call: show_date_picker(min: "2026-01-01", max: "2026-12-31")
  → Native date picker appears on screen
  → User picks dates with full rich interaction
  → Returns: { start: "2026-03-01", end: "2026-06-15" }
```

## Core Architecture

```
┌─────────────┐    stdio/SSE     ┌──────────────────────────────────────────────┐
│  CLI Agent   │ ◄────(1)──────► │  Niobium Runtime                             │
│  (any)       │    JSON-RPC     │                                              │
│              │                  │  ┌─────────────────────────────────────────┐ │
│  Claude Code │                  │  │          Event Bus (Rust)               │ │
│  Codex       │                  │  │  async, non-blocking, pub/sub          │ │
│  Gemini CLI  │                  │  └──▲──────────▲──────────▲───────────▲──┘ │
│  Custom      │                  │     │          │          │           │     │
│              │                  │  ┌──┴───┐  ┌──┴───┐  ┌──┴────┐  ┌──┴──┐  │
│              │                  │  │ MCP  │  │ UI   │  │ Env   │  │ ... │  │
│              │                  │  │Server│  │Engine│  │Sources│  │     │  │
│              │                  │  │(Rust)│  │(Dart)│  │(Rust) │  │     │  │
│              │                  │  └──────┘  └──────┘  └───────┘  └─────┘  │
│              │                  │                                           │
└─────────────┘                  └───────────────────────────────────────────┘
```

## Pluggable Architecture

Niobium core is a **thin runtime** — an event bus with a plugin system. All capabilities are plugins:

- **MCP tools** (UI commands like `show_form`, `pick_file`) are plugins
- **UI widgets** (chart types, custom inputs) are plugins
- **Environment sources** (voice capture, clipboard, file watcher) are plugins
- **Interactions** (voice input, gesture recognition) are plugins

### Plugin Contract

Plugins register with the core via a standard trait/interface:

- **Rust plugins**: implement a trait — declare MCP tools, subscribe to events, emit events
- **Dart UI plugins**: register widget builders for custom UI types
- **WASM plugins**: sandboxed third-party plugins, same contract, safe execution

### Plugin Loading

| Type | Use Case | Safety |
|------|----------|--------|
| Native (dynamic lib) | First-party / trusted plugins | Full access |
| WASM | Third-party / community plugins | Sandboxed |
| Dart package | UI-only plugins (new widget types) | Flutter sandbox |

### Plugin Installation

Plugins are npm packages. Niobium delegates to npm for download/versioning and handles the binding:

```
niobium plugin install <name>
  → npm install -g <name>
  → reads plugin manifest (package.json niobium entry)
  → registers in Niobium plugin config
  → ready to use

niobium plugin list
niobium plugin remove <name>
  → npm uninstall -g <name>
  → removes from Niobium plugin config
```

No custom registry, no custom package format. Plugins are npm packages with a `niobium` field in `package.json`:

```json
{
  "name": "niobium-voice-input",
  "niobium": {
    "type": "environment",
    "tools": ["voice_record", "voice_transcribe"],
    "events": ["VoiceActivity", "VoiceTranscript"],
    "rust_wasm": "voice_input.wasm",
    "dart_package": "niobium_voice_input"
  }
}
```

## Event Bus

The event bus is the **central nervous system**. Everything communicates through it.

Owned by Rust — async, non-blocking, pub/sub:

- Plugins subscribe to event types they care about
- Plugins emit events for others to consume
- The MCP server emits events when agents call tools
- The UI engine emits events when users interact
- Environment sources emit events continuously (streams)

### Event Flow Examples

**Agent-triggered (request/response):**
```
Agent calls show_form()
  → MCP server emits ToolCalled
    → UI engine renders form
      → User submits
        → UI emits UserInteracted
          → MCP server returns result to agent
```

**Environment-triggered (streaming):**
```
Microphone captures audio continuously
  → Voice plugin emits VoiceActivity, VoiceChunk
    → Transcription emits VoiceTranscript
      → Agent (subscribed) receives transcript
      → UI plugin (subscribed) shows waveform
      → Logger plugin (subscribed) saves to disk
```

### Environment Sources

Plugins that stream events from the outside world:

| Source | Events |
|--------|--------|
| Voice capture | VoiceActivity, VoiceSilence, VoiceChunk, VoiceTranscript, VoiceCommand |
| Clipboard | ClipboardChanged |
| File watcher | FileCreated, FileChanged, FileDeleted |
| Screen capture | ScreenContent |
| System monitor | CpuSpike, LowMemory, BatteryLow |
| Bluetooth/IoT | DeviceNearby, SensorReading |
| Calendar | MeetingStarting, MeetingEnded |

Sources run in background threads (Rust) — real-time, no GC pauses, no dropped frames.

## Caching & Determinism

Forms and UI components are cached by `client_id`:

- First call creates the UI, assigns an internal `form_id`
- Subsequent calls with the same `client_id` serve from cache — instant, deterministic
- No LLM reasoning needed for repeat interactions
- Agent gets consistent behavior across sessions

## Tech Stack

```
┌─────────────────────────────┐
│  Flutter UI (Dart)           │  GPU-accelerated via Impeller
│  Widgets, animations,        │  Metal / Vulkan / DirectX
│  gestures, rich interaction  │
│  + UI plugin registry        │
├─────────────────────────────┤
│  flutter_rust_bridge (FFI)   │  Auto-generated bindings
├─────────────────────────────┤
│  Rust Core (native)          │  Event bus + plugin host
│  MCP server, env sources,    │  Compiled per target
│  plugin loading, processing  │
└─────────────────────────────┘
```

### Dart / Flutter (UI layer)
- All UI rendering, animations, and gestures
- Form components, charts, data visualization
- File pickers, dialogs, rich input widgets
- State management and caching
- UI plugin registry — third-party widgets register here

### Rust (Core layer)
- Event bus (async pub/sub, crossbeam/tokio channels)
- Plugin host (loads native, WASM, and bridges to Dart plugins)
- MCP server implementation (stdio + HTTP/SSE transports)
- Environment source plugins (voice, clipboard, file watcher, etc.)
- CPU-bound processing (audio, transcription, data transforms)

## MCP Tools (Planned)

| Tool | Purpose |
|------|---------|
| `show_form` | Render interactive forms (text, select, date, slider, etc.) |
| `show_chart` | Render data visualizations (line, bar, pie, scatter, etc.) |
| `pick_file` | Native file picker dialog |
| `pick_directory` | Native directory picker dialog |
| `show_dialog` | Confirmation, alert, or custom dialogs |
| `show_table` | Interactive data tables with sorting/filtering |
| `show_image` | Image display with zoom/pan |
| `show_markdown` | Rich markdown rendering |
| `notify` | System notifications |
| `subscribe` | Subscribe agent to event bus topics (env streams) |
| `unsubscribe` | Unsubscribe agent from event bus topics |

## Platform Targets

| Platform | Rust Target | UI Renderer |
|----------|-------------|-------------|
| Android  | `aarch64-linux-android` | Impeller (Vulkan) |
| iOS      | `aarch64-apple-ios` | Impeller (Metal) |
| macOS    | `aarch64-apple-darwin` | Impeller (Metal) |
| Windows  | `x86_64-pc-windows-msvc` | Impeller (DirectX) |
| Linux    | `x86_64-unknown-linux-gnu` | Impeller (Vulkan) |
| Web      | `wasm32-unknown-unknown` | CanvasKit / WASM |

## Key Dependencies

- **flutter_rust_bridge** — FFI codegen between Dart and Rust
- **Impeller** — Flutter's GPU rendering engine
- **Cargo** — Rust build system, cross-compiles per target automatically

## Why This Architecture

- **Rich UI**: Flutter owns every pixel, full control over rendering
- **Raw performance**: Rust handles event bus, env capture, and compute without GC pauses
- **Cross-platform**: Single codebase ships to mobile, desktop, and web
- **Agent-agnostic**: Any MCP-capable CLI agent gets rich UI + environment awareness for free
- **Pluggable**: New tools, widgets, and environment sources without touching core
- **Extensible**: Community can build and install plugins (WASM-sandboxed for safety)
- **Memory safe**: Rust guarantees at the core layer
- **Deterministic**: Cached UI components, consistent behavior

## Distribution

Builds produce native binaries per platform. Can be distributed via:
- App stores (iOS, Android, macOS, Windows)
- Direct download (desktop)
- Web deployment (WASM)
- npm package wrapping native binaries (for CLI agent ecosystems)

## Licensing & Governance

### License: AGPL v3 + Commercial

- **Public license**: AGPL v3 — anyone can use, modify, and contribute
- **Commercial license**: Available from HDRN for production/commercial use without AGPL copyleft obligations
- AGPL requires any network-facing use to release full source — companies that can't do that purchase a commercial license
- This preserves open-source visibility and community while retaining exclusive commercial rights

### Contributor License Agreement (CLA)

All contributors must sign a CLA before merging. This grants HDRN the right to dual-license contributions under both AGPL v3 and commercial terms.

- Enforced automatically via CLA Assistant on first PR
- Required to maintain the ability to offer commercial licenses

### Repository Strategy

1. **Development**: GitLab (private) — internal development, CI/CD, milestones
2. **Public release**: GitHub — community-facing repo, issues, PRs, CLA enforcement
3. Publish to GitHub when ready for public visibility and community contributions

## Design Considerations

### UI Description Format

Simple JSON schema that maps directly to Flutter widgets. Any LLM can generate it from MCP tool descriptions alone — no special training or format adoption needed. Compatibility with standards like A2UI can be added later as an optional input format.

### Transport
- **stdio**: simplest, works with all MCP clients, but blocks during UI interaction
- **HTTP/SSE (Streamable HTTP)**: better for long-running UI sessions, event streaming, concurrent tool calls
- **Recommendation**: support both — stdio for simple integrations, HTTP/SSE for production use and event subscriptions

### Async UI Interaction
MCP tool calls are request/response. Showing UI and waiting for user interaction is inherently async. The tool call blocks until the user completes the interaction. For event streams (environment sources), agents use `subscribe`/`unsubscribe` tools and receive events via MCP notifications or SSE.
