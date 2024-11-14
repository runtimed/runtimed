use anyhow::{Context, Result};
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
        .context("Failed to execute `jupyter --paths --json` command")?;

    if output.status.success() {
        let paths: Value = serde_json::from_slice(&output.stdout)
            .context("Failed to parse JSON from jupyter output")?;
        Ok(paths)
    } else {
        Err(anyhow::anyhow!(
            "Jupyter command failed with status: {:?}",
            output.status
        ))
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
            .context("Failed to get home directory")?
            .join("Library/Jupyter"))
    } else if cfg!(windows) {
        Ok(PathBuf::from(env::var("APPDATA").context("Failed to get APPDATA")?).join("jupyter"))
    } else {
        // TODO: Respect XDG_DATA_HOME if set
        match data_dir() {
            None => Ok(home_dir()
                .context("Failed to get home directory")?
                .join(".local/share")),
            Some(data_dir) => {
                return Ok(data_dir.join("jupyter"));
            }
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
