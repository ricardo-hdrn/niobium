# Niobium — Prior Art & Landscape Research

Research conducted: 2026-02-26

## Summary

The concept of a standalone rich UI runtime for CLI AI agents, exposed via MCP, does **not exist** as a unified project. The building blocks exist but nobody has assembled them.

---

## Existing Solutions & How They Compare

### 1. MCP Apps (Official MCP Extension)

- **Repo**: [modelcontextprotocol/ext-apps](https://github.com/modelcontextprotocol/ext-apps)
- **Spec**: [modelcontextprotocol.io/docs/extensions/apps](https://modelcontextprotocol.io/docs/extensions/apps)
- **What it does**: MCP servers return interactive HTML interfaces (forms, dashboards, charts) that render inside the host client's conversation via sandboxed iframes.
- **Supported clients**: Claude (web), Claude Desktop, VS Code GitHub Copilot, Goose, Postman
- **Gap**: Only works inside GUI hosts. A pure CLI agent in a terminal **cannot render iframes**. Not a standalone UI runtime.

### 2. MCP-UI (Community, 4.5k stars)

- **Repo**: [MCP-UI-Org/mcp-ui](https://github.com/MCP-UI-Org/mcp-ui)
- **Website**: [mcpui.dev](https://mcpui.dev/)
- **What it does**: Pioneered the concept of rich UI in MCP tool responses. Now implements the MCP Apps standard. Packages: `@mcp-ui/server` (TS), `@mcp-ui/client` (React).
- **Adopters**: Shopify built interactive e-commerce experiences (product catalogs, checkout flows) with MCP-UI.
- **Gap**: Same as MCP Apps — renders inside a host, not standalone.

### 3. Google A2UI (Agent-to-User Interface)

- **Repo**: [google/A2UI](https://github.com/google/A2UI)
- **Website**: [a2ui.org](https://a2ui.org/)
- **Version**: v0.8-0.9, Public Preview, Apache 2.0
- **What it does**: Agent sends **declarative JSON** describing UI components. Client renders them using its **own native widget set**. Framework-agnostic.
- **Flutter support**: Yes, via GenUI SDK. Also Angular, Lit, with React and Jetpack Compose planned.
- **Key properties**:
  - Declarative data, not executable code (security by design)
  - Flat component list with identifier references
  - Designed for LLM streaming/incremental generation
- **Gap**: A2UI is a UI description format, not a running MCP server. Needs a bridge to MCP.
- **Relevance to Niobium**: HIGH — could adopt A2UI as the UI description format instead of inventing a custom one.

### 4. AG-UI (Agent-User Interaction Protocol, 12.2k stars)

- **Repo**: [ag-ui-protocol/ag-ui](https://github.com/ag-ui-protocol/ag-ui)
- **Docs**: [docs.ag-ui.com](https://docs.ag-ui.com/)
- **Origin**: CopilotKit
- **What it does**: Transport/interaction layer between agents and frontends. ~16 event types covering text streaming, tool orchestration, state sync, generative UI. SSE/WebSocket-based.
- **SDKs**: Kotlin, Go, Dart, Java, Rust, Ruby
- **Relationship**: MCP = data/tools layer, A2A = agent-to-agent, AG-UI = agent-to-frontend
- **Gap**: Assumes a frontend application exists. Does not provide one.

### 5. Native Desktop Popup MCP Servers

Small projects that prove the concept of "MCP server pops up native UI for a CLI agent":

#### popup-mcp (Rust)
- **Repo**: [inanna-malick/popup-mcp](https://github.com/inanna-malick/popup-mcp)
- Native GUI popup windows with form elements: text inputs, sliders, checkboxes, dropdowns, multiselect, conditional visibility
- Cross-platform (Rust)
- **Closest to Niobium's concept** but limited scope (forms only)

#### consult-user-mcp
- **Repo**: [doublej/consult-user-mcp](https://github.com/doublej/consult-user-mcp)
- Native modals (SwiftUI on macOS, WPF on Windows)
- Tools: `ask` (confirm/pick/text/form), `notify`, `tweak` (real-time value adjustment)
- Supports snoozing, feedback, 10-minute timeouts

#### user-prompt-mcp
- **Repo**: [nazar256/user-prompt-mcp](https://github.com/nazar256/user-prompt-mcp)
- Uses zenity (Linux) and osascript (macOS) for native dialog boxes
- Simple but functional

#### apple-notifier-mcp
- **Repo**: [turlockmike/apple-notifier-mcp](https://github.com/turlockmike/apple-notifier-mcp)
- macOS-specific: native notifications, interactive prompts, text-to-speech, screenshots
- **Native file picker dialog** with file type filters and multi-select
- One of the few with a real native file picker as an MCP tool

### 6. Visualization MCP Servers

- [antvis/mcp-server-chart](https://github.com/antvis/mcp-server-chart) — 25+ chart types
- [QuickChart MCP Server](https://github.com/GongRzhe/Quickchart-MCP-Server) — URL-based chart generation
- [mcp-visualization-duckdb](https://github.com/xoniks/mcp-visualization-duckdb) — Natural language to Plotly
- [plotting-mcp](https://github.com/StacklokLabs/plotting-mcp) — CSV to line/bar/pie/map

These generate charts as images or HTML files, not persistent interactive UI.

### 7. Flutter + MCP Projects (None Do What Niobium Does)

- [Arenukvern/mcp_flutter](https://github.com/Arenukvern/mcp_flutter) — MCP server for Flutter *development* (error monitoring, screenshots). Not a UI rendering service.
- [flutter_mcp](https://pub.dev/packages/flutter_mcp) — Integrates MCP into Flutter apps with background execution and system tray. Closer but not the same concept.
- Official `dart mcp-server` — SDK development tools (analyze, format, test). Not UI rendering.

### 8. Other Related

- **Gradio as MCP Server** — Gradio apps can be launched as MCP servers (`mcp_server=True`). Each API endpoint becomes an MCP tool. But renders in a browser, not native.
- **Shinkai Desktop** — Tauri-based desktop app that exposes agent capabilities as MCP servers. Reverse of Niobium's concept.

---

## Gap Analysis

| Capability | Exists? | Where |
|---|---|---|
| Rich UI in agent conversations | Yes | MCP Apps, MCP-UI |
| Native desktop popups with forms | Partial | popup-mcp, consult-user-mcp |
| Native file picker as MCP tool | macOS only | apple-notifier-mcp |
| Charts/visualization as MCP tools | Yes | antvis, QuickChart, etc. |
| Declarative UI spec for agents | Yes | A2UI (supports Flutter) |
| Agent-to-frontend protocol | Yes | AG-UI |
| **Standalone desktop UI runtime for CLI agents** | **No** | — |
| **Flutter app as MCP UI server** | **No** | — |
| **Unified UI service (forms + charts + pickers + dialogs)** | **No** | — |
| **Works with ANY CLI agent** | **No** | — |

---

## Niobium's Position

Niobium fills the gap between:
- **MCP Apps/MCP-UI** (rich UI, but locked inside GUI hosts — useless for CLI agents)
- **popup-mcp / consult-user-mcp** (works for CLI agents, but minimal UI — just popups and forms)
- **A2UI** (great declarative format, but no runtime — just a spec)

Niobium = **A2UI-style declarative UI** + **Flutter native rendering** + **MCP server interface** + **works with any CLI agent**.

---

## Key References

- [MCP Apps Blog Post (Jan 2026)](http://blog.modelcontextprotocol.io/posts/2026-01-26-mcp-apps/)
- [MCP-UI: Breaking the Text Wall (Shopify)](https://shopify.engineering/mcp-ui-breaking-the-text-wall)
- [MCP-UI Creators Interview (The New Stack)](https://thenewstack.io/mcp-ui-creators-on-why-ai-agents-need-rich-user-interfaces/)
- [Introducing A2UI (Google Developers Blog)](https://developers.googleblog.com/introducing-a2ui-an-open-project-for-agent-driven-interfaces/)
- [The State of Agentic UI (CopilotKit)](https://www.copilotkit.ai/blog/the-state-of-agentic-ui-comparing-ag-ui-mcp-ui-and-a2ui-protocols)
- [Agent UI Standards Multiply (The New Stack)](https://thenewstack.io/agent-ui-standards-multiply-mcp-apps-and-googles-a2ui/)
