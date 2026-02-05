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

### What's NOT Working

**Two-way binding** - When you drag the IntSlider in the UI, the value doesn't sync back to the kernel. The slider moves visually but the kernel's widget state doesn't update.

## The Problem

The `IntSlider` component calls `sendUpdate(modelId, { value: newValue })` when the user interacts with it, but this message isn't making it back to the kernel.

### Relevant Code Flow

1. **User drags slider** → `IntSlider.handleChange()` is called
   - File: `ui/src/components/widgets/controls/int-slider.tsx`

2. **`sendUpdate` is called** from the widget store context
   - File: `ui/src/components/widgets/widget-store-context.tsx`
   - This comes from `useCommRouter` hook

3. **`useCommRouter.sendUpdate`** constructs a `comm_msg` and calls `sendMessage`
   - File: `ui/src/components/widgets/use-comm-router.ts`

4. **`sendMessage`** is passed into `WidgetStoreProvider` from `App.tsx`
   - Currently it does: `fetch("/message", { method: "POST", body: JSON.stringify(msg) })`

5. **Rust receives the POST** at `/message` endpoint
   - File: `src/main.rs` in the `with_asynchronous_custom_protocol` handler
   - It deserializes the message and sends it via the shell channel

### Likely Issues to Investigate

1. **Message Format** - The `comm_msg` constructed by `useCommRouter.sendUpdate` may not match what the kernel expects. Check:
   - Is `header.msg_type` set to `"comm_msg"`?
   - Is `content.data.method` set to `"update"`?
   - Is `content.data.state` properly structured?
   - Are `buffer_paths` needed for any values?

2. **Rust Serialization** - The `WryJupyterMessage` struct may not serialize the outgoing message correctly for the kernel.

3. **Shell Channel** - Verify the message is actually being sent on the shell channel and the kernel receives it.

4. **Parent Header** - Widget messages need proper `parent_header` to correlate with the execution context.

### Debugging Tips

1. **Add logging in Rust** - In `main.rs` where `/message` POST is handled, log the received message:
   ```rust
   debug!("Received message from frontend: {:?}", wry_message);
   ```

2. **Add logging in JS** - In `use-comm-router.ts`, log outgoing messages:
   ```typescript
   console.log("[comm] sendUpdate:", commId, state);
   ```

3. **Use `--dump` flag** - Run sidecar with `--dump messages.jsonl` to capture all iopub messages. Compare the structure of incoming `comm_msg` from the kernel with what you're trying to send back.

4. **Test with Python** - In the kernel, add a callback to see if updates arrive:
   ```python
   slider = widgets.IntSlider()
   slider.observe(lambda change: print(f"Value changed: {change}"), names='value')
   display(slider)
   ```

## Files to Focus On

| File | Purpose |
|------|---------|
| `ui/src/components/widgets/use-comm-router.ts` | Constructs outgoing comm messages |
| `ui/src/components/widgets/controls/int-slider.tsx` | Calls `sendUpdate` on change |
| `ui/src/App.tsx` | Provides `sendMessage` to the provider |
| `src/main.rs` | Handles `/message` POST, sends to kernel |

## Message Format Reference

A `comm_msg` for updating widget state should look like:

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
  "parent_header": { ... },
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

## Related Issues Filed

- https://github.com/nteract/elements/issues/62 - Registry dependencies don't resolve
- https://github.com/nteract/elements/issues/63 - Widget support RFC
- https://github.com/nteract/elements/issues/79 - components.json with registries breaks CLI

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
