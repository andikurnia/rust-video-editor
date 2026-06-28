# Rust Video Editor — Architecture

## Design Principles

- **Clear crate boundaries** — core logic has zero I/O or UI dependencies
- **Pure core** — `editor-core` has no file I/O, no GPU, no FFmpeg; purely testable
- **Async media pipeline** — decode/encode runs on worker threads, marshals frames to UI
- **GPU compositing** — all frame rendering goes through wgpu shaders
- **Programmable API** — MCP server provides full editing surface for AI agents

## Crate Architecture

```
┌─────────────────────────────────────────────────────┐
│                  editor-app (binary)                 │
│  wires all crates together, owns event loop          │
├─────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐                 │
│  │  editor-ui    │  │ mcp-server   │                 │
│  │  egui panels  │  │ stdio MCP    │                 │
│  │  timeline UI  │  │ tool dispatch │                 │
│  │  preview      │  │              │                 │
│  └──────┬───────┘  └──────┬───────┘                 │
├─────────┼──────────────────┼────────────────────────┤
│  ┌──────┴──────────────────┴──────┐                  │
│  │        editor-core              │                  │
│  │  Timeline, Clip, Track,        │                  │
│  │  Commands, Project, Effects    │                  │
│  └────────┬──────────┬───────────┘                  │
│  ┌────────┴────────┐ ┌┴──────────────────┐          │
│  │  media-engine    │ │  render-engine     │          │
│  │  FFmpeg probe    │ │  wgpu compositor   │          │
│  │  decode/encode   │ │  shader effects    │          │
│  │  audio I/O       │ │  texture mgmt      │          │
│  └─────────────────┘ └───────────────────┘          │
│  ┌──────────────────────────────────────┐            │
│  │          project-store               │            │
│  │  SQLite persistence, project I/O     │            │
│  └──────────────────────────────────────┘            │
└──────────────────────────────────────────────────────┘
```

## Data Flow

1. **Import**: User imports media → `media-engine` probes file → `editor-core` creates `MediaItem`
2. **Edit**: User manipulates timeline → `editor-core` creates `TimelineCommand` → executes on `Timeline` → pushes to undo stack
3. **Preview**: Playhead moves → `media-engine` decodes frame at time → `render-engine` uploads to GPU texture → egui displays
4. **Save**: `project-store` serializes `Timeline` + `MediaItem`s to SQLite
5. **Export**: `media-engine` reads timeline → decodes frames → encodes with FFmpeg → writes output file
