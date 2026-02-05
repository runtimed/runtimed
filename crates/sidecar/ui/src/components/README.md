# Components

This directory contains React components from multiple sources.

## Local (sidecar-specific)

These files are specific to this sidecar implementation:

- `widget-debugger.tsx` - Debug panel for inspecting widget state

## From `@nteract` Registry

Installed via `npx shadcn@latest add @nteract/all -yo`

### Outputs
- `ansi-output.tsx` - ANSI escape sequence rendering
- `html-output.tsx` - HTML with iframe sandbox
- `image-output.tsx` - Base64/URL images
- `json-output.tsx` - Interactive JSON tree
- `markdown-output.tsx` - GFM + math + syntax highlighting
- `svg-output.tsx` - Vector graphics
- `media-router.tsx` - MIME-type dispatch

### Cell Components
- `CellContainer.tsx` - Focus/selection wrapper
- `CellControls.tsx` - Cell action menu
- `CellHeader.tsx` - Header layout
- `CellTypeButton.tsx` - Cell type buttons
- `CellTypeSelector.tsx` - Cell type dropdown
- `ExecutionCount.tsx` - `[n]:` indicator
- `ExecutionStatus.tsx` - Status badges
- `OutputArea.tsx` - Output wrapper
- `PlayButton.tsx` - Run/stop button
- `RuntimeHealthIndicator.tsx` - Kernel status

### Collaboration
- `CollaboratorAvatars.tsx` - User avatars
- `PresenceBookmarks.tsx` - Presence indicators

### Editor
- `editor/` - CodeMirror 6 setup

### Widgets
- `widgets/` - ipywidget system + 47 controls

## From `@shadcn` Registry (upstream)

Standard shadcn/ui primitives in `ui/`:

- `accordion.tsx`, `avatar.tsx`, `badge.tsx`, `button.tsx`
- `checkbox.tsx`, `collapsible.tsx`, `command.tsx`, `dialog.tsx`
- `dropdown-menu.tsx`, `hover-card.tsx`, `input.tsx`, `label.tsx`
- `popover.tsx`, `progress.tsx`, `radio-group.tsx`, `select.tsx`
- `sheet.tsx`, `slider.tsx`, `tabs.tsx`, `textarea.tsx`
- `toggle.tsx`, `toggle-group.tsx`

## Updating

To update from upstream registries:

```bash
# Update all nteract components
npx shadcn@latest add @nteract/all -yo --overwrite

# Update specific shadcn primitives
npx shadcn@latest add button badge -yo --overwrite
```

See [nteract/elements](https://github.com/nteract/elements) for registry source.