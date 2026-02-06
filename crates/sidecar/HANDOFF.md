# Sidecar - Handoff

## Current State

The sidecar has full Jupyter output rendering and ipywidgets support via the `@nteract` shadcn registry. Build passes and most widgets render correctly.

### What's Working

1. **Output components from @nteract registry** - ANSI, HTML, Markdown, JSON, Image, SVG outputs
2. **MediaRouter + MediaProvider** - MIME-type dispatch with shared context for widget renderers
3. **Widget system from @nteract registry** - 50+ widget files in `src/components/widgets/`
4. **OutputWidget** - Captures outputs inside widget trees (`@jupyter-widgets/output`)
5. **tqdm progress bars** - Render correctly with proper layout (HBox + HTMLWidget + FloatProgress)
6. **Closed widget tracking** - `leave=False` progress bars disappear cleanly
7. **Widget debugger panel** - Sheet-based inspector at `src/components/widget-debugger.tsx`
8. **TimePicker** - Fixed upstream (#119)
9. **Audio/Video from_url()** - Fixed upstream (#120), binary data now handled correctly
10. **Link widgets (jslink/link)** - Local implementation in `link-widget.tsx`

### What's Pending

| Widget | Issue | Status |
|--------|-------|--------|
| **DatePicker** | ipywidgets uses `date` not `day` for day-of-month | [#125](https://github.com/nteract/elements/issues/125) open |
| **jslink/link** | Need LinkModel/DirectionalLinkModel upstream | [#121](https://github.com/nteract/elements/issues/121) open |

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

### Post-Install: Add Link Widgets

The registry doesn't include Link widgets yet. After install, add them manually:

1. Copy `link-widget.tsx` from this repo to `src/components/widgets/controls/`
2. Add to `controls/index.ts`:
   ```typescript
   import { DirectionalLinkWidget, LinkWidget } from "./link-widget";
   registerWidget("DirectionalLinkModel", DirectionalLinkWidget);
   registerWidget("LinkModel", LinkWidget);
   ```

## Key Integration Points

### App.tsx Structure

```typescript
import "@/components/widgets/controls";  // Registers all widgets
import { WidgetStoreProvider } from "@/components/widgets/widget-store-context";
import { WidgetView } from "@/components/widgets/widget-view";
import { MediaRouter } from "@/components/outputs/media-router";
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

This pattern injects the widget renderer at the top level, so all nested `MediaRouter` instances (in OutputArea, OutputWidget, etc.) automatically inherit it.

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

### Widgets (`widgets/`)
| Item | Description |
|------|-------------|
| `@nteract/widget-controls` | All 50+ ipywidget components |
| `@nteract/widget-view` | Universal widget router |
| `@nteract/widget-store` | React state management for widget models |
| `@nteract/anywidget-view` | ESM loader for anywidget |

### Cell Components (`cell/`)
| Item | Description |
|------|-------------|
| `@nteract/output-area` | Output wrapper with collapse/scroll |
| `@nteract/cell-container` | Focus and selection wrapper |
| `@nteract/play-button` | Run/stop cell button |
| `@nteract/execution-count` | Classic `[n]:` indicator |

### Editor (`editor/`)
| Item | Description |
|------|-------------|
| `@nteract/codemirror-editor` | CodeMirror 6 with Python, Markdown, SQL, etc. |

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
│   │   │   ├── link-widget.tsx  # LOCAL: jslink/link support
│   │   │   └── ...
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
| [#121](https://github.com/nteract/elements/issues/121) | Missing LinkModel/DirectionalLinkModel | 🔄 Open |
| [#125](https://github.com/nteract/elements/issues/125) | DatePicker uses `date` not `day` for day-of-month | 🔄 Open |

## Testing

```python
# tqdm test
from tqdm.auto import tqdm
import time

for filename in tqdm(["a.txt", "b.txt", "c.txt"]):
    for _ in tqdm(range(100), leave=False):
        time.sleep(0.01)

# TimePicker (fixed)
import ipywidgets as widgets
from datetime import time
widgets.TimePicker(value=time(12, 30), description='Time:')

# Audio/Video from URL (fixed)
widgets.Audio.from_url('https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3')
widgets.Video.from_url('https://www.w3schools.com/html/mov_bbb.mp4', width=320)

# DatePicker (pending #125)
from datetime import date
widgets.DatePicker(value=date.today(), description='Date:')

# jslink (local implementation, pending #121)
source = widgets.IntSlider(description='Source')
target = widgets.IntProgress(description='Target')
widgets.jslink((source, 'value'), (target, 'value'))
widgets.VBox([source, target])

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

Current build size: ~753KB index.js (includes KaTeX for HTMLMath widget)

## Updating from Registry

```bash
# Pull latest fixes
npx shadcn@latest add @nteract/widget-controls -yo

# Re-add link widgets (until #121 is merged)
# Edit controls/index.ts to import and register LinkWidget, DirectionalLinkWidget

npm run build
```

## Next Steps

1. **Monitor #125** - DatePicker `date` vs `day` fix
2. **Monitor #121** - LinkModel/DirectionalLinkModel upstream
3. **Remove local link-widget.tsx** once #121 is merged and update from registry