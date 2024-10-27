use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::Arc;
use tokio::fs::File;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{Mutex, RwLock};

use crate::runtime_manager::RuntimeInstance;
use runtimelib::jupyter::client::{JupyterRuntime, RuntimeId};
use runtimelib::jupyter::KernelspecDir;

type RuntimeMap = Arc<RwLock<HashMap<RuntimeId, RuntimeInstance>>>;

struct ChildRuntimeData {
    pub process: tokio::process::Child,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
    pub exit_status: Option<ExitStatus>,
}

#[derive(Clone)]
pub struct ChildRuntime(Arc<Mutex<ChildRuntimeData>>);

impl ChildRuntime {
    pub async fn new(
        kernelspec: KernelspecDir,
        rt: &JupyterRuntime,
        map: RuntimeMap,
    ) -> Result<Self> {
        let cipath = &rt.connection_file;
        let stdout_path = cipath.with_extension("stdout");
        let stdout = File::create(stdout_path.clone()).await?.into_std().await;
        let stderr_path = cipath.with_extension("stderr");
        let stderr = File::create(stderr_path.clone()).await?.into_std().await;

        let mut command = kernelspec
            .command(
                &rt.connection_file,
                Some(stdout.into()),
                Some(stderr.into()),
            )
            .await?;

        let new = Self(Arc::new(Mutex::new(ChildRuntimeData {
            process: command.spawn()?,
            stdout_path,
            stderr_path,
            exit_status: None,
        })));
        new.spawn_child_reaper(rt.id, map.clone());
        new.spawn_child_signal_handler();
        Ok(new)
    }

    async fn child_reaper_cleanup(
        runtime_map: RuntimeMap,
        id: &RuntimeId,
        remove_files: &Vec<PathBuf>,
    ) {
        let mut lock = runtime_map.write().await;
        let rt = lock.remove(id);
        drop(lock);
        if let Some(rt) = rt {
            match tokio::fs::remove_file(rt.runtime.connection_file).await {
                Ok(_) => {}
                Err(e) => log::error!("Error removing connection file: {}", e),
            }
            for p in remove_files {
                match tokio::fs::remove_file(p).await {
                    Ok(_) => {}
                    Err(e) => log::error!("Error removing log file: {}", e),
                }
            }
        }
    }

    fn spawn_child_reaper(&self, id: RuntimeId, runtime_map: RuntimeMap) {
        let this = self.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let mut child_proc = this.0.lock().await;
                match child_proc.process.try_wait() {
                    Ok(None) => {
                        // child lives, fall through to next loop iteration
                    }
                    Ok(Some(status)) => {
                        log::info!("Runtime finished: {}", status);
                        child_proc.exit_status = Some(status);
                        let logs_to_delete = if status.success() {
                            vec![
                                child_proc.stdout_path.clone(),
                                child_proc.stderr_path.clone(),
                            ]
                        } else {
                            vec![]
                        };
                        Self::child_reaper_cleanup(runtime_map.clone(), &id, &logs_to_delete).await;
                        break;
                    }
                    Err(e) => {
                        log::error!("Error waiting for runtime: {}", e);
                        break;
                    }
                }
            }
        });
    }

    fn spawn_child_signal_handler(&self) {
        let this = self.clone();
        let mut stream = signal(SignalKind::interrupt()).unwrap();
        tokio::spawn(async move {
            stream.recv().await;

            let mut child = this.0.lock().await;
            if let Some(exit_status) = child.exit_status {
                log::debug!(
                    "Child SIGINT handler: child previously finished: {}",
                    exit_status
                );
                return;
            }
            log::debug!("Child SIGINT handler: sending SIGKILL to child process");

            // No need to wait around to see if it worked or not
            let _ = child.process.start_kill();
        });
    }
}
