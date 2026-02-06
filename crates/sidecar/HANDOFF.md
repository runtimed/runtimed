# Sidecar - Handoff

## Current State

The sidecar has full Jupyter output rendering and ipywidgets support via the `@nteract` shadcn registry. Build passes and most widgets work correctly.

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
10. **jslink/jsdlink** - Working via local LinkWidget + HeadlessWidgets component

### What's Pending

| Widget | Issue | Status |
|--------|-------|--------|
| **DatePicker** | ipywidgets uses `date` not `day` for day-of-month | [#125](https://github.com/nteract/elements/issues/125) open |
| **jslink/link upstream** | Need LinkModel/DirectionalLinkModel in registry | [#121](https://github.com/nteract/elements/issues/121) / PR #127 pending |

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

### Post-Install: Add Link Widgets (until PR #127 merges)

1. Copy `link-widget.tsx` from this repo to `src/components/widgets/controls/`
2. Add to `controls/index.ts`:
   ```typescript
   import { DirectionalLinkWidget, LinkWidget } from "./link-widget";
   registerWidget("LinkModel", LinkWidget);
   registerWidget("DirectionalLinkModel", DirectionalLinkWidget);
   ```

## Key Integration Points

### App.tsx Structure

```typescript
import "@/components/widgets/controls";  // Registers all widgets
import { WidgetStoreProvider, useWidgetModels } from "@/components/widgets/widget-store-context";
import { WidgetView } from "@/components/widgets/widget-view";
import { getWidgetComponent } from "@/components/widgets/widget-registry";
import { MediaRouter } from "@/components/outputs/media-router";
import { MediaProvider } from "@/components/outputs/media-provider";

// HeadlessWidgets - REQUIRED for jslink/jsdlink to work
// Mounts widgets with _view_name: null so their useEffect hooks run
function HeadlessWidgets() {
  const models = useWidgetModels();
  
  const headlessIds: string[] = [];
  models.forEach((model, id) => {
    if (model.state._view_name === null && getWidgetComponent(model.modelName)) {
      headlessIds.push(id);
    }
  });

  return (
    <>
      {headlessIds.map((id) => (
        <WidgetView key={id} modelId={id} />
      ))}
    </>
  );
}

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
        <HeadlessWidgets />  {/* Must be inside providers */}
        <AppContent />
      </MediaProvider>
    </WidgetStoreProvider>
  );
}
```

**Important:** The `HeadlessWidgets` component is required because `LinkModel` has `_view_name: null` and isn't a child of any container. Without mounting it, the React `useEffect` that sets up property sync never runs.

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
| [#121](https://github.com/nteract/elements/issues/121) | Missing LinkModel/DirectionalLinkModel | 🔄 PR #127 |
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

# jslink (working with HeadlessWidgets)
source = widgets.IntSlider(description='Source')
target = widgets.IntProgress(description='Target')
widgets.jslink((source, 'value'), (target, 'value'))
widgets.VBox([source, target])

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
# Pull latest fixes
npx shadcn@latest add @nteract/widget-controls -yo

# Re-add link widgets to index.ts (until #127 merges)
# The registry update overwrites index.ts

npm run build
```

## Next Steps

1. **Monitor #125** - DatePicker `date` vs `day` fix
2. **Monitor PR #127** - Once merged, update from registry and remove local link-widget.tsx
3. **Document HeadlessWidgets pattern** - Ensure PR #127 docs mention this requirement