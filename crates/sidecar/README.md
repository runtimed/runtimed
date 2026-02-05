# Sidecar

A lightweight viewer for Jupyter outputs that runs alongside your terminal session. Built with a React frontend using [nteract elements](https://nteract-elements.vercel.app/) for rich output rendering and interactive widgets.

![sidecar view from jupyter console](https://github.com/user-attachments/assets/f34e89b9-950d-40fd-a65a-b89b84776e32)

## Features

- **Rich Output Rendering** - text/plain, text/html, text/markdown, images, SVG, JSON tree view, ANSI terminal colors
- **Interactive Widgets** - Full ipywidgets support with two-way binding
- **anywidget Support** - Works with quak, drawdata, jupyter-scatter, and other anywidget-based libraries
- **Lightweight** - Single binary, no Python dependencies beyond your existing kernel

## Installation

```bash
cargo install sidecar
```

## Usage

In a `jupyter console` session with Python, run:

```python
import subprocess
from ipykernel.connect import get_connection_file

connection_file = get_connection_file()

sidecar = subprocess.Popen(
    ["sidecar", "--quiet", connection_file],
    stdout=subprocess.PIPE, stderr=subprocess.PIPE
)
```

That will open a separate window showing the output of your Jupyter session.

### Interactive Widgets

Standard ipywidgets work out of the box:

```python
import ipywidgets as widgets
widgets.IntSlider(value=50, min=0, max=100, description='Test:')
```

anywidgets like `quak` for data exploration:

```python
%load_ext quak
import polars as pl
df = pl.read_parquet(
    "https://github.com/uwdata/mosaic/raw/main/data/athletes.parquet"
)
df
```

Or `drawdata` for interactive annotation:

```python
from drawdata import ScatterWidget
widget = ScatterWidget()
widget
```

## Development

The UI is built with Vite + React + Tailwind CSS. The built assets are embedded in the Rust binary at compile time.

### Building the UI

```bash
cd ui
npm install
npm run build
```

### Building sidecar

After building the UI:

```bash
cargo build -p sidecar
```

For debug builds with devtools access (Cmd+Option+I):

```bash
cargo build -p sidecar
RUST_LOG=info ./target/debug/sidecar /path/to/kernel.json
```

## nteract elements Integration

The sidecar UI uses components from [nteract elements](https://nteract-elements.vercel.app/), a shadcn/ui-based component registry for Jupyter interfaces.

### Adding Components

Components are added via the shadcn CLI from the nteract-elements registry:

```bash
cd ui
npx shadcn@latest add https://nteract-elements.vercel.app/r/<component>.json
```

### Available Components

**Output Renderers:**
- `ansi-output` - ANSI escape sequences (terminal colors)
- `html-output` - HTML content
- `image-output` - PNG, JPEG, GIF, WebP images
- `svg-output` - SVG graphics
- `json-output` - Interactive JSON tree view
- `markdown-output` - Markdown with syntax highlighting and LaTeX
- `media-router` - Automatic MIME type routing

**Widget Infrastructure:**
- `widget-store` - Pure React state management for Jupyter widget models
- `anywidget-view` - ESM loader and AFM interface for anywidgets
- `widget-view` - Universal router for rendering widgets

### Import Path Adjustments

The registry installs components to `@/registry/widgets/` by default. Our project uses:
- `@/lib/` for utilities and stores
- `@/components/widgets/` for widget components

After pulling new components, you may need to adjust import paths accordingly.

### Widget Support Status

**Supported ipywidgets:**
- Sliders: `IntSlider`, `FloatSlider`
- Progress: `IntProgress`, `FloatProgress`
- Buttons: `Button`, `ToggleButton`, `ToggleButtons`
- Inputs: `Text`, `Textarea`, `Checkbox`
- Selection: `Dropdown`, `RadioButtons`, `SelectMultiple`
- Containers: `VBox`, `HBox`, `Box`, `GridBox`, `Tab`, `Accordion`

**anywidget Support:**
- Full AFM (AnyWidget Frontend Module) interface
- ESM loading (inline and URL)
- CSS injection with cleanup
- Two-way state binding
- Custom messages with binary buffers

**Not Yet Implemented** (tracked in [nteract/elements#89](https://github.com/nteract/elements/issues/89)):
- `HTMLModel` - Arbitrary HTML display
- `ColorPickerModel` - Color picker input
- `IntRangeSliderModel` / `FloatRangeSliderModel` - Dual-handle sliders

These will be added upstream in nteract/elements and can be pulled when available.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Sidecar App                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌──────────────┐    ┌───────────────┐  │
│  │   Webview   │───▶│  Rust/Wry    │───▶│    Kernel     │  │
│  │  (React UI) │◀───│  (ZMQ/JSON)  │◀───│   (Python)    │  │
│  └─────────────┘    └──────────────┘    └───────────────┘  │
│        │                   │                    │          │
│   POST /message       Shell channel        comm_msg        │
│   (widget updates)    (to kernel)          (widget sync)   │
│                                                             │
│   globalThis.onMessage  IOPub channel     comm_open/msg    │
│   (receive outputs)     (from kernel)     (widget state)   │
└─────────────────────────────────────────────────────────────┘
```

### Key Files

| File | Purpose |
|------|---------|
| `ui/src/App.tsx` | Main app, message handling, buffer decoding |
| `ui/src/lib/widget-store.ts` | Widget model state management |
| `ui/src/lib/widget-store-context.tsx` | React context and hooks |
| `ui/src/lib/use-comm-router.ts` | Jupyter comm protocol handling |
| `ui/src/lib/buffer-utils.ts` | Buffer path handling for binary data |
| `ui/src/components/widgets/anywidget-view.tsx` | anywidget ESM loader + AFM interface |
| `ui/src/components/widgets/widget-view.tsx` | Routes to correct widget renderer |
| `ui/src/components/widgets/widget-registry.ts` | Maps model names to React components |
| `ui/src/components/widgets/controls/` | Individual widget implementations |
| `src/main.rs` | Rust: ZMQ, Webview, message routing |

## Related Resources

- [nteract elements docs](https://nteract-elements.vercel.app/docs)
- [anywidget AFM spec](https://anywidget.dev/en/afm/)
- [Jupyter Widget Protocol](https://jupyter-widgets.readthedocs.io/en/latest/examples/Widget%20Low%20Level.html)
- [nteract/elements#89](https://github.com/nteract/elements/issues/89) - Phase III widget controls