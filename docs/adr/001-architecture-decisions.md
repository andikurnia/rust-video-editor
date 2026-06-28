# ADR-001: Technology Stack and Architecture

## Status

Accepted

## Context

We are building a cross-platform open-source video editor in Rust. Need to choose UI framework, media backend, GPU API, persistence, and AI integration approach.

## Decisions

### UI: egui + wgpu
- **Why**: Pure Rust, no web runtime, tight wgpu integration for preview compositing, immediate mode simplifies timeline widget development
- **Alternatives considered**: Tauri 2 (heavier, web dependency), Slint (newer ecosystem)
- **Tradeoff**: egui widget system requires custom timeline implementation vs. web-based approaches

### Media Backend: FFmpeg via ffmpeg-next
- **Why**: Most mature codec support, hardware acceleration (NVENC/VA-API/VideoToolbox), largest ecosystem
- **Alternatives considered**: GStreamer (complex setup), pure Rust codecs (immature for production)
- **Deferred**: Phase 1 implementation after core data model

### GPU: wgpu
- **Why**: Cross-platform (Vulkan/Metal/DX12), safe Rust API, compositor preview pipeline
- **Alternatives considered**: Vulkan direct ash (Linux-only, more complex)

### Persistence: SQLite via rusqlite
- **Why**: Transactional integrity, incremental saves, queryable, widely used in Rust ecosystem
- **Alternatives considered**: Pure JSON (simpler but no querying or incremental save), sled (less mature)

### AI Interface: MCP Server (stdio transport)
- **Why**: Industry standard for AI agent integration, used by multiple Rust video editors
- **Library**: Custom implementation using rmcp-style JSON-RPC over stdio

### License: MIT OR Apache-2.0
- **Why**: Dual license matches Rust ecosystem standard (used by egui, wgpu, serde). Maximum adoption with patent protection
