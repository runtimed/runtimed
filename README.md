## runtimed

![lilrunt](https://github.com/runtimed/runtimed/assets/836375/f5d36136-5154-4c2c-b968-4354c29670b1)

RuntimeD is a daemon for REPLs built on top of Jupyter kernels. It's purpose built for exposing interactive computing primitives to large language models whether hosted or local.

The main CLI for interfacing with `runtimed` is `runt`.

### Goal

The goal of `runt` is to provide simple, easy to use, and powerful access to interactive computing. We want to enable a new generation of builders to:

- Create new notebook applications
- Create new kinds of REPLs
- Allow large language models to reason about code and data

There are three main interfaces:

- `runt` - a CLI for managing runtimes
- `runtimed` - a daemon for working with the interactive computing runtimes
- `runtimelib` - a rust library for interfacing with runtimes directly

## Getting Started with `runtimelib`

```
cargo install runtimelib
```

### Asynchronous dispatch options

By default, runtimelib uses tokio. However, the [async-dispatcher](https://github.com/zed-industries/async-dispatcher) runtime can be selected at compile time with:

```bash
cargo build --feature async-dispatch-runtime
```

This will allow you to build GPUI apps with runtimelib.


## Development - getting started

```
git clone git@github.com:runtimed/runtimed.git
cd runtimed
# Install the cli, `runt` into your path
cargo install --path runt
# Install the CLI for the `runtimed` daemon.
cargo install --path runtimed
# Start the daemon
runtimed
```

### Usage

If you haven't already, start the `runtimed` daemon.

```
$ runtimed
```

List the available runtimes.

```
$ runt ps
╭─────────────┬────────────┬──────────────────────────────────────┬───────────┬───────────┬───────╮
│ Kernel Name │ Language   │ ID                                   │ IP        │ Transport │ State │
├─────────────┼────────────┼──────────────────────────────────────┼───────────┼───────────┼───────┤
│ python3     │ python     │ 6a090dec-08cb-5429-a6c7-ea19d71fc06e │ 127.0.0.1 │ tcp       │ alive │
│ deno        │ typescript │ 79c4c28f-1ffb-579a-b77c-23e4a1bb45ec │ 127.0.0.1 │ tcp       │ alive │
╰─────────────┴────────────┴──────────────────────────────────────┴───────────┴───────────┴───────╯
```

### Submitting an Execution

```
$ runt exec 79c4c28f-1ffb-579a-b77c-23e4a1bb45ec $'function X() { return Math.random() };\nX()'
Execution "2d3827a2-4a7f-4a1f-bc41-5091f9ade2ab" submitted, run

runt get-results "2d3827a2-4a7f-4a1f-bc41-5091f9ade2ab"

to get the results of the execution.
```

### Getting Results

```
$ runt get-results "2d3827a2-4a7f-4a1f-bc41-5091f9ade2ab"
╭────────────────────────────────────────────────────╮
│ Execution Results                                  │
├────────────────────────────────────────────────────┤
│ Execution ID: 2d3827a2-4a7f-4a1f-bc41-5091f9ade2ab │
│ Status: idle                                       │
│ Started: 2024-03-05T23:57:46.680992Z               │
│ Finished: 2024-03-05T23:57:46.688572Z              │
│                                                    │
│ -- Code --                                         │
│ function X() { return Math.random() };             │
│ X()                                                │
│                                                    │
│ -- Output --                                       │
│ 0.21616865512200545                                │
│                                                    │
╰────────────────────────────────────────────────────╯
```

## The idea behind the `runtimed` API 💡

We're exposing a document oriented interface to working with kernels, as a REST API.

RuntimeD tracks executions of runtimes for recall and for working with interactive applications like notebooks and consoles. We track the association between `Execution` and `Runtime` (a running kernel). We also track for specific notebook apps with a `Code Cell -> Execution`.

```typescript
Execution {
  id: ULID,
  execution_started: timestamp,
  execution_end: timestamp,
  status: running | queued | ...
  runtime: Runtime
}
```

```typescript
Runtime {
  id: ULID,
  status: dead | alive | unresponsive,
  last_keepalive: timestamp
}
```

```typescript
CodeCell {
  src: str,
  execution: Execution
}
```

## Development

### Working with the DB

The database is managed by the [sqlx library](https://github.com/launchbadge/sqlx). The db is created and any migrations are run automatically. If you are updating the schema or add more queries to the app, more tooling needs to be installed.

```sh
cargo install sqlx-cli
```

```sh
ln -s .env.example .env
```

#### Preparing new queries

After you write a new query, you will need to run

```sh
cargo sqlx prepare --workspace
```

#### Migrations

New migrations should be added with the following command:

```sh
cargo sqlx migrate add create_executions_table
```

This will create a new migration file in `migrations/` that you can edit. After you are done, you can run the migration with:

```sh
cargo sqlx migrate run
```
