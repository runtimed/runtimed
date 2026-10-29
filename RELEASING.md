# Releasing

## Current patch release candidate

Prepare and review only `ollama-kernel` 1.2.5. It moves to the workspace
reqwest configuration (Rustls), which removes native TLS and openssl from the
published binary lockfile. Other package versions stay unchanged. No library
or Python publication is included.

Validate workspace archives and lockfiles in CI, then dry-run and execute
`cargo release publish -p ollama-kernel` from the reviewed merge. Audit the
packaged `ollama-kernel-1.2.5/Cargo.lock`, install the registry archive with
`--locked`, and run `ollama-kernel --help`. Tag and push only
`ollama-kernel-v1.2.5` after verification. The older commands below describe
preceding releases; use this candidate's package and version for this patch.

We use `cargo-release` and publish Rust crates to crates.io. Every mutating
release step has a dry-run default: inspect that result before adding
`--execute`. Never bypass required CI or branch protections.

## Prepare a reviewed release

1. Start a release branch from current `main`. Compare each package with its
   last package tag and the crates.io version. Bump only packages being released.
   Public dependency types and feature removals count in the semver decision.
2. Use `cargo release version -p <crate> <patch|minor|major>` to preview the
   version/dependency changes, then repeat with `--execute`. This step edits
   manifests only; it does not publish, tag, or push.
3. Add notes under each crate's `## [Unreleased]` heading. Run
   `cargo release replace -p <crate>` and then `--execute` once to promote notes
   through the crate's `release.toml`. `cargo-release` generates dated headings.
4. Commit the version, notes, migration instructions, and necessary fixes in a
   reviewable PR. Run `cargo fmt --all -- --check`, `cargo package --workspace --locked`,
   `python3 scripts/check-package-locks.py target/package/*.crate`, the normal CI suite, and all four runtime/crypto combinations from
   `.github/workflows/packages.yml`. Do not use `--all-features`: the runtime and
   crypto backends are mutually exclusive. Workspace packaging stages unpublished
   internal dependencies and verifies each archive independently.
5. Obtain review and merge the release PR. Re-fetch and verify the exact merged
   commit, clean tree, successful required checks, and unchanged registry state.
   If additional commits landed, reassess their release impact before publishing.

## Preserve the reviewed dependency resolution

The root `Cargo.lock` is tracked because this workspace publishes binaries as
well as libraries. A clean source commit alone does not prove that an archive
uses the dependency versions reviewed on another machine. Refresh dependencies
on the release branch, review the lockfile diff, and commit it before CI.

Package and check the actual archives:

```sh
cargo package --workspace --locked
# This patch releases only Ollama, so use registry checksums for its libraries.
cargo package -p ollama-kernel --locked
python3 scripts/check-package-locks.py target/package/*.crate
cargo audit --deny warnings --file target/package/ollama-kernel-<version>/Cargo.lock
```

Build/install the archive with `--locked`. Treat an audit finding or resolution
mismatch as a release blocker; investigate reachability rather than dismissing
it because the source commit matches. Library consumers still resolve their own
dependencies.

For the corrective Ollama 1.2.3 release, first run:

```sh
cargo publish -p ollama-kernel --locked --dry-run
```

Inspect the generated archive with the checks above, then repeat without
`--dry-run` only after the release PR is approved, merged, and publication is
authorized. This command preserves the lockfile; keep the existing cargo-release
tag and push steps after registry verification.
Do not republish the already shipped library versions or rewrite their tags.

## Publish the already prepared commit

Use the existing authorized crates.io identity. Do not create new credentials or
change registry grants as part of a release. If credentials are missing, stop
before publication and ask the owner to supply an authorized publishing path.

For a commit whose versions and dated changelogs are already prepared, use the
individual `publish`, `tag`, and `push` steps. Do not repeat the all-in-one release
command or `replace`: that would prepare the versions/notes a second time.

Publish in dependency order, verifying registry visibility after each step:

1. `jupyter-protocol`
2. Its consumers: `nbformat`, `jupyter-websocket-client`, `jupyter-zmq-client`
3. `runtimelib` after `jupyter-zmq-client`; `jupyter-ysync` after its protocol,
   notebook, and optional WebSocket dependencies; `ollama-kernel` after
   `jupyter-zmq-client`

`mybinder` is independent. Skip packages not selected for the release.

The October 2026 candidate selects these commands, each first as a dry run:

```sh
cargo release publish -p jupyter-protocol
cargo release publish -p jupyter-zmq-client
cargo release publish -p runtimelib
cargo release publish -p ollama-kernel
```

After a successful dry run, repeat that one command with `--execute`, then check
its exact new version on crates.io before moving to the next. For this candidate
those versions are 2.0.3, 2.0.0, 4.0.0, and 1.2.2 respectively. The default runtime
is now Tokio plus ring, so publication needs no extra runtime feature flag.
A dependent dry run cannot resolve a new dependency from crates.io until the
preceding publication is available; archive staging provides the pre-publication
build gate.

Once all selected uploads are verified, preview and execute package tagging,
then push the package tags and approved commit. The local branch must already
match the reviewed, merged commit on `main`:

```sh
cargo release tag -p jupyter-protocol -p jupyter-zmq-client -p runtimelib -p ollama-kernel
cargo release push -p jupyter-protocol -p jupyter-zmq-client -p runtimelib -p ollama-kernel
```

Inspect each dry run before repeating with `--execute`. Do not rewrite an
existing tag or force-push. GitHub Release pages are optional; the established
artifacts are crates.io archives and package tags. There is no publishing Actions
workflow in this repository.

## Verify delivery

- Confirm the exact registry versions, checksums, repository links, and package
  tags at the approved source commit. Download the released archives and inspect
  `.cargo_vcs_info.json`; do not infer success from an upload command alone.
- Build a fresh external consumer from registry versions. Exercise shared
  protocol types and the explicit dispatcher/AWS-LC combination as well as
  defaults. Run a loopback transport smoke with a timeout and the versionless
  CodeMirror fixture. Record which platforms and real kernels were tested.
- Confirm docs.rs builds, and install/run the selected binary from crates.io if
  applicable. `ollama-kernel --help` is a bounded CLI smoke; do not install a
  global kernelspec or contact an Ollama service without an explicit test scope.
- If publication partially succeeds, record exact versions and tags; inspect
  registry state before resuming. Do not delete/yank releases to hide failures.

## Scope

The candidate's breaking changes and downstream migration notes are in
[MIGRATION.md](MIGRATION.md). Unrelated dependency PRs have independent review and
version gates. The Python extension metadata is separate; no PyPI publication is
implied by a Rust crate release.

For `runt`, `sidecar`, and the Python `runtimed` package, use the
[runtimed/runt](https://github.com/runtimed/runt) repository. Keep its release
artifacts distinct from this repository and from nteract/nteract.
