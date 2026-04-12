# Changelog

All notable changes to `nbformat` will be documented in this file.

## [Unreleased]

## [2.0.0] - 2026-04-12

### Breaking

- Add `Notebook::V4QuirksMode` variant for v4.5 notebooks with missing cell IDs. (#295)
- Mark `Notebook` enum as `#[non_exhaustive]`.

### Added

- `V4Quirks` wrapper with `repair()` method to surface and fix spec violations via the type system.
- `Quirk` enum (also `#[non_exhaustive]`) with `MissingCellId` variant.

## [1.2.2] - 2026-03-14

## [1.2.1] - 2026-03-06

## [1.2.0] - 2026-02-23

### Added

- Support for v3 notebook parsing and upconversion to v4. (#275)

## [1.1.0] - 2026-02-20

### Fixed

- Accept both string and array for cell source.
- Trailing newline bug in MultilineString and media serialization. (#271)

## [1.0.0] - 2026-01-07

Stable release of the Jupyter notebook format parser. See git history for earlier changes.
