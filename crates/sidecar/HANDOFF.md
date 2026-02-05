# Sidecar Widget Implementation - Handoff

## Current State

We've integrated [nteract elements](https://nteract-elements.vercel.app/) components into sidecar for rendering Jupyter outputs and widgets. The UI is built with Vite + React + Tailwind and embedded into the Rust binary via `rust-embed`.

### What's Working ✅

1. **Output Rendering** - All standard Jupyter outputs render correctly:
   - `text/plain`, `text/html`, `text/markdown`
   - Images (PNG, JPEG, SVG)
   - JSON (interactive tree view)
   - ANSI terminal colors
   - Error tracebacks

2. **Built-in ipywidgets** - Two-way binding works:
   - IntSlider, FloatSlider
   - IntProgress, FloatProgress
   - Button, Checkbox
   - Text, Textarea
   - Dropdown, RadioButtons, SelectMultiple
   - ToggleButton, ToggleButtons
   - Tab, Accordion
   - Box, HBox, VBox, GridBox
   - All 19 widget controls implemented with shadcn/ui

3. **anywidget Support** - Full lifecycle support:
   - Factory pattern (`export default () => ({ render, initialize })`)
   - Standard pattern (`export default { render }`)
   - CSS injection with cleanup
   - Two-way state binding (`model.get/set/save_changes`)
   - Custom messages (`model.send` + `model.on("msg:custom")`)

4. **quak / Data Widgets** - Complex anywidgets work:
   - SQL query dispatch via `model.send()`
   - Arrow IPC binary buffer responses
   - Mosaic coordinator integration
   - Full interactive data tables with histograms

## Key Fix: DataView[] for Buffers

The critical fix for anywidget support was converting buffers to `DataView[]` instead of `ArrayBuffer[]`.

**The Problem:**
JupyterLab services deserializes binary message buffers as `DataView[]`. Anywidgets like quak access the underlying ArrayBuffer via `buffers[0].buffer`:

```typescript
// quak widget.ts
const buffer = buffers[0].buffer;  // DataView.buffer → ArrayBuffer
const table = decodeIPC(buffer);
```

We were passing `ArrayBuffer[]` directly, which doesn't have a `.buffer` property.

**The Fix (App.tsx):**
```typescript
// Decode base64 buffers to DataView (matching JupyterLab's format)
if (message.buffers && Array.isArray(message.buffers)) {
  message.buffers = message.buffers.map((b64) => {
    if (typeof b64 === "string") {
      const binary = atob(b64);
      const bytes = new Uint8Array(binary.length);
      for (let i = 0; i < binary.length; i++) {
        bytes[i] = binary.charCodeAt(i);
      }
      return new DataView(bytes.buffer);  // DataView, not ArrayBuffer!
    }
    return b64;
  });
}
```

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
│        ▼                   ▼                    ▼          │
│   POST /message       Shell channel        comm_msg        │
│   (state updates)     (send to kernel)     (widget sync)   │
│                                                             │
│   globalThis.onMessage  IOPub channel     comm_open/msg    │
│   (receive outputs)     (from kernel)     (widget state)   │
└─────────────────────────────────────────────────────────────┘
```

## Key Files

| File | Purpose |
|------|---------|
| `ui/src/lib/widget-store.ts` | Widget model state management |
| `ui/src/lib/widget-store-context.tsx` | React context and hooks |
| `ui/src/lib/use-comm-router.ts` | Jupyter comm protocol handling |
| `ui/src/lib/buffer-utils.ts` | Buffer path handling for binary data |
| `ui/src/components/widgets/anywidget-view.tsx` | anywidget ESM loader + AFM interface |
| `ui/src/components/widgets/widget-view.tsx` | Routes to correct widget renderer |
| `ui/src/components/widgets/controls/` | All 19 built-in widget components |
| `ui/src/App.tsx` | Main app, message handling, buffer decoding |
| `src/main.rs` | Rust: IOPub listener, Shell sender, Webview |

## AFM (AnyWidget Frontend Module) Interface

Our `createAFMModelProxy()` implements the full AFM interface:

```typescript
interface AnyWidgetModel {
  get(key: string): unknown;           // Read from state (pending or committed)
  set(key: string, value: unknown);    // Buffer local changes
  save_changes(): void;                // Send buffered changes to kernel
  on(event: string, callback: Function);   // Subscribe to events
  off(event: string, callback?: Function); // Unsubscribe
  send(content: object, callbacks?, buffers?: ArrayBuffer[]); // Custom message
  widget_manager: {
    get_model(modelId: string): Promise<AnyWidgetModel>;  // Access other models
  };
}
```

### Event Types
- `change:key` - Fires when specific state key changes
- `change` - Fires on any state change  
- `msg:custom` - Fires when kernel sends custom message response

### Custom Message Protocol
```typescript
// Frontend → Kernel
data: { method: "custom", content: { ...userContent } }

// Kernel → Frontend  
data: { method: "custom", content: { ...responseContent } }
buffers: [DataView, ...]  // Binary data (Arrow IPC, etc.)
```

## Debugging

Key log prefixes:
- `[sidecar]` - Message routing in App.tsx
- `[anywidget]` - ESM loading and lifecycle
- `[AFM]` - AnyWidget Frontend Module proxy (get/set/send/on)

Example successful quak flow:
```
[anywidget] Loading ESM module...
[anywidget] Calling factory function...
[anywidget] Calling initialize...
[AFM] on: msg:custom
[anywidget] Calling render...
[AFM] send - full content: {"type":"arrow","sql":"SELECT...","uuid":"..."}
[sidecar] comm_msg details: {hasBuffers: true, bufferCount: 1, method: "custom"}
```

## Build Commands

```bash
# Build UI
cd crates/sidecar/ui
npm install
npm run build

# Build Rust (debug - includes devtools)
cargo build -p sidecar

# Run with info logging
RUST_LOG=info ./target/debug/sidecar /path/to/connection.json
```

## Testing

**Terminal 1** - Start a kernel:
```bash
python -m ipykernel_launcher -f /tmp/kernel.json
```

**Terminal 2** - Run sidecar:
```bash
RUST_LOG=info ./target/debug/sidecar /tmp/kernel.json
```

**Terminal 3** - Connect Jupyter console:
```bash
jupyter console --existing /tmp/kernel.json
```

Test ipywidgets:
```python
import ipywidgets as widgets
slider = widgets.IntSlider(value=50, min=0, max=100, description='Test:')
slider.observe(lambda change: print(f"Value: {change['new']}"), names='value')
display(slider)
```

Test quak:
```python
import polars as pl
import quak
df = pl.read_parquet("https://github.com/uwdata/mosaic/raw/main/data/athletes.parquet")
quak.Widget(df)
```

## nteract-elements Update Needed

The current nteract-elements registry has a bug in the `send()` implementation:

```typescript
// WRONG (spreads content into data):
data: { method: "custom", ...content, buffer_paths: [] }

// CORRECT (wraps content):
data: { method: "custom", content: content }
```

The correct version (which we have here) follows ipywidgets protocol where Python's `_handle_msg` extracts `data['content']`.

## Remaining Work

1. **Test more anywidgets** - Try ipyleaflet, ipyvolume, etc.
2. **Layout models** - IPY_MODEL_ references for layout/style
3. **Output widgets** - Nested output capture
4. **Performance** - Virtualize large outputs
5. **Backport to nteract-elements** - DataView fix and send() fix