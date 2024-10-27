## ROADMAP

### Milestone 1: Basic runtime management

- [x] List running runtimes (`runt ps`)
- [x] Start a new runtime (`runt start`)
- [ ] Stop a running runtime (`runt stop`)
- [x] Store client messages into a SQL DB

- [x] Add a `runt exec` command to submit code to a runtime
- [x] Add a `runt get-results` command to get the results of an execution

#### REST API

- [x] `GET /v0/runtime_instances/`
  - [x] `GET /v0/runtime_instances/:id`
  - [x] `POST /v0/runtime_instances/:id/run_code`
  - [x] `POST /v0/runtime_instances`
- [x] `GET /v0/executions/:msg_id`
- [x] `GET /v0/environments`

### Milestone 2: Document based access to runtimes

- [ ] Add a `runt export` command to export a runtime session to a notebook
- [ ] `/v0/notebook` APIs
  - [ ] `POST /v0/notebook`
  - [ ] `GET /v0/notebook/:id`
  - [ ] `POST /v0/notebook/:id/cell`
  - [ ] `GET /v0/notebook/:id/cell/:cell_id`
  - [ ] `POST /v0/notebook/:id/cell/:cell_id/run`

### Milestone 3: Release with documentation

Make an initial release with supplemental materials.

### Milestone 4: write Python bindings

For the purpose of getting users to hook up runtimes with large language models, write bindings and documentation to make it easy to interface.

### Milestone X: Jupyter server integration

- [ ] Communicate using Jupyter server's CRDT prototcol for notebooks

### Future Vision

We want to go beyond jupyter kernels for showing off an environment. We want to have access to what the packages available in a kernel are as well as databases. LLMs do very well getting access to databases. Exposing it through interactive computing.

```

```
