use crate::{Result, RuntimeError};
use dirs::{data_dir, home_dir};
use serde::Deserialize;
use std::env;
use std::path::PathBuf;

#[cfg(feature = "tokio-runtime")]
use tokio::process::Command;
#[cfg(feature = "tokio-runtime")]
use tokio::sync::OnceCell;

#[cfg(feature = "async-dispatcher-runtime")]
use smol::process::Command;

/// The paths returned by `jupyter --paths --json`.
#[derive(Debug, Clone, Deserialize)]
pub struct JupyterPaths {
    pub runtime: Vec<PathBuf>,
    pub config: Vec<PathBuf>,
    pub data: Vec<PathBuf>,
}

/// Global cache for Jupyter paths, initialized at most once.
#[cfg(feature = "tokio-runtime")]
static JUPYTER_PATHS_CACHE: OnceCell<Option<JupyterPaths>> = OnceCell::const_new();

/// Get the cached Jupyter paths, calling `ask_jupyter` at most once.
/// Returns `None` if the `jupyter` command fails or is not available.
#[cfg(feature = "tokio-runtime")]
pub async fn get_jupyter_paths() -> Option<JupyterPaths> {
    JUPYTER_PATHS_CACHE
        .get_or_init(|| async {
            match ask_jupyter().await {
                Ok(paths) => Some(paths),
                Err(_) => None,
            }
        })
        .await
        .clone()
}

#[cfg(any(feature = "tokio-runtime", feature = "async-dispatcher-runtime"))]
pub async fn ask_jupyter() -> Result<JupyterPaths> {
    let output = Command::new("jupyter")
        .args(["--paths", "--json"])
        .output()
        .await
        .map_err(|e| RuntimeError::CommandFailed {
            command: "jupyter --paths --json",
            source: e,
        })?;

    if output.status.success() {
        let paths: JupyterPaths = serde_json::from_slice(&output.stdout)?;
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

/// Returns the Jupyter config directories.
///
/// If the `jupyter` command is available, uses the cached result from `jupyter --paths --json`.
/// Otherwise, falls back to platform-specific defaults.
#[cfg(feature = "tokio-runtime")]
pub async fn config_dirs() -> Vec<PathBuf> {
    if let Some(jupyter_paths) = get_jupyter_paths().await {
        return jupyter_paths.config;
    }

    let mut paths = vec![];

    if let Ok(jupyter_config_dir) = env::var("JUPYTER_CONFIG_DIR") {
        paths.push(PathBuf::from(jupyter_config_dir));
    }
    if let Some(home_dir) = home_dir() {
        paths.push(home_dir.join(".jupyter"));
    }

    paths.extend(system_config_dirs());

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

/// Returns the Jupyter data directories.
///
/// If the `jupyter` command is available, uses the cached result from `jupyter --paths --json`.
/// Otherwise, falls back to platform-specific defaults.
#[cfg(feature = "tokio-runtime")]
pub async fn data_dirs() -> Vec<PathBuf> {
    if let Some(jupyter_paths) = get_jupyter_paths().await {
        return jupyter_paths.data;
    }

    let mut paths = vec![];

    if let Ok(jupyter_path) = env::var("JUPYTER_PATH") {
        paths.push(PathBuf::from(jupyter_path));
    }

    if let Ok(user_data_dir) = user_data_dir() {
        paths.push(user_data_dir);
    }

    paths.extend(system_data_dirs());

    paths
}

/// Returns the Jupyter runtime directory.
///
/// If the `jupyter` command is available, uses the cached result from `jupyter --paths --json`.
/// Otherwise, falls back to platform-specific defaults.
#[cfg(feature = "tokio-runtime")]
pub async fn runtime_dir() -> PathBuf {
    if let Some(jupyter_paths) = get_jupyter_paths().await {
        if let Some(runtime_dir) = jupyter_paths.runtime.into_iter().next() {
            return runtime_dir;
        }
    }

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

    #[tokio::test]
    async fn smoke_test() {
        // Test ask_jupyter (this will fail if Jupyter is not installed)
        match ask_jupyter().await {
            Ok(paths) => println!("Jupyter Paths: {:?}", paths),
            Err(e) => panic!("Failed to ask Jupyter about its paths: {}", e),
        };

        let config = config_dirs().await;
        assert!(!config.is_empty(), "Config dirs should not be empty");

        let data = data_dirs().await;
        assert!(!data.is_empty(), "Data dirs should not be empty");

        let runtime = runtime_dir().await;
        assert!(!runtime.as_os_str().is_empty(), "Runtime dir should not be empty");
    }

    #[tokio::test]
    async fn test_caching() {
        // First call should initialize the cache
        let paths1 = get_jupyter_paths().await;
        // Second call should return the cached value
        let paths2 = get_jupyter_paths().await;

        assert_eq!(paths1.is_some(), paths2.is_some());
        if let (Some(p1), Some(p2)) = (paths1, paths2) {
            assert_eq!(p1.data, p2.data);
            assert_eq!(p1.config, p2.config);
            assert_eq!(p1.runtime, p2.runtime);
        }
    }
}
