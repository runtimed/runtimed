## API

> [!WARNING]
> This is a Work in Progress. The API is not yet implemented.

The `runtimed` API is a REST API that exposes a document oriented interface to working with kernels.

### `GET /v0/runtime`

List all runtimes.

### `POST /v0/runtime`

Launch a new runtime

### `GET /v0/runtime/:id`

Get the status of a runtime.

### `POST /v0/runtime/:id/cell`

Create a cell for a session.

### `GET /v0/runtime/:id/cell/:cell_id`

Returns

```
{
    cell_type: "code",
    source: ...,
    metadata: ...,
    outputs_id: ...
}
```

### `GET /v0/outputs/:id`

Get the list of outputs given an ID


