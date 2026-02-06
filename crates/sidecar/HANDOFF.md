# Sidecar - Handoff

## Current State

The sidecar has full Jupyter output rendering and ipywidgets support via the `@nteract` shadcn registry. Build passes and all widgets work correctly.

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
10. **jslink/jsdlink** - Store-layer implementation via `createLinkManager` (from PR #127 approach)

### What's Pending

| Widget | Issue | Status |
|--------|-------|--------|
| **DatePicker** | ipywidgets uses `date` not `day` for day-of-month | [#125](https://github.com/nteract/elements/issues/125) open |

### Store-Layer jslink (PR #127 - Merged)

PR #127 has been merged. The `createLinkManager` function in `link-subscriptions.ts` manages `LinkModel` and `DirectionalLinkModel` at the store level. Now pulled from official registry.

**Benefits:**
- No component mounting required
- Works everywhere including iframes
- Store handles syncing without React lifecycle
- `HeadlessWidgets` component no longer needed

**Key files (from @nteract registry):**
- `src/components/widgets/link-subscriptions.ts` - Core link manager
- `src/components/widgets/controls/link-widget.tsx` - Headless stub components
- `src/components/widgets/widget-store-context.tsx` - Integrates `createLinkManager` via `useEffect`

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

# 4. Build
npm run build
```

## Key Integration Points

### App.tsx Structure

```typescript
import "@/components/widgets/controls";
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

Link subscriptions are managed automatically by `WidgetStoreProvider` via `createLinkManager`.

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
│   │   │   ├── link-widget.tsx  # Headless stubs for registry
│   │   │   └── ...
│   │   ├── link-subscriptions.ts  # Store-layer jslink/jsdlink
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
| [#121](https://github.com/nteract/elements/issues/121) | Missing LinkModel/DirectionalLinkModel | ✅ Fixed (PR #127 merged) |
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

# jslink (store-layer implementation)
source = widgets.IntSlider(description='Source')
target = widgets.IntProgress(description='Target')
widgets.jslink((source, 'value'), (target, 'value'))
widgets.VBox([source, target])

# jsdlink (one-way)
a = widgets.IntSlider(description='A')
b = widgets.IntSlider(description='B (follows A)')
widgets.jsdlink((a, 'value'), (b, 'value'))
widgets.VBox([a, b])

# DatePicker (pending #125 - changing value crashes kernel)
from datetime import date
widgets.DatePicker(value=date.today(), description='Date:')

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
# Pull latest widget controls (includes link-widget.tsx)
npx shadcn@latest add @nteract/widget-controls -yo

# Pull latest widget store (includes link-subscriptions.ts)
npx shadcn@latest add @nteract/widget-store -yo

npm run build
```

## Next Steps

1. **Monitor #125** - DatePicker `date` vs `day` fix
2. **Test edge cases** - Verify links work in complex widget trees (accordions, tabs, etc.)