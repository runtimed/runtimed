pub mod jupyter;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn smoke_test() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Test ask_jupyter (this will fail if Jupyter is not installed)
            match jupyter::dirs::ask_jupyter().await {
                Ok(paths) => println!("Jupyter Paths: {:?}", paths),
                Err(e) => panic!("Failed to ask Jupyter about its paths: {}", e),
            };

            let config_dirs = jupyter::dirs::config_dirs();
            assert!(!config_dirs.is_empty(), "Config dirs should not be empty");

            let data_dirs = jupyter::dirs::data_dirs();
            assert!(!data_dirs.is_empty(), "Data dirs should not be empty");

            // TODO: Test the runtime directory behavior
            // let runtime_dir = jupyter_dirs::runtime_dir();
        });
    }

    #[test]
    fn check_for_runtimes() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let jupyter_runtimes = jupyter::discovery::get_jupyter_runtime_instances().await;

            println!("Jupyter runtimes: {:?}", jupyter_runtimes)
        })
    }
}
