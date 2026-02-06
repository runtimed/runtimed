# Sidecar - Handoff

## Current State

The sidecar has full Jupyter output rendering and ipywidgets support via the `@nteract` shadcn registry. Build passes and all widgets work correctly, including third-party anywidget-based libraries.

### What's Working

1. **Output components** - ANSI, HTML, Markdown (with KaTeX math), JSON, Image, SVG
2. **MediaRouter + MediaProvider** - MIME-type dispatch with shared context for widget renderers
3. **Widget system** - 50+ widget components in `src/components/widgets/controls/`
4. **OutputWidget** - Captures outputs inside widget trees (`@jupyter-widgets/output`)
5. **tqdm progress bars** - Render correctly with proper layout (HBox + HTMLWidget + FloatProgress)
6. **Closed widget tracking** - `leave=False` progress bars disappear cleanly
7. **Widget debugger panel** - Sheet-based inspector at `src/components/widget-debugger.tsx`
8. **TimePicker** - Fixed upstream (#119)
9. **Audio/Video from_url()** - Fixed upstream (#120), binary data handled correctly
10. **jslink/jsdlink** - Store-layer implementation via `createLinkManager` (PR #127)
11. **ipycanvas** - Multi-canvas support via manager-as-dispatcher architecture (PR #134)
12. **anywidget ecosystem** - drawdata, quak, and other anywidget-based libraries work

### What's Pending

| Widget | Issue | Status |
|--------|-------|--------|
| **DatePicker** | ipywidgets uses `date` not `day` for day-of-month | [#125](https://github.com/nteract/elements/issues/125) open |
| **ipycanvas TS6133** | Unused `ctx` params in helper functions cause build errors with strict TS | Local fix applied, needs upstream |

## Third-Party Widget Compatibility

### ✅ Working (anywidget-based)

These use the anywidget protocol and work out of the box:

| Library | Install | Description |
|---------|---------|-------------|
| **ipycanvas** | `pip install ipycanvas` | HTML5 canvas drawing |
| **drawdata** | `pip install drawdata` | Draw data points for ML demos |
| **quak** | `pip install quak` | Interactive data tables |
| **lonboard** | `pip install lonboard geopandas` | Fast geospatial visualization |

```python
# ipycanvas
from ipycanvas import Canvas
canvas = Canvas(width=400, height=300)
canvas.fill_style = 'red'
canvas.fill_rect(50, 50, 100, 100)
canvas

# drawdata
from drawdata import ScatterWidget
ScatterWidget()

# quak
import quak
import pandas as pd
quak.Widget(pd.DataFrame({'a': [1,2,3], 'b': [4,5,6]}))
```

### ❌ Not Compatible (custom frontends)

These have their own JavaScript frontends that need separate bundling:

- **bqplot** - Custom D3-based frontend
- **ipyvolume** - Custom Three.js frontend
- **pythreejs** - Custom Three.js frontend
- **ipyleaflet** - Custom Leaflet frontend
- **ipycytoscape** - Custom Cytoscape frontend

These would require either:
1. Bundling their JS into the sidecar build
2. Waiting for them to migrate to anywidget

## Key Fixes

### ipycanvas Manager-as-Dispatcher (PR #134)

ipycanvas uses a singleton `CanvasManagerModel` that receives ALL drawing commands. The old approach had each `CanvasWidget` subscribe to the manager and filter commands via `activeCanvasRef` — but this caused interference when multiple canvases existed (e.g., animated canvas + canvases in tabs).

**New architecture (PR #134):**

1. **CanvasManagerWidget** subscribes to its own comm_id, parses `switchCanvas` commands, and re-emits messages to specific canvas comm_ids
2. **Each CanvasWidget** subscribes only to its own comm_id — completely isolated from other canvases

```typescript
// CanvasManagerWidget - dispatcher
useEffect(() => {
  const unsubscribe = store.subscribeToCustomMessage(modelId, (content, buffers) => {
    // Parse commands, find switchCanvas targets
    const targets = new Set<string>();
    collectSwitchCanvasTargets(commands, currentTargetRef, targets);
    
    // Route to current target if no switchCanvas in this message
    if (targets.size === 0 && currentTargetRef.current) {
      targets.add(currentTargetRef.current);
    }
    
    // Re-emit to each target canvas's comm_id
    for (const targetId of targets) {
      store.emitCustomMessage(targetId, content, rawBuffers);
    }
  });
  return unsubscribe;
}, [store, modelId]);

// CanvasWidget - isolated subscriber
useEffect(() => {
  const unsubscribe = store.subscribeToCustomMessage(modelId, (content, buffers) => {
    // Process commands - always active since manager only sends to us
    await processCommands(ctx, commands, dataBuffers, canvas, modelId, true);
  });
  return unsubscribe;
}, [store, modelId]);
```

**Benefits:**
- No shared routing state between canvases
- Animation on one canvas doesn't break others
- Tabs/accordions with canvases work correctly

Upstream: [PR #134](https://github.com/nteract/elements/pull/134)

### Local Fix: TS6133 Unused Parameters

The `ipycanvas-commands.ts` file has three helper functions (`drawRects`, `drawCircles`, `drawArcs`) with unused `ctx` parameters. These cause TS6133 errors with strict TypeScript configs.

**Local fix applied:**
```typescript
// Change from:
function drawRects(ctx: CanvasRenderingContext2D, ...)
// To:
function drawRects(_ctx: CanvasRenderingContext2D, ...)
```

This needs to be fixed upstream in nteract/elements.

### Store-Layer jslink (PR #127 - Merged)

The `createLinkManager` function manages `LinkModel` and `DirectionalLinkModel` at the store level.

**Benefits:**
- No component mounting required
- Works everywhere including iframes
- Store handles syncing without React lifecycle

**Key files (from @nteract registry):**
- `src/components/widgets/link-subscriptions.ts` - Core link manager
- `src/components/widgets/controls/link-widget.tsx` - Headless stub components
- `src/components/widgets/widget-store-context.tsx` - Integrates via `useEffect`

### Branch & PR

- **Branch:** `sidecar-with-elements`
- **PR:** #221 on runtimed/runtimed

## Documentation

**Full component documentation (LLM-friendly):**
- https://nteract-elements.vercel.app/llms-full.txt

## Registry Setup

The `@nteract` registry is configured in `components.json`:

```json
{
  "registries": {
    "@nteract": "https://nteract-elements.vercel.app/r/{name}.json"
  }
}
```

## Fresh Install

```bash
# 1. Init shadcn in your project
npx shadcn@latest init

# 2. Add @nteract registry to components.json
# "registries": { "@nteract": "https://nteract-elements.vercel.app/r/{name}.json" }

# 3. Install ALL components with one command
npx shadcn@latest add @nteract/all -yo

# 4. Optional: Add ipycanvas support
npx shadcn@latest add @nteract/ipycanvas -yo

# 5. Build (may need TS6133 fix - see below)
npm run build
```

**Note:** After installing ipycanvas, you may need to fix TS6133 errors in `ipycanvas-commands.ts` by prefixing unused `ctx` params with underscore in `drawRects`, `drawCircles`, and `drawArcs` functions.

## Key Integration Points

### App.tsx Structure

```typescript
import "@/components/widgets/controls";
import "@/components/widgets/ipycanvas";  // Optional: ipycanvas support
import { WidgetStoreProvider } from "@/components/widgets/widget-store-context";
import { WidgetView } from "@/components/widgets/widget-view";
import { MediaProvider } from "@/components/outputs/media-provider";

export default function App() {
  return (
    <WidgetStoreProvider sendMessage={sendToKernel}>
      <MediaProvider
        renderers={{
          "application/vnd.jupyter.widget-view+json": ({ data }) => {
            const { model_id } = data as { model_id: string };
            return <WidgetView modelId={model_id} />;
          },
        }}
      >
        <AppContent />
      </MediaProvider>
    </WidgetStoreProvider>
  );
}
```

Link subscriptions and custom message buffering are managed automatically by `WidgetStoreProvider`.

## File Structure

```
src/
├── components/
│   ├── cell/                  # @nteract cell components
│   ├── editor/                # @nteract CodeMirror editor
│   ├── outputs/               # @nteract output renderers
│   │   ├── media-router.tsx
│   │   ├── media-provider.tsx
│   │   └── ...
│   ├── widgets/               # @nteract widget system
│   │   ├── controls/          # 50+ ipywidget components
│   │   │   ├── index.ts
│   │   │   ├── link-widget.tsx
│   │   │   └── ...
│   │   ├── ipycanvas/         # ipycanvas support
│   │   │   ├── index.ts
│   │   │   ├── canvas-widget.tsx
│   │   │   └── ipycanvas-commands.ts
│   │   ├── link-subscriptions.ts
│   │   ├── widget-store.ts
│   │   ├── widget-store-context.tsx
│   │   ├── widget-view.tsx
│   │   └── anywidget-view.tsx
│   ├── ui/                    # @shadcn primitives
│   └── widget-debugger.tsx    # LOCAL: debug panel
├── App.tsx
└── types.ts
```

## Upstream Issues Tracker

| Issue | Description | Status |
|-------|-------------|--------|
| [#115](https://github.com/nteract/elements/issues/115) | Missing files: audio-widget, video-widget, button-style-utils | ✅ Fixed |
| [#118](https://github.com/nteract/elements/issues/118) | DatePicker crash - expects JS Date, gets object | ✅ Fixed |
| [#119](https://github.com/nteract/elements/issues/119) | TimePicker missing `milliseconds` field | ✅ Fixed |
| [#120](https://github.com/nteract/elements/issues/120) | Audio/Video crash with `from_url()` binary data | ✅ Fixed |
| [#121](https://github.com/nteract/elements/issues/121) | Missing LinkModel/DirectionalLinkModel | ✅ Fixed (PR #127) |
| [#125](https://github.com/nteract/elements/issues/125) | DatePicker uses `date` not `day` for day-of-month | 🔄 Open |
| [#129](https://github.com/nteract/elements/issues/129) | Buffer custom messages for unsubscribed comm_ids | ✅ Fixed (PR #130) |
| [#131](https://github.com/nteract/elements/issues/131) | ipycanvas multi-canvas buffering + isActive | ✅ Fixed (PR #132) |
| [#133](https://github.com/nteract/elements/issues/133) | Multiple canvases interfere via shared activeCanvasRef | ✅ Fixed (PR #134) |
| — | TS6133: unused `ctx` params in ipycanvas-commands.ts | 🔄 Needs upstream fix |

## Testing

```python
# tqdm test
from tqdm.auto import tqdm
import time

for filename in tqdm(["a.txt", "b.txt", "c.txt"]):
    for _ in tqdm(range(100), leave=False):
        time.sleep(0.01)

# jslink (bidirectional)
import ipywidgets as widgets
source = widgets.IntSlider(description='Source')
target = widgets.IntProgress(description='Target')
widgets.jslink((source, 'value'), (target, 'value'))
widgets.VBox([source, target])

# jsdlink (one-way)
a = widgets.IntSlider(description='A')
b = widgets.IntSlider(description='B (follows A)')
widgets.jsdlink((a, 'value'), (b, 'value'))
widgets.VBox([a, b])

# ipycanvas - single
from ipycanvas import Canvas
canvas = Canvas(width=400, height=300)
canvas.fill_style = 'red'
canvas.fill_rect(50, 50, 100, 100)
canvas

# ipycanvas - multi-canvas isolation test
import ipywidgets as widgets
import asyncio

c1 = Canvas(width=200, height=150)
c2 = Canvas(width=200, height=150)
c1.fill_style = 'red'
c1.fill_rect(25, 25, 150, 100)
c2.fill_style = 'blue'
c2.fill_rect(25, 25, 150, 100)
tabs = widgets.Tab(children=[c1, c2], titles=['Red', 'Blue'])
display(tabs)

# Animated canvas - should not break tabs above
anim = Canvas(width=300, height=100)
display(anim)
for i in range(60):
    anim.fill_style = f'hsl({i * 6}, 70%, 50%)'
    anim.fill_rect(0, 0, 300, 100)
    await asyncio.sleep(0.05)
# Tabs should still be clickable!

# anywidget - drawdata
from drawdata import ScatterWidget
ScatterWidget()

# anywidget - quak
import quak
import pandas as pd
quak.Widget(pd.DataFrame({'x': [1,2,3], 'y': [4,5,6]}))

# Output widget
out = widgets.Output()
with out:
    print("Captured output!")
display(out)
```

## Build

```bash
cd crates/sidecar/ui
npm run build
```

Current build size: ~765KB index.js (includes KaTeX + ipycanvas)

## Updating from Registry

```bash
# Pull latest widget controls
npx shadcn@latest add @nteract/widget-controls -yo

# Pull latest widget store
npx shadcn@latest add @nteract/widget-store -yo

# Pull latest ipycanvas (then apply TS6133 fix if needed)
npx shadcn@latest add @nteract/ipycanvas -yo

npm run build
```

## Next Steps

1. **Upstream TS6133 fix** - Get unused `ctx` param fix into nteract/elements ipycanvas-commands.ts
2. **Monitor #125** - DatePicker `date` vs `day` fix
3. **Test more anywidgets** - Validate other anywidget-based libraries
4. **Consider bundling popular widgets** - bqplot, ipyleaflet if demand exists