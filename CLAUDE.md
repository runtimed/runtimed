# CLAUDE.md

## Project Overview

**runtimed** is a Rust workspace providing libraries for working with the Jupyter ecosystem. It enables Rust developers to create notebook applications, REPLs, and LLM-powered code execution tools by implementing the Jupyter messaging protocol, notebook format parsing, WebSocket connectivity, and real-time collaborative editing via CRDTs.

## Repository Structure

```
runtimed/
├── Cargo.toml              # Workspace root (resolver v2)
├── rust-toolchain.toml     # Pinned to Rust 1.90.0 with rustfmt + clippy
├── RELEASING.md            # Crate release order and commands
├── .github/
│   ├── workflows/
│   │   ├── linux.yml       # Main CI: build, test, doc tests (Linux)
│   │   ├── windows.yml     # Windows CI: runtimelib with tokio-runtime
│   │   └── clippy.yml      # Clippy with -Dwarnings (all warnings are errors)
│   └── requirements.txt    # Python deps for CI (jupyter, ipykernel, nbclient)
└── crates/
    ├── jupyter-protocol/   # Core Jupyter message types and traits (no internal deps)
    ├── jupyter-serde/      # DEPRECATED - re-exports jupyter-protocol
    ├── jupyter-websocket-client/  # WebSocket client for Jupyter servers
    ├── jupyter-ysync/      # Y-CRDT collaboration for notebooks (early alpha)
    ├── nbformat/           # Jupyter notebook parsing (v4.0-v4.5)
    ├── runtimelib/         # ZeroMQ-based native kernel interaction
    ├── ollama-kernel/      # Reference Jupyter kernel for Ollama (binary)
    └── mybinder/           # MyBinder build API parser (standalone)
```

## Crate Dependency Graph

```
jupyter-protocol (foundation - no internal deps)
    ├── jupyter-serde (deprecated re-export)
    ├── nbformat
    ├── jupyter-websocket-client
    └── runtimelib
         └── ollama-kernel
    └── jupyter-ysync (depends on jupyter-protocol, nbformat; optionally jupyter-websocket-client)

mybinder (standalone, no internal deps)
```

## Key Crates

### `jupyter-protocol`
Core types for Jupyter messaging (`JupyterMessage`, `JupyterMessageContent`, `Media`, `ConnectionInfo`). Transport-independent. All other crates depend on this.

### `runtimelib`
Native Jupyter kernel interaction over ZeroMQ. Requires a runtime feature flag:
- `tokio-runtime` — Tokio async runtime (most common)
- `async-dispatcher-runtime` — async-std/smol runtime
- `ring` (default) or `aws-lc-rs` — HMAC provider for message signing

### `nbformat`
Parses and serializes Jupyter notebooks. Supports v4.0-v4.4 (legacy) and v4.5 (current). Byte-for-byte roundtrip fidelity is a design goal.

### `jupyter-websocket-client`
WebSocket client connecting to Jupyter servers (local and remote).

### `jupyter-ysync`
Early alpha. CRDT-based collaborative editing of notebooks using Yrs (Rust Y.js port). Features: `client` (WebSocket sync), `python` (PyO3 bindings).

### `mybinder`
Standalone parser for MyBinder build API responses.

### `ollama-kernel`
Reference implementation of a Jupyter kernel backed by Ollama LLMs. Binary crate.

### `jupyter-serde`
Deprecated. Re-exports `jupyter-protocol`.

## Build Commands

```bash
# Build default members (excludes runtimelib and ollama-kernel)
cargo build

# Build runtimelib (requires runtime feature flag)
cargo build -p runtimelib --features tokio-runtime

# Build ollama-kernel
cargo build -p ollama-kernel

# Build runtimelib with alternative runtime
cargo build -p runtimelib --features async-dispatcher-runtime
```

## Testing

Tests require Python with `jupyter`, `ipykernel`, and `nbclient` installed, and kernel specs registered:
```bash
pip install jupyter ipykernel nbclient
python -m ipykernel install --user --name=python
python -m ipykernel install --user --name=python3
```

```bash
# Run default workspace tests
cargo test

# Run doc tests
cargo test --doc

# Test runtimelib with tokio
cargo test -p runtimelib --features tokio-runtime
cargo test -p runtimelib --doc --features tokio-runtime

# Test runtimelib with async-dispatcher
cargo test -p runtimelib --features async-dispatcher-runtime
cargo test -p runtimelib --doc --features async-dispatcher-runtime

# Run runtimelib tokio example (launches a Python kernel, executes code)
cargo run -p runtimelib --example tokio-launch-kernel --features tokio-runtime

# jupyter-ysync integration tests (requires running Jupyter server, ignored by default)
cargo test -p jupyter-ysync --features client -- --ignored --test-threads=1
```

## Linting

CI enforces `RUSTFLAGS="-Dwarnings"` — all Clippy warnings are treated as errors.

```bash
# Clippy for default workspace members
cargo clippy --all-targets

# Clippy for runtimelib (must specify runtime + crypto feature)
cargo clippy -p runtimelib --all-targets --no-default-features --features "tokio-runtime,ring"
cargo clippy -p runtimelib --all-targets --no-default-features --features "async-dispatcher-runtime,ring"
```

## Formatting

```bash
cargo fmt --all          # Format all crates
cargo fmt --all --check  # Check formatting without modifying
```

## Code Conventions

### Error Handling
- Each crate defines its own error type using `thiserror`:
  - `jupyter-protocol` → `JupyterError`
  - `runtimelib` → `RuntimeError` (with `Result<T>` type alias)
  - `jupyter-ysync` → `YSyncError` (with `Result<T>` type alias)
  - `nbformat` → `NotebookError`
- Error types are `#[non_exhaustive]` where appropriate (see `RuntimeError`)
- `anyhow` is used in binary crates (`ollama-kernel`) and examples

### Async Patterns
- `runtimelib` abstracts over async runtimes via feature flags (`tokio-runtime`, `async-dispatcher-runtime`)
- `async-trait` is used for trait async methods in `jupyter-protocol`
- Connection types are generic over ZeroMQ socket types: `Connection<S: zeromq::Socket>`

### Serialization
- `serde` with derive is used throughout for JSON serialization
- `uuid` with `v4` and `v5` features for message IDs and cell IDs
- `chrono` for timestamps (no default features, `std` + `serde` only)
- Notebook serialization uses single-space indentation to match Python nbformat

### Module Organization
- Each crate uses `lib.rs` with `pub mod` declarations and `pub use` re-exports
- Internal modules are kept private (`mod error`) with selective public re-exports
- Feature-gated modules use `#[cfg(feature = "...")]`
- Deprecated re-exports are marked with `#[deprecated]` and `#[doc(hidden)]`

### Naming
- Crate names use hyphens: `jupyter-protocol`, `jupyter-websocket-client`
- Module/type names follow Rust conventions: `JupyterMessage`, `ConnectionInfo`
- Connection type aliases use descriptive names: `ClientShellConnection`, `KernelIoPubConnection`

## Workspace Configuration

- **Resolver**: v2
- **Default members**: All crates except `runtimelib` and `ollama-kernel` (these need feature flags)
- **Shared dependencies**: Defined in `[workspace.dependencies]` and referenced with `{ workspace = true }`
- **Release profile**: Optimized for size (`opt-level = 'z'`, LTO, strip, single codegen unit, panic=abort)

## Rust Toolchain

Pinned to **Rust 1.90.0** with `rustfmt` and `clippy` components via `rust-toolchain.toml`.

## Release Process

Uses `cargo-release`. Crates must be released in dependency order:
1. `jupyter-protocol` (first, everything depends on it)
2. `jupyter-serde`, `nbformat`, `jupyter-websocket-client` (can release together)
3. `runtimelib` (requires `--features tokio-runtime` when publishing)
4. `jupyter-ysync` (after jupyter-protocol, nbformat, and jupyter-websocket-client)
5. `ollama-kernel` (after runtimelib)

`mybinder` is standalone and can be released independently.

## CI Workflows

- **linux.yml**: Full build + test suite on Ubuntu (default members, runtimelib with both runtimes, ollama-kernel build, examples)
- **windows.yml**: runtimelib build + test with tokio-runtime on Windows
- **clippy.yml**: Clippy lint check with warnings-as-errors for all feature combinations

All workflows trigger on push/PR to `main`.

## License

BSD-3-Clause (all crates).
