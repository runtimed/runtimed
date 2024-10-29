pub mod client;
pub mod dirs;
pub mod kernelspec;

pub use kernelspec::KernelspecDir;

pub use client::*;
pub use kernelspec::*;

#[cfg(all(test, feature = "tokio-runtime"))]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn smoke_test() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Test ask_jupyter (this will fail if Jupyter is not installed)
            match dirs::ask_jupyter().await {
                Ok(paths) => println!("Jupyter Paths: {:?}", paths),
                Err(e) => panic!("Failed to ask Jupyter about its paths: {}", e),
            };

            let config_dirs = dirs::config_dirs();
            assert!(!config_dirs.is_empty(), "Config dirs should not be empty");

            let data_dirs = dirs::data_dirs();
            assert!(!data_dirs.is_empty(), "Data dirs should not be empty");
        });
    }
}
