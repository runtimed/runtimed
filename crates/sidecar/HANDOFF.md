# Sidecar - Handoff

## Current State

The sidecar has full Jupyter output rendering and ipywidgets support via the `@nteract` shadcn registry. Build passes and tqdm progress bars render correctly.

### What's Working

1. **Output components from @nteract registry** - ANSI, HTML, Markdown, JSON, Image, SVG outputs
2. **Media router** - MIME-type based output dispatch with priority ordering
3. **Widget system from @nteract registry** - 47+ widget files in `src/components/widgets/`
4. **tqdm progress bars** - Render correctly with proper layout (HBox + HTMLWidget + FloatProgress)
5. **Closed widget tracking** - `leave=False` progress bars disappear cleanly
6. **Widget debugger panel** - Sheet-based inspector at `src/components/widget-debugger.tsx`
7. **All ipywidget controls** - Sliders, buttons, text inputs, dropdowns, gamepad controller, etc.

### Branch & PR

- **Branch:** `sidecar-with-elements`
- **PR:** #221 on runtimed/runtimed

## Registry Setup

The `@nteract` registry is configured in `components.json`:

```json
{
  "registries": {
    "@nteract": "https://nteract-elements.vercel.app/r/{name}.json"
  }
}
```

Registry index available at:
- https://nteract-elements.vercel.app/r/registry.json

## Bootstrapping a New Project

The simplest way to get everything:

```bash
# 1. Init shadcn in your project
npx shadcn@latest init

# 2. Add @nteract registry to components.json
# "registries": { "@nteract": "https://nteract-elements.vercel.app/r/{name}.json" }

# 3. Install ALL components with one command
npx shadcn@latest add @nteract/all -yo
```

This installs:
- All output renderers (ANSI, HTML, Markdown, JSON, Image, SVG)
- Media router for MIME-type dispatch
- Cell UI components (container, header, controls, execution count/status)
- All ipywidgets (47+ controls including gamepad)
- CodeMirror 6 editor with Python, Markdown, SQL support
- Collaboration components (avatars, presence bookmarks)

### À la carte installation

If you only need specific pieces:

```bash
npx shadcn@latest add @nteract/media-router -yo      # All output renderers
npx shadcn@latest add @nteract/output-area -yo       # Output wrapper
npx shadcn@latest add @nteract/widget-controls -yo   # All ipywidgets
npx shadcn@latest add @nteract/codemirror-editor -yo # Code editing
npx shadcn@latest add @nteract/cell-container -yo    # Cell UI
npx shadcn@latest add @nteract/cell-controls -yo     # Cell actions
```

## Available from @nteract Registry

### Outputs
| Item | Description |
|------|-------------|
| `@nteract/media-router` | MIME-type dispatcher (pulls in all outputs below) |
| `@nteract/ansi-output` | ANSI escape sequence rendering |
| `@nteract/html-output` | HTML with iframe sandbox for scripts |
| `@nteract/markdown-output` | GFM + math (KaTeX) + syntax highlighting |
| `@nteract/image-output` | Base64/URL image handling |
| `@nteract/svg-output` | Vector graphics |
| `@nteract/json-output` | Interactive tree view |

### Cell Components
| Item | Description |
|------|-------------|
| `@nteract/output-area` | Output wrapper with collapse/scroll |
| `@nteract/cell-container` | Focus and selection wrapper |
| `@nteract/cell-header` | Slot-based header layout |
| `@nteract/cell-controls` | Cell action menu |
| `@nteract/execution-count` | Classic `[n]:` indicator |
| `@nteract/execution-status` | Queued/running/error badges |
| `@nteract/play-button` | Run/stop cell button |
| `@nteract/runtime-health-indicator` | Kernel status dot |
| `@nteract/cell-type-selector` | Cell type dropdown |
| `@nteract/collaborator-avatars` | Collaboration avatars |
| `@nteract/presence-bookmarks` | User presence indicators |

### Widgets
| Item | Description |
|------|-------------|
| `@nteract/widget-controls` | All ipywidget components |
| `@nteract/widget-view` | Universal widget router |
| `@nteract/widget-store` | React state management for widget models |
| `@nteract/anywidget-view` | ESM loader for anywidget |

### Editor
| Item | Description |
|------|-------------|
| `@nteract/codemirror-editor` | CodeMirror 6 with Python, Markdown, SQL, etc. |

### Meta
| Item | Description |
|------|-------------|
| `@nteract/all` | Everything above in one install |

## Known Issues Filed Upstream

| Issue | Description | Status |
|-------|-------------|--------|
| [nteract/elements#100](https://github.com/nteract/elements/issues/100) | Missing widget files in registry | Fixed |
| [nteract/elements#101](https://github.com/nteract/elements/issues/101) | Wrong import paths for shadcn primitives in widgets | Fixed |
| [nteract/elements#106](https://github.com/nteract/elements/issues/106) | json-output imports collapsible from wrong path | Fixed |

## File Structure

```
src/
├── components/
│   ├── ui/                    # shadcn primitives
│   ├── widgets/               # @nteract widget system
│   │   ├── controls/          # ipywidget components
│   │   ├── widget-store.ts    # Model state management
│   │   ├── widget-view.tsx    # Universal widget renderer
│   │   └── ...
│   ├── widget-debugger.tsx    # Local: debug panel
│   ├── media-router.tsx       # @nteract: MIME type routing
│   ├── ansi-output.tsx        # @nteract: ANSI rendering
│   ├── html-output.tsx        # @nteract: HTML rendering
│   ├── image-output.tsx       # @nteract: Image rendering
│   ├── json-output.tsx        # @nteract: JSON tree view
│   ├── markdown-output.tsx    # @nteract: Markdown rendering
│   └── svg-output.tsx         # @nteract: SVG rendering
├── App.tsx                    # Main app
└── types.ts                   # Jupyter message types
```

## Key Integration Points

### App.tsx imports:
```typescript
import "@/components/widgets/controls";  // Registers all widgets
import { WidgetStoreProvider, useWidgetStoreRequired } from "@/components/widgets/widget-store-context";
import { WidgetView } from "@/components/widgets/widget-view";
import { MediaRouter } from "@/components/media-router";
```

### Output rendering:
```typescript
// Widget outputs
const widgetData = output.data["application/vnd.jupyter.widget-view+json"];
if (widgetData?.model_id) {
  return <WidgetView modelId={widgetData.model_id} />;
}

// Standard outputs via MediaRouter
return <MediaRouter data={output.data} metadata={output.metadata} />;
```

## tqdm Fix Summary

Key fixes for proper tqdm rendering (now upstream in nteract/elements):

1. **HBox**: `items-baseline gap-1` - matches JupyterLab's `.widget-inline-hbox`
2. **HTMLWidget**: `inline-flex shrink-0 items-baseline` - proper inline display
3. **FloatProgress/IntProgress**: `flex-1` expansion, no numeric readout
4. **Closed model tracking**: `wasModelClosed()` returns true for `comm_close`d widgets

## Testing

```python
# tqdm test
from tqdm.auto import tqdm
import time

for filename in tqdm(["a.txt", "b.txt", "c.txt"]):
    for _ in tqdm(range(100), leave=False):
        time.sleep(0.01)
```

Expected: Inner bars disappear when complete, outer bar shows progress correctly.

## Build

```bash
cd crates/sidecar/ui
npm run build
```

Current build size: ~740KB index.js (includes KaTeX for HTMLMath widget)