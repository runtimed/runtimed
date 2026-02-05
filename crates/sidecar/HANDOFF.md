# Sidecar Widget Implementation - Handoff

## Overview

The sidecar is a standalone Jupyter output viewer built with Rust (Wry/Tauri webview) and React. It connects to a running Jupyter kernel and displays outputs including interactive widgets.

We're using components from [nteract/elements](https://nteract-elements.vercel.app/) for the UI, which provides shadcn/ui-based Jupyter components.

## Current State

### What's Working ✅

1. **Output Rendering** - All standard MIME types:
   - text/plain, text/html, text/markdown
   - Images (PNG, JPEG, SVG)
   - JSON (interactive tree view)
   - ANSI terminal colors
   - Error tracebacks

2. **ipywidgets Controls** - Two-way binding works:
   - IntSlider, FloatSlider
   - IntProgress, FloatProgress
   - Button (all styles), Checkbox
   - Text, Textarea
   - Dropdown, RadioButtons, SelectMultiple
   - ToggleButton, ToggleButtons
   - Tab, Accordion
   - Box, HBox, VBox, GridBox

3. **anywidget Support** - Full lifecycle:
   - Factory pattern (`export default () => ({ render, initialize })`)
   - CSS injection with cleanup
   - Two-way state binding
   - Custom messages with binary buffers
   - **quak works!** (complex data table with Arrow IPC)

### What's NOT Working ❌

Some ipywidgets controls are not yet implemented. Reported in https://github.com/nteract/elements/issues/89:

| Model | Description |
|-------|-------------|
| `HTMLModel` | Render arbitrary HTML (very common) |
| `ColorPickerModel` | Color picker input |
| `IntRangeSliderModel` | Dual-handle integer range slider |
| `FloatRangeSliderModel` | Dual-handle float range slider |

## Recent Fixes

### 1. DataView[] for Buffers

JupyterLab deserializes binary buffers as `DataView[]`, not `ArrayBuffer[]`. Anywidgets access the underlying buffer via `buffers[0].buffer`:

```typescript
// In App.tsx - decode base64 to DataView
return new DataView(bytes.buffer);  // NOT bytes.buffer directly
```

### 2. Content Wrapping in send()

The `send()` method must wrap content for ipywidgets protocol:

```typescript
// CORRECT:
data: { method: "custom", content: content }

// WRONG:
data: { method: "custom", ...content }
```

Both fixes have been submitted to nteract/elements and merged.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Sidecar App                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌──────────────┐    ┌───────────────┐  │
│  │   Webview   │───▶│  Rust/Wry    │───▶│    Kernel     │  │
│  │  (React UI) │◀───│  (ZMQ/JSON)  │◀───│   (Python)    │  │
│  └─────────────┘    └──────────────┘    └───────────────┘  │
│        │                   │                    │          │
│   POST /message       Shell channel        comm_msg        │
│   (widget updates)    (to kernel)          (widget sync)   │
│                                                             │
│   globalThis.onMessage  IOPub channel     comm_open/msg    │
│   (receive outputs)     (from kernel)     (widget state)   │
└─────────────────────────────────────────────────────────────┘
```

## Key Files

| File | Purpose |
|------|---------|
| `ui/src/App.tsx` | Main app, message handling, buffer decoding |
| `ui/src/lib/widget-store.ts` | Widget model state management |
| `ui/src/lib/widget-store-context.tsx` | React context and hooks |
| `ui/src/lib/use-comm-router.ts` | Jupyter comm protocol handling |
| `ui/src/lib/buffer-utils.ts` | Buffer path handling for binary data |
| `ui/src/components/widgets/anywidget-view.tsx` | anywidget ESM loader + AFM interface |
| `ui/src/components/widgets/widget-view.tsx` | Routes to correct widget renderer |
| `ui/src/components/widgets/widget-registry.ts` | Maps model names to React components |
| `ui/src/components/widgets/controls/` | Individual widget implementations |
| `src/main.rs` | Rust: ZMQ, Webview, message routing |

## nteract/elements Integration

We use shadcn's registry system to pull components:

```bash
cd crates/sidecar/ui
npx shadcn@latest add https://nteract-elements.vercel.app/r/widget-store.json
npx shadcn@latest add https://nteract-elements.vercel.app/r/anywidget-view.json
```

**Note:** After pulling, you may need to adjust import paths. The registry installs to `@/registry/widgets/` but our structure uses `@/lib/` and `@/components/widgets/`.

Related issues:
- https://github.com/nteract/elements/issues/63 - Widget support RFC
- https://github.com/nteract/elements/issues/73 - Phase II widgets
- https://github.com/nteract/elements/issues/89 - Phase III (HTMLModel, etc.)

## Build & Test

```bash
# Build UI
cd crates/sidecar/ui
npm install
npm run build

# Build Rust (debug - includes devtools via Cmd+Option+I)
cargo build -p sidecar

# Run
RUST_LOG=info ./target/debug/sidecar /path/to/kernel-connection.json
```

### Test Setup

**Terminal 1** - Start kernel:
```bash
python -m ipykernel_launcher -f /tmp/kernel.json
```

**Terminal 2** - Run sidecar:
```bash
RUST_LOG=info ./target/debug/sidecar /tmp/kernel.json
```

**Terminal 3** - Jupyter console:
```bash
jupyter console --existing /tmp/kernel.json
```

### Test Commands

```python
# Basic ipywidgets
import ipywidgets as widgets
widgets.IntSlider(value=50, min=0, max=100, description='Test:')

# quak (anywidget with binary data)
import polars as pl
import quak
df = pl.read_parquet("https://github.com/uwdata/mosaic/raw/main/data/athletes.parquet")
quak.Widget(df)

# Complex composition
tabs = widgets.Tab(children=[
    widgets.VBox([widgets.IntSlider(), widgets.FloatSlider()]),
    widgets.VBox([widgets.Text(), widgets.Textarea()]),
])
tabs.set_title(0, 'Sliders')
tabs.set_title(1, 'Text')
display(tabs)
```

## Next Steps

1. **Pull updated nteract/elements components**
   - The DataView buffer fix and send() content wrapping fix have been merged upstream
   - Pull fresh copies of `widget-store.json` and `anywidget-view.json`
   - Adjust import paths as needed (registry uses `@/registry/widgets/`, we use `@/lib/`)

2. **Verify quak still works** after component update
   ```python
   import polars as pl
   import quak
   df = pl.read_parquet("https://github.com/uwdata/mosaic/raw/main/data/athletes.parquet")
   quak.Widget(df)
   ```

3. **Test additional anywidgets**:
   - `jupyter-scatter` - WebGL scatter plots with binary data
   - `drawdata` - Drawing/annotation widget
   - Any other anywidget-based libraries

4. **Don't worry about Phase III controls** - The nteract/elements team is actively working on:
   - `HTMLModel`, `ColorPickerModel`, `IntRangeSliderModel`, `FloatRangeSliderModel`
   - Track progress: https://github.com/nteract/elements/issues/89
   - These will come in a future update

5. **Future work** (lower priority):
   - Layout models - IPY_MODEL_ references for widget layout/style
   - Output widget - Nested output capture (`widgets.Output()`)

## Debugging

Console log prefixes:
- `[sidecar]` - Message routing in App.tsx
- `[anywidget]` - ESM loading and lifecycle
- `[AFM]` - AnyWidget Frontend Module proxy (get/set/send/on)

Open devtools with **Cmd+Option+I** (debug builds only).

## Git History

Recent commits on `sidecar-with-elements` branch:
```
fix(sidecar): use DataView[] for buffers to match JupyterLab protocol
feat(sidecar): shadcn widget controls + anywidget AFM proxy
feat(sidecar): implement two-way widget binding
```
