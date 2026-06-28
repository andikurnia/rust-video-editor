# AI Agent Working Rules

## Project: Rust Video Editor

### Workspace Layout
- `crates/editor-core/` — Pure timeline logic; no I/O, no GPU, no FFmpeg
- `crates/media-engine/` — FFmpeg probe, decode, encode, audio
- `crates/render-engine/` — wgpu compositor, shaders (WGSL)
- `crates/project-store/` — SQLite persistence
- `crates/mcp-server/` — MCP protocol server (stdio)
- `crates/editor-ui/` — egui frontend panels
- `crates/editor-app/` — Thin binary entry point

### Conventions
- All new types derive `Debug, Clone, Serialize, Deserialize`
- Error types use `thiserror` with `#[error("...")]` format strings
- Timeline operations go through `TimelineCommand` trait for undo/redo
- Long-running work (decode, encode, waveform gen) runs on worker threads
- MCP tools match the naming pattern: `domain.action` (e.g., `timeline.add_clip`)

### Testing
- Core timeline logic has non-I/O unit tests in `editor-core`
- Integration tests go in `tests/` at workspace root
- Run: `cargo test` for all crate tests

### Building
- `cargo build` — builds all crates
- `cargo run` — launches GUI
- `cargo run -- --mcp` — launches MCP server in stdio mode
