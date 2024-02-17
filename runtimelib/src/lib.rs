pub mod jupyter_dirs;
pub mod jupyter_msg;

// Your existing library code here...

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
}