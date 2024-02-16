# runtimed

RuntimeD is a daemon for REPLs built on top of Jupyter kernels.

## Introduction

We're exposing a document oriented interface to working with kernels, as a REST or GraphQL API:

![image](https://github.com/runtimed/runtimed/assets/836375/07bf5289-8b2a-466b-a9ad-e243d289c232)

The short term goal is to track executions of runtimes for use by interactive applications like notebooks and consoles.

RuntimeD tracks executions of runtimes for recall and for working with interactive applications like notebooks and consoles.

We track the association between `Execution` and `Runtime` (a running kernel). We also track for specific notebook apps with a `Code Cell -> Execution`.

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
  kernel_json: ...,
  status: dead,
  last_keepalive: ... # not sure what we need here.
}
```


```typescript
CodeCell {
  src: str,
  execution: Execution
}
```


### CLI

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
