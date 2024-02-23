mod jupyter;
use crate::jupyter::discovery;
use crate::jupyter::client;


pub async fn list_instances() -> Vec<client::JupyterRuntime>  {
    discovery::get_jupyter_runtime_instances().await
}
