# UI Components (Shadcn + nteract)

This repo keeps shared UI components in `packages/ui` using the shadcn CLI and the `@nteract` registry.

## Quick Start

```bash
# from repo root
cd packages/ui

# add/refresh the registry (safe to re-run)
npx shadcn@latest registry add @nteract

# install or refresh all nteract components
npx shadcn@latest add @nteract/all -yo

# optional: add ipycanvas support
npx shadcn@latest add @nteract/ipycanvas -yo

# add a core shadcn component (example)
npx shadcn@latest add dialog -yo
```

## Notes

- `components.json` lives in `packages/ui` and is the source of truth for shadcn.
- The CLI will often create a `deno.lock` when running `@nteract/all`. This hasn't been diagnosed.
- Use `--overwrite` if you need to force-refresh generated files.

## Package Manager

- Prefer `pnpm` for shadcn updates in this repo.
- We have seen `npm install` error with `@repo/*` workspace package resolution when there is no root JS workspace.
