# Sidecar - Handoff

## Current State

The sidecar has full Jupyter output rendering and ipywidgets support via the `@nteract` shadcn registry. Build passes and tqdm progress bars render correctly.

### What's Working

1. **Output components from @nteract registry** - ANSI, HTML, Markdown, JSON, Image, SVG outputs
2. **MediaRouter + MediaProvider** - MIME-type dispatch with shared context for renderers
3. **Widget system from @nteract registry** - 50+ widget files in `src/components/widgets/`
4. **OutputWidget** - Captures outputs inside widget trees (`@jupyter-widgets/output`)
5. **tqdm progress bars** - Render correctly with proper layout (HBox + HTMLWidget + FloatProgress)
6. **Closed widget tracking** - `leave=False` progress bars disappear cleanly
7. **Widget debugger panel** - Sheet-based inspector at `src/components/widget-debugger.tsx`
8. **All ipywidget controls** - Sliders, buttons, text inputs, dropdowns, audio/video, gamepad controller

### Branch & PR

- **Branch:** `sidecar-with-elements`
- **PR:** #221 on runtimed/runtimed

## Documentation

**Full component documentation (LLM-friendly):**
- https://nteract-elements.vercel.app/llms-full.txt

This contains detailed API docs, props, examples for all components.

## Registry Setup

The `@nteract` registry is configured in `components.json`:

```json
{
  "registries": {
    "@nteract": "https://nteract-elements.vercel.app/r/{name}.json"
  }
}
```

Registry index:
- https://nteract-elements.vercel.app/r/registry.json

## Fresh Install

```bash
# 1. Init shadcn in your project
npx shadcn@latest init

# 2. Add @nteract registry to components.json
# "registries": { "@nteract": "https://nteract-elements.vercel.app/r/{name}.json" }

# 3. Install ALL components with one command
npx shadcn@latest add @nteract/all -yo

# 4. Build
npm run build
```

### Post-Install Fixes

If build fails with missing files, manually add from upstream:
- `button-style-utils.ts` - https://github.com/nteract/elements/blob/main/registry/widgets/controls/button-style-utils.ts
- `audio-widget.tsx` - https://github.com/nteract/elements/blob/main/registry/widgets/controls/audio-widget.tsx
- `video-widget.tsx` - https://github.com/nteract/elements/blob/main/registry/widgets/controls/video-widget.tsx

See issue [#115](https://github.com/nteract/elements/issues/115).

## Available from @nteract Registry

### Outputs (`outputs/`)
| Item | Description |
|------|-------------|
| `@nteract/media-router` | MIME-type dispatcher (pulls in all outputs) |
| `@nteract/media-provider` | Context for sharing renderers/priority/unsafe |
| `@nteract/ansi-output` | ANSI escape sequence rendering |
| `@nteract/html-output` | HTML with iframe sandbox for scripts |
| `@nteract/markdown-output` | GFM + math (KaTeX) + syntax highlighting |
| `@nteract/image-output` | Base64/URL image handling |
| `@nteract/svg-output` | Vector graphics |
| `@nteract/json-output` | Interactive tree view |

### Cell Components (`cell/`)
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

### Widgets (`widgets/`)
| Item | Description |
|------|-------------|
| `@nteract/widget-controls` | All 50+ ipywidget components |
| `@nteract/widget-view` | Universal widget router |
| `@nteract/widget-store` | React state management for widget models |
| `@nteract/anywidget-view` | ESM loader for anywidget |

### Editor (`editor/`)
| Item | Description |
|------|-------------|
| `@nteract/codemirror-editor` | CodeMirror 6 with Python, Markdown, SQL, etc. |

### Meta
| Item | Description |
|------|-------------|
| `@nteract/all` | Everything above in one install |

## File Structure

```
src/
├── components/
│   ├── cell/                  # @nteract cell components
│   │   ├── CellContainer.tsx
│   │   ├── CellControls.tsx
│   │   ├── CellHeader.tsx
│   │   ├── OutputArea.tsx
│   │   ├── PlayButton.tsx
│   │   └── ...
│   ├── editor/                # @nteract CodeMirror editor
│   │   ├── codemirror-editor.tsx
│   │   ├── extensions.ts
│   │   ├── languages.ts
│   │   └── themes.ts
│   ├── outputs/               # @nteract output renderers
│   │   ├── media-router.tsx
│   │   ├── media-provider.tsx
│   │   ├── ansi-output.tsx
│   │   ├── html-output.tsx
│   │   ├── json-output.tsx
│   │   ├── markdown-output.tsx
│   │   └── ...
│   ├── widgets/               # @nteract widget system
│   │   ├── controls/          # 50+ ipywidget components
│   │   │   ├── index.ts
│   │   │   ├── int-slider.tsx
│   │   │   ├── output-widget.tsx
│   │   │   └── ...
│   │   ├── widget-store.ts
│   │   ├── widget-store-context.tsx
│   │   ├── widget-view.tsx
│   │   ├── widget-registry.ts
│   │   └── anywidget-view.tsx
│   ├── ui/                    # @shadcn primitives (upstream)
│   └── widget-debugger.tsx    # Local: debug panel
├── App.tsx                    # Main app
└── types.ts                   # Jupyter message types
```

## Key Integration Points

### App.tsx imports:
```typescript
import "@/components/widgets/controls";  // Registers all widgets
import { WidgetStoreProvider, useWidgetStoreRequired } from "@/components/widgets/widget-store-context";
import { WidgetView } from "@/components/widgets/widget-view";
import { MediaRouter } from "@/components/outputs/media-router";
import { MediaProvider } from "@/components/outputs/media-provider";
import { AnsiStreamOutput, AnsiErrorOutput } from "@/components/outputs/ansi-output";
```

### Output rendering with MediaProvider:
```typescript
// Wrap app with providers
<WidgetStoreProvider sendMessage={sendToKernel}>
  <MediaProvider
    renderers={{
      "application/vnd.jupyter.widget-view+json": ({ data }) => {
        const { model_id } = data as { model_id: string };
        return <WidgetView modelId={model_id} />;
      },
    }}
  >
    {/* All nested MediaRouters inherit the widget renderer */}
    <OutputArea outputs={cell.outputs} />
  </MediaProvider>
</WidgetStoreProvider>
```

### Direct output rendering:
```typescript
// Widget outputs
const widgetData = output.data["application/vnd.jupyter.widget-view+json"];
if (widgetData?.model_id) {
  return <WidgetView modelId={widgetData.model_id} />;
}

// Standard outputs via MediaRouter
return <MediaRouter data={output.data} metadata={output.metadata} />;
```

## Known Issues Filed Upstream

| Issue | Description | Status |
|-------|-------------|--------|
| [#100](https://github.com/nteract/elements/issues/100) | Missing widget files in registry | Fixed |
| [#101](https://github.com/nteract/elements/issues/101) | Wrong import paths for shadcn primitives | Fixed |
| [#106](https://github.com/nteract/elements/issues/106) | json-output imports collapsible from wrong path | Fixed |
| [#115](https://github.com/nteract/elements/issues/115) | Missing files: audio-widget, video-widget, button-style-utils | Open - manually added |

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

# Output widget test
import ipywidgets as widgets
out = widgets.Output()
with out:
    print("Captured output!")
display(out)

# Audio/Video test
from IPython.display import Audio, Video
Audio(url="https://example.com/audio.mp3")
```

## Build

```bash
cd crates/sidecar/ui
npm run build
```

Current build size: ~747KB index.js (includes KaTeX for HTMLMath widget)

## Next Steps

1. **Integrate MediaProvider** - Consider wrapping the app with MediaProvider to share widget renderers
2. **Test OutputWidget** - Verify `widgets.Output()` captures work correctly
3. **Test Audio/Video widgets** - Verify media playback works
4. **Monitor #115** - Once fixed upstream, remove manually added files