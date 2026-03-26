# GraniteX — Development Tooling Guide

## Essential Cargo Tools (install after Rust)

```bash
# Fast test runner (parallel, better output than cargo test)
cargo install cargo-nextest

# Auto-rebuild on file change
cargo install cargo-watch

# Benchmarking
cargo install cargo-criterion

# CPU profiling / flamegraphs
cargo install flamegraph

# Dependency audit
cargo install cargo-audit

# Check for unused dependencies
cargo install cargo-udeps

# Better error messages during compilation
cargo install cargo-expand
```

## Day-to-Day Workflow

```bash
# Dev loop: auto-rebuild and run on save
cargo watch -x run

# Run tests continuously
cargo watch -x 'nextest run'

# Run only tests matching a pattern
cargo nextest run test_camera

# Benchmark geometry operations
cargo criterion --bench geometry

# Profile a slow frame
cargo flamegraph -- --bin granitex
```

## Testing Strategy

| Layer        | Tool         | What to test                              |
|-------------|-------------|------------------------------------------|
| Geometry    | proptest    | Invariants (normals consistent, no degenerate triangles) |
| Rendering   | RenderDoc   | Visual debugging, shader inspection       |
| UI          | insta       | Snapshot tests for serialized UI state    |
| File I/O    | assert_eq   | Round-trip: load → save → load → compare  |
| Performance | criterion   | Frame time, mesh operation throughput      |

## GPU Debugging

- **RenderDoc** (https://renderdoc.org/) — Free, works with wgpu's Vulkan/DX12 backends
- Set `WGPU_BACKEND=vulkan` env var for best RenderDoc compatibility
- wgpu has built-in validation that catches most GPU errors in debug builds

## CI Recommendations (GitHub Actions)

- `cargo nextest run` — unit + integration tests
- `cargo clippy -- -D warnings` — lint
- `cargo fmt --check` — formatting
- `cargo audit` — security vulnerabilities in deps
- `cargo udeps` — catch unused dependencies
