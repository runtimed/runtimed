# nteract-elements Widget Fixes

This document describes two fixes needed in the nteract-elements registry for full anywidget compatibility, discovered while implementing widget support in the runtimed sidecar.

## Summary

| Issue | Impact | Severity |
|-------|--------|----------|
| Buffers should be `DataView[]` not `ArrayBuffer[]` | Binary data widgets (quak, etc.) fail to decode Arrow IPC | **Critical** |
| `send()` should wrap content, not spread it | Custom messages malformed for ipywidgets protocol | **Critical** |

---

## Issue 1: Buffer Type Mismatch

### Problem

JupyterLab services deserializes binary message buffers as `DataView[]`, not `ArrayBuffer[]`. Anywidgets like quak access the underlying ArrayBuffer via the `.buffer` property:

```typescript
// quak widget.ts - how anywidgets access binary data
model.on("msg:custom", (msg, buffers) => {
  const buffer = buffers[0].buffer;  // DataView.buffer → ArrayBuffer
  const table = decodeIPC(buffer);
});
```

If buffers are `ArrayBuffer[]` instead of `DataView[]`, `buffers[0].buffer` returns `undefined` and the widget fails silently.

### Fix Location

**File:** `registry/widgets/anywidget-view.tsx` (and anywhere buffers are handled)

The buffer decoding (likely in the message handler or context) needs to produce `DataView[]`:

```typescript
// BEFORE (wrong):
if (typeof b64 === "string") {
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes.buffer;  // ❌ Returns ArrayBuffer
}

// AFTER (correct):
if (typeof b64 === "string") {
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return new DataView(bytes.buffer);  // ✅ Returns DataView
}
```

### Type Updates Needed

```typescript
// Add buffer type alias
export type BufferType = ArrayBuffer | DataView;

// Update CustomMessageCallback
type CustomMessageCallback = (
  content: Record<string, unknown>,
  buffers?: DataView[]  // Changed from ArrayBuffer[]
) => void;
```

When emitting custom messages to widget handlers, ensure buffers are `DataView[]`:

```typescript
// In emitCustomMessage or equivalent:
const dataViewBuffers = buffers?.map((b) =>
  b instanceof DataView ? b : new DataView(b)
);
callbacks.forEach((cb) => cb(content, dataViewBuffers));
```

---

## Issue 2: Custom Message Content Wrapping

### Problem

The current `send()` implementation in `anywidget-view.tsx` spreads content directly into the data object:

```typescript
// CURRENT (wrong):
send(content, _callbacks, buffers): void {
  sendMessage({
    content: {
      comm_id: model.id,
      data: {
        method: "custom",
        ...content,           // ❌ Spreads content into data
        buffer_paths: [],
      },
    },
    // ...
  });
}
```

This produces:
```json
{ "method": "custom", "type": "arrow", "sql": "...", "uuid": "..." }
```

But ipywidgets Python expects `content` to be nested:

```python
# ipywidgets/widgets/widget.py
def _handle_msg(self, msg):
    data = msg['content']['data']
    method = data['method']
    if method == 'custom':
        if 'content' in data:
            self._handle_custom_msg(data['content'], msg['buffers'])
            #                       ^^^^^^^^^^^^^^ Expects nested content!
```

### Fix

```typescript
// CORRECT:
send(
  content: Record<string, unknown>,
  _callbacks?: Record<string, unknown>,
  buffers?: ArrayBuffer[],
): void {
  sendMessage({
    header: createHeader("comm_msg"),
    parent_header: null,
    metadata: {},
    content: {
      comm_id: model.id,
      data: {
        method: "custom",
        content: content,     // ✅ Wraps content properly
      },
    },
    buffers: buffers ?? [],
    channel: "shell",
  });
}
```

This produces:
```json
{ "method": "custom", "content": { "type": "arrow", "sql": "...", "uuid": "..." } }
```

---

## Verification

After applying these fixes, test with quak:

```python
import polars as pl
import quak

df = pl.read_parquet("https://github.com/uwdata/mosaic/raw/main/data/athletes.parquet")
quak.Widget(df)
```

Expected behavior:
1. Widget loads and calls `initialize()`
2. Widget's `render()` executes
3. `model.send()` dispatches SQL queries to kernel
4. Kernel responds with `comm_msg` containing Arrow IPC buffers
5. Widget receives buffers via `model.on("msg:custom")` callback
6. Data table renders with histograms and filters

Console logs should show:
```
[AFM] send - full content: {"type":"arrow","sql":"SELECT...","uuid":"..."}
[sidecar] comm_msg details: {hasBuffers: true, bufferCount: 1, method: "custom"}
```

---

## References

- [JupyterLab services serialize.ts](https://github.com/jupyterlab/jupyterlab/blob/main/packages/services/src/kernel/serialize.ts) - Shows buffers are `DataView[]`
- [ipywidgets widget.py](https://github.com/jupyter-widgets/ipywidgets/blob/main/python/ipywidgets/ipywidgets/widgets/widget.py#L766) - Shows `data['content']` extraction
- [anywidget AFM spec](https://anywidget.dev/en/afm/) - Frontend module interface
- [quak widget.ts](https://github.com/manzt/quak/blob/main/lib/widget.ts) - Example anywidget using binary buffers