use dirs::{data_dir, home_dir};
use serde_json::Value;
use std::env;
use std::path::PathBuf;
use tokio::process::Command;

/// Asynchronously queries Jupyter for its paths.
///
/// # Errors
///
/// Returns an error if the `jupyter` command fails or the output cannot be parsed as JSON.
pub async fn ask_jupyter() -> Result<Value, Box<dyn std::error::Error>> {
    let output = Command::new("jupyter")
        .args(["--paths", "--json"])
        .output()
        .await?;

    if output.status.success() {
        let paths: Value = serde_json::from_slice(&output.stdout)?;
        Ok(paths)
    } else {
        Err("Failed to ask Jupyter about its paths".into())
    }
}

/// Returns the system-wide configuration directories for Jupyter.
///
/// On Windows, it returns the `PROGRAMDATA` environment variable path appended with `jupyter`.
/// On Unix-like systems, it returns the common system-wide configuration paths for Jupyter.
pub fn system_config_dirs() -> Vec<PathBuf> {
    if cfg!(windows) {
        vec![PathBuf::from(env::var("PROGRAMDATA").unwrap_or_default()).join("jupyter")]
    } else {
        vec![
            PathBuf::from("/usr/local/etc/jupyter"),
            PathBuf::from("/etc/jupyter"),
        ]
    }
}

/// Returns the user and system-wide configuration directories for Jupyter.
///
/// It includes the user-specific configuration directory, the `JUPYTER_CONFIG_DIR` environment
/// variable path if set, and the system-wide configuration directories.
pub fn config_dirs() -> Vec<PathBuf> {
    let mut paths = vec![];

    if let Ok(jupyter_config_dir) = env::var("JUPYTER_CONFIG_DIR") {
        paths.push(PathBuf::from(jupyter_config_dir));
    }

    paths.push(home_dir().unwrap_or_default().join(".jupyter"));
    paths.extend(system_config_dirs());

    // TODO: Use the sys.prefix from python and add that to the paths
    paths
}

/// Returns the system-wide data directories for Jupyter.
///
/// On Windows, it returns the `PROGRAMDATA` environment variable path appended with `jupyter`.
/// On Unix-like systems, it returns the common system-wide data paths for Jupyter.
pub fn system_data_dirs() -> Vec<PathBuf> {
    if cfg!(windows) {
        vec![PathBuf::from(env::var("PROGRAMDATA").unwrap_or_default()).join("jupyter")]
    } else {
        vec![
            PathBuf::from("/usr/local/share/jupyter"),
            PathBuf::from("/usr/share/jupyter"),
        ]
    }
}

/// Returns the user-specific data directory for Jupyter.
///
/// On macOS, it returns the `Library/Jupyter` directory inside the user's home directory.
/// On Windows, it returns the `APPDATA` environment variable path appended with `jupyter`.
/// On other Unix-like systems, it respects the `XDG_DATA_HOME` environment variable if set,
/// or defaults to `.local/share/jupyter` inside the user's home directory.
pub fn user_data_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        home_dir().unwrap_or_default().join("Library/Jupyter")
    } else if cfg!(windows) {
        PathBuf::from(env::var("APPDATA").unwrap_or_default()).join("jupyter")
    } else {
        // TODO: Respect XDG_DATA_HOME if set
        data_dir()
            .unwrap_or_else(|| home_dir().unwrap_or_default().join(".local/share"))
            .join("jupyter")
    }
}

/// Returns the user-specific and system-wide data directories for Jupyter.
///
/// It includes the user-specific data directory, the `JUPYTER_PATH` environment variable path if
/// set, and the system-wide data directories.
pub fn data_dirs() -> Vec<PathBuf> {
    let mut paths = vec![];

    if let Ok(jupyter_path) = env::var("JUPYTER_PATH") {
        paths.push(PathBuf::from(jupyter_path));
    }

    paths.push(user_data_dir());
    paths.extend(system_data_dirs());

    // TODO: Use the sys.prefix from python and add that to the paths
    paths
}

/// Returns the runtime directory for Jupyter.
///
/// It respects the `JUPYTER_RUNTIME_DIR` environment variable if set, otherwise it falls back
/// to the `XDG_RUNTIME_DIR` environment variable appended with `jupyter`, or defaults to
/// the user data directory appended with `runtime`.
pub fn runtime_dir() -> PathBuf {
    if let Ok(jupyter_runtime_dir) = env::var("JUPYTER_RUNTIME_DIR") {
        PathBuf::from(jupyter_runtime_dir)
    } else if let Ok(xdg_runtime_dir) = env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(xdg_runtime_dir).join("jupyter")
    } else {
        user_data_dir().join("runtime")
    }
}
