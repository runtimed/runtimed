use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct KernelSpec {
    /// The command-line arguments to pass to the kernel process
    /// By convention, the argv has a {} in it that will be replaced with the connection file
    ///
    /// Example:
    /// ```json
    /// "argv": [
    ///    "/opt/homebrew/bin/deno",
    ///    "--unstable",
    ///    "jupyter",
    ///    "--kernel",
    ///    "--conn",
    ///    "{connection_file}"
    /// ],
    /// ```
    ///
    argv: Vec<String>,
    display_name: String,
    language: String,
}

impl KernelSpec {
    pub async fn start_kernel(&self, connection_file: String) -> std::io::Result<()> {
        todo!()
    }

    pub async fn stop_kernel(&self) -> std::io::Result<()> {
        todo!()
    }
}
