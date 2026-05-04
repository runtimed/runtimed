use crate::{Result, RuntimeError};
use dirs::{data_dir, home_dir};
use serde_json::Value;
use std::env;
use std::path::PathBuf;

#[cfg(feature = "tokio-runtime")]
use tokio::process::Command;

#[cfg(feature = "async-dispatcher-runtime")]
use smol::process::Command;

#[cfg(any(feature = "tokio-runtime", feature = "async-dispatcher-runtime"))]
pub async fn ask_jupyter() -> Result<Value> {
    let output = Command::new("jupyter")
        .args(["--paths", "--json"])
        .output()
        .await
        .map_err(|e| RuntimeError::CommandFailed {
            command: "jupyter --paths --json",
            source: e,
        })?;

    if output.status.success() {
        let paths: Value = serde_json::from_slice(&output.stdout)?;
        Ok(paths)
    } else {
        Err(RuntimeError::JupyterCommandFailed(output.status))
    }
}

pub fn system_config_dirs() -> Vec<PathBuf> {
    if cfg!(windows) {
        match env::var("PROGRAMDATA") {
            Err(_) => vec![],
            Ok(program_data) => {
                vec![PathBuf::from(program_data).join("jupyter")]
            }
        }
    } else {
        vec![
            PathBuf::from("/usr/local/etc/jupyter"),
            PathBuf::from("/etc/jupyter"),
        ]
    }
}

pub fn config_dirs() -> Vec<PathBuf> {
    let mut paths = vec![];

    if let Ok(jupyter_config_dir) = env::var("JUPYTER_CONFIG_DIR") {
        paths.push(PathBuf::from(jupyter_config_dir));
    }
    if let Some(home_dir) = home_dir() {
        paths.push(home_dir.join(".jupyter"));
    }

    paths.extend(system_config_dirs());

    // TODO: Use the sys.prefix from python and add that to the paths
    paths
}

pub fn system_data_dirs() -> Vec<PathBuf> {
    if cfg!(windows) {
        match env::var("PROGRAMDATA") {
            Err(_) => vec![],
            Ok(program_data) => {
                let program_data_dir = PathBuf::from(program_data);
                vec![program_data_dir.join("jupyter")]
            }
        }
    } else {
        vec![
            PathBuf::from("/usr/local/share/jupyter"),
            PathBuf::from("/usr/share/jupyter"),
        ]
    }
}

pub fn user_data_dir() -> Result<PathBuf> {
    if cfg!(target_os = "macos") {
        Ok(home_dir()
            .ok_or(RuntimeError::DirNotFound("home"))?
            .join("Library/Jupyter"))
    } else if cfg!(windows) {
        Ok(
            PathBuf::from(env::var("APPDATA").map_err(|_| RuntimeError::DirNotFound("APPDATA"))?)
                .join("jupyter"),
        )
    } else {
        // TODO: Respect XDG_DATA_HOME if set
        match data_dir() {
            None => Ok(home_dir()
                .ok_or(RuntimeError::DirNotFound("home"))?
                .join(".local/share")),
            Some(data_dir) => Ok(data_dir.join("jupyter")),
        }
    }
}

pub fn data_dirs() -> Vec<PathBuf> {
    let mut paths = vec![];

    if let Ok(jupyter_path) = env::var("JUPYTER_PATH") {
        paths.push(PathBuf::from(jupyter_path));
    }

    if let Ok(user_data_dir) = user_data_dir() {
        paths.push(user_data_dir);
    }

    paths.extend(system_data_dirs());

    // TODO: Use the sys.prefix from python and add that to the paths
    paths
}

/// Like [`data_dirs`] but also includes paths reported by
/// `jupyter --paths --json`, so virtualenv-installed kernels are
/// visible. Falls back to [`data_dirs`] if `jupyter` is unavailable.
#[cfg(any(feature = "tokio-runtime", feature = "async-dispatcher-runtime"))]
pub async fn data_dirs_with_jupyter_paths() -> Vec<PathBuf> {
    let static_dirs = data_dirs();
    let jupyter_response = ask_jupyter().await.ok();
    merge_jupyter_data_paths(static_dirs, jupyter_response.as_ref())
}

// Appends entries from `jupyter_response["data"]` onto `static_dirs`,
// preserving order and skipping duplicates. Factored out for testing
// without spawning `jupyter`.
fn merge_jupyter_data_paths(
    static_dirs: Vec<PathBuf>,
    jupyter_response: Option<&Value>,
) -> Vec<PathBuf> {
    let mut seen: std::collections::HashSet<PathBuf> = static_dirs.iter().cloned().collect();
    let mut result = static_dirs;

    if let Some(value) = jupyter_response {
        if let Some(arr) = value.get("data").and_then(|v| v.as_array()) {
            for entry in arr {
                if let Some(s) = entry.as_str() {
                    let path = PathBuf::from(s);
                    if seen.insert(path.clone()) {
                        result.push(path);
                    }
                }
            }
        }
    }

    result
}

pub fn runtime_dir() -> PathBuf {
    if let Ok(jupyter_runtime_dir) = env::var("JUPYTER_RUNTIME_DIR") {
        PathBuf::from(jupyter_runtime_dir)
    } else if let Ok(xdg_runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(xdg_runtime_dir).join("jupyter")
    } else if let Ok(user_data_dir) = user_data_dir() {
        user_data_dir.join("runtime")
    } else {
        // Fallback to a temp dir
        env::temp_dir().join("jupyter").join("runtime")
    }
}

#[cfg(all(test, feature = "tokio-runtime"))]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::runtime::Runtime;

    #[test]
    fn smoke_test() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            // Test ask_jupyter (this will fail if Jupyter is not installed)
            match ask_jupyter().await {
                Ok(paths) => println!("Jupyter Paths: {:?}", paths),
                Err(e) => panic!("Failed to ask Jupyter about its paths: {}", e),
            };

            let config_dirs = config_dirs();
            assert!(!config_dirs.is_empty(), "Config dirs should not be empty");

            let data_dirs = data_dirs();
            assert!(!data_dirs.is_empty(), "Data dirs should not be empty");
        });
    }

    #[test]
    fn merge_appends_jupyter_data_paths_in_order() {
        let static_dirs = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        let response = json!({
            "data": ["/venv/share/jupyter", "/extra"],
            "config": ["/ignored"],
            "runtime": "/also-ignored"
        });

        let merged = merge_jupyter_data_paths(static_dirs, Some(&response));

        assert_eq!(
            merged,
            vec![
                PathBuf::from("/a"),
                PathBuf::from("/b"),
                PathBuf::from("/venv/share/jupyter"),
                PathBuf::from("/extra"),
            ]
        );
    }

    #[test]
    fn merge_dedups_against_static_dirs() {
        let static_dirs = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        let response = json!({"data": ["/b", "/c", "/a"]});

        let merged = merge_jupyter_data_paths(static_dirs, Some(&response));

        assert_eq!(
            merged,
            vec![
                PathBuf::from("/a"),
                PathBuf::from("/b"),
                PathBuf::from("/c"),
            ]
        );
    }

    #[test]
    fn merge_with_no_response_returns_static_dirs() {
        let static_dirs = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        let merged = merge_jupyter_data_paths(static_dirs.clone(), None);
        assert_eq!(merged, static_dirs);
    }

    #[test]
    fn merge_ignores_malformed_response() {
        let static_dirs = vec![PathBuf::from("/a")];
        // `data` is a string, not an array — should be ignored, not panic.
        let response = json!({"data": "not-an-array"});
        let merged = merge_jupyter_data_paths(static_dirs.clone(), Some(&response));
        assert_eq!(merged, static_dirs);
    }
}
