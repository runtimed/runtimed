# Sidecar Widget Implementation - Handoff

## Current State

We've integrated [nteract elements](https://nteract-elements.vercel.app/) into sidecar for rendering Jupyter outputs and widgets. The UI is built with Vite + React + Tailwind and embedded into the Rust binary via `rust-embed`.

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

3. **Built-in ipywidgets** ✅ - Two-way binding works:
   - IntSlider, FloatSlider
   - IntProgress, FloatProgress
   - Button, Checkbox
   - Text, Textarea
   - Dropdown, RadioButtons, etc.
   - All 19 widget controls from nteract elements

4. **anywidget Loading** ✅ - ESM modules load and execute:
   - Factory pattern (`export default () => ({ render })`) supported
   - CSS injection works
   - Initialize lifecycle runs
   - Render is called

### What's NOT Working

**anywidget Custom Messages (quak)** ❌

quak (and similar data widgets) use `model.send()` to request data from the kernel. The flow breaks:

1. quak's `render()` calls `model.send({ type: "arrow", sql: "...", uuid: "..." })`
2. Our code sends `comm_msg` to kernel with this content
3. Kernel receives it (status busy→idle shows it processed)
4. **Kernel sends NO `comm_msg` response back** ← THE ISSUE
5. quak waits forever for data via `on("msg:custom")` callback

**The Protocol Problem:**

ipywidgets expects ALL `comm_msg.data` to have a `method` field:
```python
# ipywidgets/widgets/widget.py:766
method = data['method']  # KeyError if missing!
```

But anywidget's `model.send()` is supposed to pass content directly without wrapping.

When we include `method: "custom"`:
- ipywidgets receives it but quak's kernel-side handler doesn't respond

When we omit `method`:
- ipywidgets throws `KeyError: 'method'`

**The main branch works** because it uses `@jupyter-widgets/html-manager` which has the full Backbone.js model system and Comm class that handles this correctly.

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
| `ui/src/lib/widget-store.ts` | Widget model state management (from nteract elements) |
| `ui/src/lib/widget-store-context.tsx` | React context and hooks |
| `ui/src/lib/use-comm-router.ts` | Jupyter comm protocol handling |
| `ui/src/components/widgets/anywidget-view.tsx` | anywidget ESM loader + AFM interface |
| `ui/src/components/widgets/widget-view.tsx` | Routes to correct widget renderer |
| `ui/src/components/widgets/controls/` | All 19 built-in widget components |
| `ui/src/App.tsx` | Main app, provides `sendMessage` to provider |
| `src/main.rs` | Rust: IOPub listener, Shell sender, Webview |

## Debugging anywidgets

We added extensive logging. Key log prefixes:
- `[sidecar]` - Message routing in App.tsx
- `[anywidget]` - ESM loading and lifecycle
- `[AFM]` - AnyWidget Frontend Module proxy (get/set/send/on)

Example debug flow for quak:
```
[anywidget] Effect running: hasModel=true, hasEsm=true
[anywidget] Loading ESM module...
[anywidget] ESM loaded: defaultType=function, hasRender=false (factory pattern)
[anywidget] Calling factory function...
[anywidget] Factory returned: hasRender=true, hasInitialize=true
[anywidget] Calling initialize...
[AFM] on: msg:custom
[anywidget] Calling render...
[AFM] get: _table_name = "df"
[AFM] get: _columns = Array(12)
[AFM] send - full content: {"type":"arrow","sql":"SELECT...","uuid":"..."}
[AFM] send - to comm_id: 53ddf155922248f480756b06ee03fb6c
[sidecar] Received message: status (busy)
[sidecar] Received message: status (idle)
[WARNING] render() has been running for 5+ seconds - waiting for data
```

## Next Steps - Investigate html-manager

The main branch uses `@jupyter-widgets/html-manager` which works with quak. Need to understand:

1. **How does html-manager's Comm class send custom messages?**
   - Check `main:crates/sidecar/src/static/main.js` - the `Comm.send()` method
   - Does it wrap with `method` or not?

2. **How does anywidget's kernel-side handle messages?**
   - Look at anywidget Python package source
   - What format does it expect for custom messages?

3. **Is there a different comm protocol for anywidget vs ipywidgets?**
   - anywidget might use a raw comm without the ipywidgets method convention

4. **Consider hybrid approach:**
   - Use nteract elements for UI components
   - Use `@jupyter-widgets/html-manager` just for comm handling
   - Or extract just the Comm class logic

## Build Commands

```bash
# Build UI
cd crates/sidecar/ui
npm install
npm run build

# Build Rust (debug)
cargo build -p sidecar

# Run with info logging
RUST_LOG=info ./target/debug/sidecar /path/to/connection.json

# Run with message dump
./target/debug/sidecar --dump messages.jsonl /path/to/connection.json
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

Test ipywidgets (works):
```python
import ipywidgets as widgets
slider = widgets.IntSlider(value=50, min=0, max=100, description='Test:')
slider.observe(lambda change: print(f"Value: {change['new']}"), names='value')
display(slider)
```

Test quak (broken - hangs waiting for data):
```python
import polars as pl
import quak
df = pl.read_parquet("https://github.com/uwdata/mosaic/raw/main/data/athletes.parquet")
quak.Widget(df)
```

## Issues to Report to nteract/elements

1. **`--overwrite` flag doesn't work** - `npx shadcn add ... --overwrite` says "Skipped"
2. **Unused import** - `widget-store-context.tsx` imports `JupyterMessageHeader` but doesn't use it (TypeScript error)
3. **Relative import paths** - `anywidget-view.tsx` uses `./widget-store-context` but shadcn installs to different location

## Related Issues

- https://github.com/nteract/elements/issues/62 - Registry dependencies don't resolve
- https://github.com/nteract/elements/issues/63 - Widget support RFC
- https://github.com/nteract/elements/issues/79 - components.json with registries breaks CLI
- https://github.com/nteract/elements/pull/84 - Fixes from our feedback (merged)