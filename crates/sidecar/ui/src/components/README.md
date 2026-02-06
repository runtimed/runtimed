# Components

This directory contains React components from multiple sources, organized by namespace.

## Directory Structure

```
components/
├── cell/             # @nteract cell components
├── editor/           # @nteract CodeMirror editor
├── outputs/          # @nteract output renderers + MediaProvider
├── widgets/          # @nteract widget system (50+ controls)
├── ui/               # @shadcn primitives (upstream)
├── widget-debugger.tsx  # Local (sidecar-specific)
└── README.md
```

## Local (sidecar-specific)

- `widget-debugger.tsx` - Debug panel for inspecting widget state

## From `@nteract` Registry

Installed via `npx shadcn@latest add @nteract/all -yo`

### `outputs/` - Output Renderers
- `ansi-output.tsx` - ANSI escape sequence rendering
- `html-output.tsx` - HTML with iframe sandbox
- `image-output.tsx` - Base64/URL images
- `json-output.tsx` - Interactive JSON tree
- `markdown-output.tsx` - GFM + math (KaTeX) + syntax highlighting
- `svg-output.tsx` - Vector graphics
- `media-router.tsx` - MIME-type dispatch
- `media-provider.tsx` - Context for sharing renderers/priority/unsafe across nested MediaRouters

### `cell/` - Cell Components
- `CellContainer.tsx` - Focus/selection wrapper
- `CellControls.tsx` - Cell action menu
- `CellHeader.tsx` - Header layout
- `CellTypeButton.tsx` - Cell type buttons
- `CellTypeSelector.tsx` - Cell type dropdown
- `CollaboratorAvatars.tsx` - User avatars
- `ExecutionCount.tsx` - `[n]:` indicator
- `ExecutionStatus.tsx` - Status badges
- `OutputArea.tsx` - Output wrapper
- `PlayButton.tsx` - Run/stop button
- `PresenceBookmarks.tsx` - Presence indicators
- `RuntimeHealthIndicator.tsx` - Kernel status

### `editor/` - CodeMirror Editor
- `codemirror-editor.tsx` - Main editor component
- `extensions.ts` - Editor extensions
- `languages.ts` - Language support
- `themes.ts` - Editor themes
- `index.ts` - Exports

### `widgets/` - Jupyter Widgets
- `widget-store.ts` - State management
- `widget-store-context.tsx` - React context
- `widget-view.tsx` - Widget renderer
- `widget-registry.ts` - Widget type registry
- `anywidget-view.tsx` - ESM widget loader
- `use-comm-router.ts` - Comm message routing
- `buffer-utils.ts` - Binary buffer utilities
- `controls/` - 50+ ipywidget components including:
  - Sliders, progress bars, text inputs
  - Selection widgets (dropdown, radio, toggle)
  - Media widgets (audio, video, image)
  - Layout containers (VBox, HBox, Tab, Accordion)
  - OutputWidget for capturing outputs in widget trees
  - Controller widget for gamepad input

## From `@shadcn` Registry (upstream)

### `ui/` - Primitives
Standard shadcn/ui components pulled from upstream:

`accordion`, `avatar`, `badge`, `button`, `checkbox`, `collapsible`,
`command`, `dialog`, `dropdown-menu`, `hover-card`, `input`, `label`,
`popover`, `progress`, `radio-group`, `select`, `sheet`, `slider`,
`tabs`, `textarea`, `toggle`, `toggle-group`

## Updating

```bash
# Update all nteract components
npx shadcn@latest add @nteract/all -yo --overwrite

# Update specific shadcn primitives
npx shadcn@latest add button badge -yo --overwrite
```

## Import Paths

```typescript
// Outputs
import { MediaRouter } from "@/components/outputs/media-router";
import { MediaProvider } from "@/components/outputs/media-provider";
import { AnsiOutput } from "@/components/outputs/ansi-output";

// Cell
import { OutputArea } from "@/components/cell/OutputArea";
import { PlayButton } from "@/components/cell/PlayButton";

// Widgets
import { WidgetView } from "@/components/widgets/widget-view";
import "@/components/widgets/controls"; // Register all widgets

// Editor
import { CodeMirrorEditor } from "@/components/editor";

// UI primitives
import { Button } from "@/components/ui/button";
```

## MediaProvider Usage

Configure renderers once at the top of your app:

```typescript
<WidgetStoreProvider sendMessage={sendToKernel}>
  <MediaProvider
    renderers={{
      "application/vnd.jupyter.widget-view+json": ({ data }) => {
        const { model_id } = data as { model_id: string };
        return <WidgetView modelId={model_id} />;
      },
    }}
  >
    <OutputArea outputs={cell.outputs} />
  </MediaProvider>
</WidgetStoreProvider>
```

## Known Issues

| Issue | Status |
|-------|--------|
| [#115](https://github.com/nteract/elements/issues/115) | Missing files in widget-controls (audio, video, button-style-utils) - manually added |

See [nteract/elements](https://github.com/nteract/elements) for registry source.

Documentation: https://nteract-elements.vercel.app/llms-full.txt