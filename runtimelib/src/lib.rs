pub mod jupyter_dirs;
pub mod jupyter_msg;


use serde::{Deserialize, Serialize};
use tokio::fs;
use serde_json::from_str;

#[derive(Serialize, Clone)]
pub struct JupyterEnvironment {
    process: String,
    argv: Vec<String>,
    display_name: String,
    language: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JupyterRuntime {
    pub shell_port: u16,
    pub iopub_port: u16,
    pub stdin_port: u16,
    pub control_port: u16,
    pub hb_port: u16,
    pub kernel_name: String,
    pub ip: String,
    key: String,
    pub transport: String, // TODO: Enumify with tcp, ipc
    signature_scheme: String,
    // We'll track the connection file path here as well
    #[serde(skip_deserializing)]
    pub connection_file: String,
}


pub async fn get_jupyter_runtime_instances() -> Vec<JupyterRuntime>{
    let runtime_dir = jupyter_dirs::runtime_dir();
    let mut runtimes = Vec::new();

    if let Ok(mut entries) = fs::read_dir(runtime_dir).await {
        while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).await.unwrap_or_default();
                if let Ok(mut runtime) = from_str::<JupyterRuntime>(&content) {
                    runtime.connection_file = path.to_str().unwrap_or_default().to_string();
                    runtimes.push(runtime);
                }
            }
        }
    }

    runtimes
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn smoke_test() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Test ask_jupyter (this will fail if Jupyter is not installed)
            match jupyter_dirs::ask_jupyter().await {
                Ok(paths) => println!("Jupyter Paths: {:?}", paths),
                Err(e) => panic!("Failed to ask Jupyter about its paths: {}", e),
            };

            // Test config_dirs
            let config_dirs = jupyter_dirs::config_dirs();
            assert!(!config_dirs.is_empty(), "Config dirs should not be empty");

            // Test data_dirs
            let data_dirs = jupyter_dirs::data_dirs();
            assert!(!data_dirs.is_empty(), "Data dirs should not be empty");

            // Test runtime_dir
            let runtime_dir = jupyter_dirs::runtime_dir();
            assert!(runtime_dir.exists(), "Runtime dir should exist");
        });
    }

    #[test]
    fn check_for_runtimes() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {

            let jupyter_runtimes = get_jupyter_runtime_instances().await;

            println!("Jupyter runtimes: {:?}", jupyter_runtimes)

        })
    }
}
