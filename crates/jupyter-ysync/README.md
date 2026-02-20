# jupyter-ysync

> ⚠️ **Early Alpha** — This crate is in early alpha. The API is unstable and will change. It is being developed alongside conformance testing against [jupyter-server-documents](https://github.com/jupyter-ai-contrib/jupyter-server-documents) and [jupyter-collaboration](https://github.com/jupyterlab/jupyter-collaboration).

Y-sync protocol implementation for Jupyter notebook collaboration, built on [yrs](https://docs.rs/yrs) (Rust port of Y.js).

## What it does

- **NotebookDoc**: CRDT document representation of Jupyter notebooks
- **Conversion**: Bidirectional conversion between `nbformat::v4::Notebook` and Y.Doc
- **Character-level editing**: Cell sources use Y.Text for fine-grained collaboration
- **Protocol**: Y-sync v1 message encoding/decoding and sync state machine

## Features

| Feature | Description |
|---------|-------------|
| *(default)* | Core document types, conversion, and protocol |
| `client` | WebSocket client for connecting to Y-sync servers (`YSyncClient`, `NotebookSession`) |
| `python` | PyO3 bindings for conformance testing against `jupyter_ydoc` and `pycrdt` |

## Conformance status

This crate is being tested for compatibility with the Jupyter CRDT ecosystem:

- **jupyter_ydoc** — Y.Doc schema compatibility (cell structure, metadata layout, shared types)
- **pycrdt** — Update and state vector encoding roundtrips
- **jupyter-server-documents** — WebSocket sync protocol (via the `client` feature)

Conformance is partial and actively in progress. If you're building on this crate, expect breaking changes.

## Example

```rust
use jupyter_ysync::{NotebookDoc, notebook_to_ydoc, ydoc_to_notebook};

// Convert an existing notebook to a collaborative document
let doc = notebook_to_ydoc(&notebook)?;

// Make edits via the Y.Doc API
// ...

// Convert back to nbformat for saving
let updated_notebook = ydoc_to_notebook(&doc)?;
```

## License

BSD-3-Clause