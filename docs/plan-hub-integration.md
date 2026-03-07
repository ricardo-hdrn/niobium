# Niobium — Hub Integration Plan

## Current State

Niobium is a standalone MCP server (stdio). It renders forms, confirmations,
output displays, and runs pipelines. No connection to mcp-discuss hub.

What exists and works:
- 5 MCP tools (show_form, show_confirmation, show_output, save/list forms)
- 17+ form field types with scale-aware widget selection
- Pipeline engine (HTTP, process, transform, parallel, redaction)
- FFI bridge (single-process Rust + Flutter)
- Output display (text, markdown, JSON, table, diff)
- Theme system with configurable accent

What does NOT exist:
- WebSocket client
- Hub connection
- Pills rendering (real-time updates from hub)
- Voice integration
- Output types from arch v2 (datatable, grid, toast, decision)

---

## Phase 1 — Hub Sync (WebSocket Client)

**Goal:** Niobium connects to mcp-discuss hub and receives real-time updates.

### Rust side
- [ ] Add WebSocket client to niobium-mcp (tokio-tungstenite)
- [ ] Hub connection config (url, auth token) via env or config file
- [ ] Reconnection logic with backoff
- [ ] Parse incoming hub events (new update, state change, new actionable)
- [ ] Emit events on the existing event bus (new event types)

### Flutter side
- [ ] New FFI callback: `on_hub_event(String) -> ()` (JSON payload, fire-and-forget)
- [ ] Hub events land in Dart, ready for rendering

### Hub side (mcp-discuss)
- [ ] WebSocket endpoint on hub (`/ws`) with auth
- [ ] Push events: new update_event, actionable state change, new actionable
- [ ] Per-user filtering (only push events for authenticated user)

---

## Phase 2 — Pills View

**Goal:** Niobium renders a persistent Pills view showing hub activity.

### Flutter side
- [ ] New `PillsView` widget — persistent panel (not a popup like forms)
- [ ] Shows feed of hub events (newest first)
- [ ] Each pill = one update event or actionable state change
- [ ] Pill renders based on output_type (text, markdown, datatable, grid, toast)
- [ ] Toast pills auto-dismiss after timeout
- [ ] Pills view coexists with form/output popups (split screen or tabbed)

### Output type renderers
- [ ] `markdown` — reuse existing output_display markdown renderer
- [ ] `datatable` — sortable/searchable table (new widget, based on existing table renderer)
- [ ] `grid` — visual card grid layout (new widget)
- [ ] `toast` — ephemeral notification bar (map to existing toast mechanism)
- [ ] `decision` — plaintext choices rendered as buttons

### Data model (Dart)
- [ ] `HubEvent` model (maps to hub's update event + output fields)
- [ ] `Pill` model (rendered representation of a hub event)
- [ ] Local cache of recent pills (in-memory, bounded)

---

## Phase 3 — Decision Response Flow

**Goal:** Workers ask questions via hub, user answers in Niobium, response flows back.

### Flow
```
Worker posts update with output_type: "decision"
  -> hub stores it
  -> WS pushes to Niobium
  -> Pills renders decision as buttons
  -> User picks an option
  -> Niobium POSTs response back to hub (REST)
  -> Hub stores response on the update/actionable
  -> Worker reads response
```

### Niobium side
- [ ] Decision pill renders options as tappable buttons
- [ ] On tap, POST response to hub via REST (reuse pipeline HTTP or direct reqwest)
- [ ] Mark decision pill as "answered" in UI
- [ ] New FFI callback or direct Rust HTTP call for response

### Hub side (mcp-discuss)
- [ ] `response` field on actionable_updates
- [ ] `PUT /actionables/{id}/updates/{update_id}/response` endpoint
- [ ] Push response event via WS (so worker gets notified)

---

## Phase 4 — Voice Integration

**Goal:** Voice runs inside Niobium process, talks to Gemini Live and hub.

### Architecture decision needed
- Option A: Rust-native audio (cpal + Gemini Live WS protocol)
- Option B: Flutter-native audio (flutter_sound + web_socket_channel)
- Option C: Keep Python voice-client as subprocess, bridge via IPC

### Regardless of option
- [ ] Voice component captures mic audio, streams to Gemini Live API
- [ ] Receives tool_call objects, executes against hub REST API
- [ ] Sends tool_response back to Gemini
- [ ] Can inject text from other Niobium components (cross-component events)
- [ ] Session startup: loads subject context from hub

### Cross-component events
- [ ] Pills detects new hub update → injects text into voice session
- [ ] Voice receives proactive notification, Gemini speaks it
- [ ] Uses existing event bus for in-process communication

---

## Phase 5 — Output Types in Hub Schema

**Goal:** Hub stores typed outputs, Niobium renders them.

### Hub side (mcp-discuss)
- [ ] Add `output_type` field to update_events and actionable_updates
- [ ] Add `output` field (JSON blob) to update_events and actionable_updates
- [ ] Add `response` field to actionable_updates (for decisions)
- [ ] Update REST + WS to include these fields

### Niobium side
- [ ] Parse output_type + output from hub events
- [ ] Route to appropriate renderer (reuse show_output renderers where possible)
- [ ] Handle null output_type as plain text (summary only)

---

## Dependencies

```
Phase 1 (WS client) — no dependency, can start now
Phase 2 (Pills view) — depends on Phase 1
Phase 3 (Decisions) — depends on Phase 2
Phase 4 (Voice) — independent, can parallel with Phase 2-3
Phase 5 (Output types) — depends on Phase 2, parallel with Phase 3
```

## What NOT to build

- Encryption (Layer 1+, not now)
- Plugin system (WASM sandboxing) — existing scaffolding is enough
- Mobile Niobium — desktop first
- SSE/HTTP streaming transport — stdio + WS covers all cases
