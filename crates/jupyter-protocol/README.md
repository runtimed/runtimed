# `jupyter-protocol`

This crate provides types for `JupyterMessage`s, for use with either the native `zeromq` backend (currently in `runtimelib`) or the `websocket` backend (in `jupyter-websocket-client`).

## Usage

```rust
use jupyter_protocol::{JupyterMessage, ExecuteRequest, ExecuteReply, JupyterMessageContent};

fn main() {
    let message: JupyterMessage = ExecuteRequest {
        code: "print('Hello, world!')".to_string(),
        silent: false,
        store_history: true,
        user_expressions: Default::default(),
        allow_stdin: false,
    }.into();

    socket.send(message).await?;

    while let Some(reply) = socket.recv().await? {
        match reply.content {
            JupyterMessageContent::ExecuteReply(reply) => {
                println!("Execution completed with status: {:?}", reply.status);
            }
            JupyterMessageContent::StreamContent(content) => {
                assert_eq!(content.name, "stdout");
                assert_eq!(content.text, "Hello, world!\n");
                println!("Received stdout message: {:?}", content.text);
            }
            other => {
                println!("Received other message: {:?}", other);
            }
        }
    }
}
```
