## runtimed

![lilrunt](https://github.com/runtimed/runtimed/assets/836375/f5d36136-5154-4c2c-b968-4354c29670b1)

RuntimeD is a daemon for REPLs built on top of Jupyter kernels. It's purpose built for exposing interactive computing primitives to large language models whether hosted or local. 

The main CLI for interfacing with `runtimed` is `runt`.

### Goal

The goal of `runt` is to provide simple, easy to use, and powerful access to interactive computing. We want to enable a new generation of builders to:

* Create new notebook applications
* Create new kinds of REPLs
* Allow large language models to reason about code and data 

There are two main interfaces: 

* `runt` - a CLI for managing runtimes
* `runtimed` - a daemon for working with the interactive computing runtimes

## Getting Started

```
git clone git@github.com:runtimed/runtimed.git
cd runtimed
# Install the cli, `runt` into your path
cargo install --path cli
```

### Usage

```
runt ps
| Kernel Name | IP        | Transport | Connection File                                                                            |
|-------------|-----------|-----------|--------------------------------------------------------------------------------------------|
| deno        | 127.0.0.1 | tcp       | ~/Library/Jupyter/runtime/kernel-76d276d5-3625-43ae-aee4-9628a22d64e8.json |
| python3     | 127.0.0.1 | tcp       | ~/Library/Jupyter/runtime/kernel-581f74c6-e366-4518-8826-84132763f68c.json |
| deno        | 127.0.0.1 | tcp       | ~/Library/Jupyter/runtime/kernel-f1d7210b-1942-44c8-90c6-35ca8135054c.json |
| deno        | 127.0.0.1 | tcp       | ~/Library/Jupyter/runtime/kernel-4bfd804a-befc-4e4c-b10a-3b3d79c3bf24.json |
| python3     | 127.0.0.1 | tcp       | ~/Library/Jupyter/runtime/kernel-05122fc6-3d9f-4ed0-8fcb-93d1f7316756.json |
```

## The idea behind the `runtimed` API 💡

We're exposing a document oriented interface to working with kernels, as a REST API:

![image](https://github.com/runtimed/runtimed/assets/836375/07bf5289-8b2a-466b-a9ad-e243d289c232)

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


### CLI Goals

```
$ runt start python3
started 14598
```

```
$ runt kill 14598
```

```
$ runt ps
14598 | Python
```

```
$ runt run python3

In [1]: 2 + 3
Out[1]: 5

In [2]: import pandas as pd

Runtime "ul3bn3or" exited with no errors. To export a notebook from your session, run

  runtime export ul3bn3or --notebook
```

```
$ runt ps

| Kernel Name     | Language | IP        | Transport | Connection File                                                                            | State        |
|-----------------|----------|-----------|-----------|--------------------------------------------------------------------------------------------|--------------|
| deno            |          | 127.0.0.1 | tcp       | /Users/kylekelley/Library/Jupyter/runtime/kernel-76d276d5-3625-43ae-aee4-9628a22d64e8.json | unresponsive |
| python3         |          | 127.0.0.1 | tcp       | /Users/kylekelley/Library/Jupyter/runtime/kernel-24661.json                                | alive        |
| python3         |          | 127.0.0.1 | tcp       | /Users/kylekelley/Library/Jupyter/runtime/kernel-95973.json                                | alive        |
| python3         |          | 127.0.0.1 | tcp       | /Users/kylekelley/Library/Jupyter/runtime/kernel-581f74c6-e366-4518-8826-84132763f68c.json | unresponsive |
| deno            |          | 127.0.0.1 | tcp       | /Users/kylekelley/Library/Jupyter/runtime/kernel-f1d7210b-1942-44c8-90c6-35ca8135054c.json | unresponsive |
| deno            |          | 127.0.0.1 | tcp       | /Users/kylekelley/Library/Jupyter/runtime/kernel-4bfd804a-befc-4e4c-b10a-3b3d79c3bf24.json | unresponsive |
| python3         |          | 127.0.0.1 | tcp       | /Users/kylekelley/Library/Jupyter/runtime/kernel-05122fc6-3d9f-4ed0-8fcb-93d1f7316756.json | unresponsive |
| pythonjvsc74a57 |          | 127.0.0.1 | tcp       | /Users/kylekelley/Library/Jupyter/runtime/kernel-v2-17060MJ3bVzCxcpj6.json                 | unresponsive |

$ runt rm kernel-76d276d5-3625-43ae-aee4-9628a22d64e8
```

## Development

### Working with the DB

The database is managed by the [sqlx library](https://github.com/launchbadge/sqlx). The db is created and any migrations are run automatically. If you are updating the schema or add more queries to the app, more tooling needs to be installed.

1. cargo install sqlx-cli
2. ln -s .env.example .env

New queries will need to be prepared with `cargo sqlx prepare --workspace`.

New migrations should be added with `cargo sqlx migrate add <description>`, edited, and then executed with `cargo sqlx migrate run`
