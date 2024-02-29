## ROADMAP

### Milestone 1: Basic runtime management

- [x] List running runtimes (`runt ps`)
- [ ] Start a new runtime (`runt start`)
- [ ] Stop a running runtime (`runt stop`)
- [ ] Store client messages into a SQL DB

### Milestone 2: REST API for runtime access

- [ ] Connect to a running runtime (`runt connect`)
- [ ] `/v0/runtime` APIs
  - [x] `GET /v0/runtime`
  - [ ] `POST /v0/runtime`
- [ ] `/v0/runtime/:id/cell` APIs
  - [ ] `POST /v0/runtime/:id/cell`
  - [ ] `GET /v0/runtime/:id/cell/:cell_id`

### Milestone 3: Release with documentation

Make an initial release with supplemental materials.

### Milestone 4: write Python bindings

For the purpose of getting users to hook up runtimes with large language models, write bindings and documentation to make it easy to interface.

### Milestone X: Jupyter server integration

- [ ] Communicate using Jupyter server's CRDT prototcol for notebooks

### Future Vision

We want to go beyond jupyter kernels for showing off an environment. We want to have access to what the packages available in a kernel are as well as databases. LLMs do very well getting access to databases. Exposing it through interactive computing.
