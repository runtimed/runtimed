# Sidecar Widget Implementation - Handoff

## Current State

We've successfully integrated [nteract elements](https://nteract-elements.vercel.app/) into sidecar for rendering Jupyter outputs and widgets. The UI is built with Vite + React + Tailwind and embedded into the Rust binary via `rust-embed`.

### What's Working

1. **Output Rendering** - All standard Jupyter outputs render correctly:
   - `text/plain`, `text/html`, `text/markdown`
   - Images (PNG, JPEG, SVG)
   - JSON (interactive tree view)
   - ANSI terminal colors
   - Error tracebacks

2. **Widget Infrastructure** - The widget store and message routing is in place:
   - `WidgetStoreProvider` wraps the app
   - `comm_open` creates widget models in the store
   - `comm_msg` updates widget state
   - `comm_close` removes widgets

3. **Widget Rendering** - Widgets display correctly:
   - **anywidgets** (like `quak`) render via `AnyWidgetView` 
   - **IntSlider** renders with shadcn's Slider component
   - `WidgetView` routes to the appropriate renderer based on model type

4. **Two-Way Binding** ✅ - Widget state syncs back to the kernel:
   - Dragging the IntSlider updates the kernel's widget state
   - `slider.observe()` callbacks fire correctly
   - Messages flow: Frontend → Rust → Shell channel → Kernel

### Architecture

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
| `ui/src/components/widgets/use-comm-router.ts` | Constructs outgoing comm messages |
| `ui/src/components/widgets/widget-store.ts` | Widget model state management |
| `ui/src/components/widgets/widget-store-context.tsx` | React context for widget store |
| `ui/src/components/widgets/controls/int-slider.tsx` | IntSlider widget component |
| `ui/src/App.tsx` | Main app, provides `sendMessage` to provider |
| `src/main.rs` | Rust: handles `/message` POST, sends to kernel |

## Message Format Reference

A `comm_msg` for updating widget state:

```json
{
  "header": {
    "msg_id": "<uuid>",
    "msg_type": "comm_msg",
    "username": "sidecar",
    "session": "<session-id>",
    "date": "<iso-timestamp>",
    "version": "5.3"
  },
  "parent_header": null,
  "metadata": {},
  "content": {
    "comm_id": "<widget-model-id>",
    "data": {
      "method": "update",
      "state": {
        "value": 42
      },
      "buffer_paths": []
    }
  },
  "buffers": [],
  "channel": "shell"
}
```

**Important**: `parent_header` must be `null` (not `{}`), otherwise Rust's serde will try to deserialize an empty object as a `Header` struct and fail.

## Next Steps

### Near-term

1. **More Widget Controls** - Add support for more ipywidgets:
   - FloatSlider, IntText, FloatText
   - Checkbox, ToggleButton
   - Dropdown, Select, RadioButtons
   - Text, Textarea
   - Button (with click events)

2. **Widget Layout** - Support Layout and Style models for proper widget sizing/styling

3. **Linked Widgets** - Handle `IPY_MODEL_` references for widgets that reference other widgets

### Medium-term

1. **Binary Buffers** - Full support for `buffer_paths` and binary data transfer (needed for array-heavy widgets)

2. **Custom Messages** - Handle `method: "custom"` for widgets with custom comm protocols

3. **Error Handling** - Graceful degradation when widgets fail to render

### Long-term

1. **Full ipywidgets Support** - Cover the complete ipywidgets control library

2. **Third-party Widgets** - Support for popular widget libraries (bqplot, ipyvolume, etc.)

3. **Performance** - Optimize for high-frequency updates (e.g., continuous_update on sliders)

## Build Commands

```bash
# Build UI
cd crates/sidecar/ui
npm install
npm run build

# Build Rust
cargo build -p sidecar

# Run with debug logging
RUST_LOG=debug ./target/debug/sidecar /path/to/connection.json

# Run with message dump
./target/debug/sidecar --dump messages.jsonl /path/to/connection.json
```

## Testing Setup

**Terminal 1** - Start a kernel:
```bash
python -m ipykernel_launcher -f /tmp/kernel.json
```

**Terminal 2** - Run sidecar:
```bash
./target/debug/sidecar /tmp/kernel.json
```

**Terminal 3** - Connect a console:
```bash
jupyter console --existing /tmp/kernel.json
```

Then test:
```python
import ipywidgets as widgets
slider = widgets.IntSlider(value=50, min=0, max=100, description='Test:')
slider.observe(lambda change: print(f"Value changed: {change['new']}"), names='value')
display(slider)

# After dragging in sidecar UI:
slider.value  # Should reflect the new value!
```

## Related Issues Filed

- https://github.com/nteract/elements/issues/62 - Registry dependencies don't resolve
- https://github.com/nteract/elements/issues/63 - Widget support RFC
- https://github.com/nteract/elements/issues/79 - components.json with registries breaks CLI