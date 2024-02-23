use crate::jupyter_dirs;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::from_str;
use serde_json::json;
use tokio::fs;
use zeromq::SocketRecv;

use crate::jupyter::client;

use crate::jupyter::messaging;

pub async fn get_jupyter_runtime_instances() -> Vec<client::JupyterRuntime> {
    let runtime_dir = jupyter_dirs::runtime_dir();
    let mut runtimes = Vec::new();

    if let Ok(mut entries) = fs::read_dir(runtime_dir).await {
        while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).await.unwrap_or_default();
                if let Ok(mut runtime) = from_str::<client::JupyterRuntime>(&content) {
                    runtime.connection_file = path.to_str().unwrap_or_default().to_string();

                    check_kernel_info(runtime.clone()).await;

                    runtimes.push(runtime);
                }
            }
        }
    }

    runtimes
}

pub async fn check_kernel_info(runtime: client::JupyterRuntime) {
    let res = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let mut client = runtime.attach().await;

        let message = messaging::JupyterMessage::new_with_type(
            "kernel_info_request",
            Some(json!({})),
            Some(json!({})),
        );

        let _res = message.send(&mut client.shell).await;
        let reply = client.shell.socket.recv().await;
        match reply {
            Ok(msg) => {
                println!("Received kernel info reply: {:?}", msg);
            }
            Err(e) => println!("Failed to receive kernel info reply: {:?}", e),
        }
    })
    .await;

    match res {
        Ok(result) => println!("we ok {:?}", result),
        Err(e) => println!("Timeout error: {:?}", e),
    }

    // let message = messaging::JupyterMessage{
    //     zmq_identities: Vec::new(),
    //     header,
    //     metadata: json!({}),
    //     content: json!({}),
    //     buffers: vec![],
    // }

    // let message:

    // client.shell.socket.send(message)
}
