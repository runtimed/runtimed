use std::fs::{read_dir, File};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

// A pointer to a kernelspec directory, with the parsed JSON struct and the name
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KernelspecDir {
    pub name: String,
    pub path: PathBuf,
    pub kernelspec: JupyterKernelspec,
}

// Struct for the contents of a kernel.json file
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JupyterKernelspec {
    #[serde(default)]
    pub argv: Vec<String>,
    pub display_name: String,
    pub language: String,
    pub metadata: Option<Value>,
    pub interrupt_mode: Option<String>,
    pub env: Option<Value>,
}

// We look for files of the sort:
//    `<datadir>/kernels/<kernel_name>/kernel.json`
// But we must check through all the possible <datadir> to figure that out.
//
// For now, just use a combination of the standard system and user data directories.
pub fn kernelspecs() -> Vec<KernelspecDir> {
    let mut kernelspecs = Vec::new();
    let data_dirs = crate::dirs::data_dirs();
    for data_dir in data_dirs {
        kernelspecs.append(&mut read_kernelspec_jsons(&data_dir));
    }
    kernelspecs
}

// Design choice here is to not report any errors, keep going if possible,
// and skip any paths that don't have a kernels subdirectory.
pub fn list_kernelspec_names_at(data_dir: &Path) -> Vec<String> {
    let mut kernelspecs = Vec::new();
    let kernels_dir = data_dir.join("kernels");
    if let Ok(entries) = read_dir(kernels_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    if let Some(kernel_name) = entry.file_name().to_str() {
                        kernelspecs.push(kernel_name.to_string());
                    }
                }
            }
        }
    }
    kernelspecs
}

// For a given data directory, return all the parsed kernelspecs and corresponding directories
pub fn read_kernelspec_jsons(data_dir: &Path) -> Vec<KernelspecDir> {
    let mut kernelspecs = Vec::new();
    let kernel_names = list_kernelspec_names_at(data_dir);
    for kernel_name in kernel_names {
        let kernel_path = data_dir.join("kernels").join(&kernel_name);
        if let Ok(jupyter_runtime) = read_kernelspec_json(&kernel_path.join("kernel.json")) {
            kernelspecs.push(KernelspecDir {
                name: kernel_name,
                path: kernel_path,
                kernelspec: jupyter_runtime,
            });
        }
    }
    kernelspecs
}

fn read_kernelspec_json(json_file_path: &Path) -> anyhow::Result<JupyterKernelspec> {
    let file = File::open(json_file_path)?;
    let jupyter_runtime: JupyterKernelspec = serde_json::from_reader(&file)?;
    Ok(jupyter_runtime)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_jupyter_runtime_config() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("tests/kernels/ir/kernel.json");
        let jupyter_runtime = read_kernelspec_json(&d).unwrap();
        assert_eq!(jupyter_runtime.display_name, "R");
        assert_eq!(jupyter_runtime.language, "R");
        assert!(jupyter_runtime.env.is_none());
        assert!(jupyter_runtime.metadata.is_none());
        assert_eq!(jupyter_runtime.argv.len(), 6);
        assert_eq!(jupyter_runtime.interrupt_mode, Some("signal".to_string()));
    }

    #[test]
    fn test_read_missing_config() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("tests/kernels/NONEXISTENT/kernel.json");
        let jupyter_runtime = read_kernelspec_json(&d);
        assert!(jupyter_runtime.is_err());
    }

    #[test]
    fn test_list_kernelspec_jsons() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("tests");
        let kernelspecs = list_kernelspec_names_at(&d);
        assert_eq!(kernelspecs.len(), 3);
        assert!(kernelspecs.contains(&"ir".to_string()));
        assert!(kernelspecs.contains(&"python3".to_string()));
        assert!(kernelspecs.contains(&"rust".to_string()));
    }

    #[test]
    fn test_read_kernelspec_jsons() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("tests");
        let kernels = read_kernelspec_jsons(&d);
        assert_eq!(kernels.len(), 3);
        let mut r_count = 0;
        let mut python_count = 0;
        let mut rust_count = 0;
        for kerneldir in kernels {
            let kernelspec = &kerneldir.kernelspec;
            match kernelspec.display_name.as_str() {
                "R" => {
                    assert_eq!(kernelspec.language, "R");
                    assert_eq!(kernelspec.argv.len(), 6);
                    assert_eq!(kernelspec.interrupt_mode, Some("signal".to_string()));
                    r_count += 1;
                }
                "Python 3" => {
                    assert_eq!(kernelspec.language, "python");
                    assert_eq!(kernelspec.argv.len(), 5);
                    assert_eq!(kernelspec.interrupt_mode, None);
                    python_count += 1;
                }
                "Rust" => {
                    assert_eq!(kernelspec.language, "rust");
                    assert_eq!(kernelspec.argv.len(), 3);
                    assert_eq!(kernelspec.interrupt_mode, Some("message".to_string()));
                    rust_count += 1;
                }
                _ => panic!("Unexpected kernelspec found: {}", &kernelspec.display_name),
            }
        }
        assert_eq!(r_count, 1);
        assert_eq!(python_count, 1);
        assert_eq!(rust_count, 1);
    }

    #[test]
    fn list_nonexistent_kernelspec_datadir() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("tests/NOTHINGHERE");
        let kernels = list_kernelspec_names_at(&d);
        assert_eq!(kernels.len(), 0);
    }
}
