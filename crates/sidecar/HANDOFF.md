# Sidecar Widget Debugging - Handoff

## Current Issue: tqdm Progress Bars Look Wonky

tqdm in Jupyter uses `tqdm.auto` which creates ipywidgets-based progress bars:

```
HBox([
  Label(description),
  FloatProgress(bar),
  HTML(stats)
])
```

**Observed problems:**
1. Raw numeric values (96.00, 173131.00, etc.) appearing that should be hidden
2. HBox children rendering as separate cards instead of inline
3. Layout not matching JupyterLab's rendering

## Hypothesis: We're Not Reading Layout/Style Models

ipywidgets use separate models for layout and styling:

```python
widget.layout  # -> LayoutModel (IPY_MODEL_xxx)
widget.style   # -> StyleModel (IPY_MODEL_xxx)
```

These models contain properties like:
- `visibility: 'hidden' | 'visible'`
- `display: 'none' | 'flex' | ...`
- `width`, `height`, `flex`
- `bar_color` (for progress bars)

tqdm likely sets `layout.visibility = 'hidden'` on internal backing widgets (like the raw value displays), but if we're ignoring these fields, everything renders visible.

## Investigation: Capture Full Model State

### Option 1: Console Logging

Add to `widget-store.ts` in `createModel`:

```typescript
createModel(commId, state, buffers) {
  console.log('[widget-store] createModel:', {
    modelName: state._model_name,
    modelModule: state._model_module,
    allKeys: Object.keys(state),
    layout: state.layout,  // IPY_MODEL_ reference?
    style: state.style,    // IPY_MODEL_ reference?
    fullState: state,
  });
  // ...
}
```

Also log in `updateModel` to see what fields change.

### Option 2: Widget Debugger Panel (Recommended)

Add a collapsible debug panel to the sidecar UI that shows all widget models and their state in real-time. This would be invaluable for debugging widget rendering issues.

**Suggested implementation:**

```tsx
// components/widget-debugger.tsx
function WidgetDebugger() {
  const models = useWidgetModels();
  
  return (
    <details className="widget-debugger">
      <summary>🔧 Widget Models ({models.size})</summary>
      <div className="models-list">
        {Array.from(models.entries()).map(([id, model]) => (
          <details key={id}>
            <summary>{model.modelName} ({id.slice(0, 8)}...)</summary>
            <pre>{JSON.stringify(model.state, null, 2)}</pre>
          </details>
        ))}
      </div>
    </details>
  );
}
```

Add it to `App.tsx` below the header or as a slide-out panel. This gives immediate visibility into:
- What models exist
- Their full state including `layout` and `style` refs
- How state changes over time

## Expected Findings

When inspecting tqdm's widget models, we'll likely see:

1. **LayoutModel** instances with properties we're ignoring:
   ```json
   {
     "_model_name": "LayoutModel",
     "visibility": "hidden",
     "width": "auto",
     "display": "inline-flex"
   }
   ```

2. **IPY_MODEL_ references** in widget state:
   ```json
   {
     "_model_name": "FloatProgressModel",
     "value": 50.0,
     "layout": "IPY_MODEL_abc123",
     "style": "IPY_MODEL_def456"
   }
   ```

3. **ProgressStyleModel** with bar colors:
   ```json
   {
     "_model_name": "ProgressStyleModel", 
     "bar_color": "#00ff00"
   }
   ```

## Next Steps

1. **Add widget debugger panel** - Quick win for visibility
2. **Log model state** - Capture tqdm's actual widget tree
3. **Identify missing fields** - Compare what tqdm sends vs what we read
4. **Implement LayoutModel support** - Apply visibility, display, flex properties
5. **Implement StyleModel support** - Apply colors, custom styling

## Files to Modify

| File | Change |
|------|--------|
| `ui/src/App.tsx` | Add WidgetDebugger component |
| `ui/src/components/widget-debugger.tsx` | New file - debug panel |
| `ui/src/lib/widget-store.ts` | Add debug logging (temporary) |
| `ui/src/components/*-widget.tsx` | Read layout/style refs and apply |

## Related

- ipywidgets Layout docs: https://ipywidgets.readthedocs.io/en/latest/examples/Widget%20Layout.html
- ipywidgets Styling docs: https://ipywidgets.readthedocs.io/en/latest/examples/Widget%20Styling.html
- tqdm notebook mode: https://tqdm.github.io/docs/notebook/