# Sidecar

A lightweight viewer of Jupyter output to run next to your terminal session. Built with a React frontend using [nteract elements](https://nteract-elements.vercel.app/) for rich output rendering.

![sidecar view from jupyter console](https://github.com/user-attachments/assets/f34e89b9-950d-40fd-a65a-b89b84776e32)

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

It even works with anywidgets like `quak`!

```python
%load_ext quak
import polars as pl
df = pl.read_parquet(
    "https://github.com/uwdata/mosaic/raw/main/data/athletes.parquet"
)
df
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

### Adding nteract elements components

Components are added via the shadcn CLI from the nteract-elements registry:

```bash
cd ui
npx shadcn@latest add https://nteract-elements.vercel.app/r/<component>.json
```

Available output components:
- `ansi-output` - ANSI escape sequences (terminal colors)
- `html-output` - HTML content
- `image-output` - PNG, JPEG, GIF, WebP images
- `svg-output` - SVG graphics
- `json-output` - Interactive JSON tree view
- `markdown-output` - Markdown with syntax highlighting and LaTeX
- `media-router` - Automatic MIME type routing (install dependencies individually first)
