# Sidecar Widget Integration - Handoff

## Current State

The sidecar now has full ipywidgets support via the `@nteract` shadcn registry. Build passes and tqdm progress bars render correctly.

### What's Working

1. **Widget system from @nteract registry** - 51 widget files installed to `src/components/widgets/`
2. **tqdm progress bars** - Render correctly with proper layout (HBox + HTMLWidget + FloatProgress)
3. **Closed widget tracking** - `leave=False` progress bars disappear cleanly (no "Loading widget...")
4. **Widget debugger panel** - Sheet-based inspector at `src/components/widget-debugger.tsx`
5. **All 44 ipywidget controls** - Sliders, buttons, text inputs, dropdowns, etc.

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

Install command:
```bash
pnpm dlx shadcn@latest registry add @nteract
npx shadcn@latest add @nteract/widget-controls -yo
```

## Known Issues Filed Upstream

| Issue | Description | Status |
|-------|-------------|--------|
| [nteract/elements#100](https://github.com/nteract/elements/issues/100) | Missing widget files in registry | Fixed |
| [nteract/elements#101](https://github.com/nteract/elements/issues/101) | Wrong import paths for shadcn primitives (`@/components/command` vs `@/components/ui/command`) | Open |

### Local Workaround for #101

Fixed imports in these files (will need to reapply after fresh install until upstream fixes):
- `combobox-widget.tsx` - command, popover imports
- `tags-input-widget.tsx` - badge import

```bash
cd src/components/widgets/controls
sed -i '' 's|from "@/components/command"|from "@/components/ui/command"|g' combobox-widget.tsx
sed -i '' 's|from "@/components/popover"|from "@/components/ui/popover"|g' combobox-widget.tsx
sed -i '' 's|from "@/components/badge"|from "@/components/ui/badge"|g' tags-input-widget.tsx
```

## Next Steps: Output Components

The sidecar currently has custom output components in `src/components/`:
- `ansi-output.tsx`
- `html-output.tsx`
- `image-output.tsx`
- `json-output.tsx`
- `markdown-output.tsx`
- `svg-output.tsx`
- `media-router.tsx`

**nteract/elements also provides output components** that should be evaluated:
- Check `@nteract/outputs` or similar registry items
- Compare with our local implementations
- Consider migrating to upstream versions for consistency

### To investigate:

```bash
# See what output-related items are in the nteract registry
curl -s https://nteract-elements.vercel.app/r/index.json | jq '.[] | select(.name | contains("output"))'
```

## File Structure

```
src/
├── components/
│   ├── ui/                    # shadcn primitives
│   ├── widgets/               # @nteract widget system
│   │   ├── controls/          # 44 ipywidget components
│   │   ├── widget-store.ts    # Model state management
│   │   ├── widget-view.tsx    # Universal widget renderer
│   │   └── ...
│   ├── widget-debugger.tsx    # Local: debug panel
│   ├── media-router.tsx       # Local: MIME type routing
│   ├── json-output.tsx        # Local: JSON viewer
│   └── ...                    # Other local outputs
├── App.tsx                    # Main app, imports widget system
└── types.ts                   # Jupyter message types
```

## Key Integration Points

### App.tsx imports:
```typescript
import "@/components/widgets/controls";  // Registers all widgets
import { WidgetStoreProvider, useWidgetStoreRequired } from "@/components/widgets/widget-store-context";
import { WidgetView } from "@/components/widgets/widget-view";
```

### Widget rendering in OutputCell:
```typescript
const widgetData = output.data["application/vnd.jupyter.widget-view+json"];
if (widgetData?.model_id) {
  return <WidgetView modelId={widgetData.model_id} />;
}
```

## tqdm Fix Summary

The key fixes for proper tqdm rendering (now upstream in nteract/elements):

1. **HBox**: `items-baseline gap-1` - matches JupyterLab's `.widget-inline-hbox`
2. **HTMLWidget**: `inline-flex shrink-0 items-baseline` - proper inline display
3. **FloatProgress/IntProgress**: `flex-1` expansion, no numeric readout (tqdm controls display via HTML widgets)
4. **Closed model tracking**: `wasModelClosed()` returns true for `comm_close`d widgets, `WidgetView` renders null

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