use std::collections::HashMap;
use std::process::ExitStatus;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{Mutex, RwLock};

use crate::runtime_manager::RuntimeInstance;
use runtimelib::jupyter::client::RuntimeId;

type RuntimeMap = Arc<RwLock<HashMap<RuntimeId, RuntimeInstance>>>;

struct ChildRuntime {
    pub process: tokio::process::Child,
    pub exit_status: Option<ExitStatus>,
}

#[derive(Clone)]
pub struct ChildRuntimeLock(Arc<Mutex<ChildRuntime>>);

impl ChildRuntimeLock {
    pub fn new(process: tokio::process::Child, id: RuntimeId, map: RuntimeMap) -> Self {
        let new = Self(Arc::new(Mutex::new(ChildRuntime {
            process,
            exit_status: None,
        })));
        new.spawn_child_reaper(id, map.clone());
        new.spawn_child_signal_handler();
        new
    }

    async fn child_reaper_cleanup(runtime_map: RuntimeMap, id: &RuntimeId) {
        let mut lock = runtime_map.write().await;
        let rt = lock.remove(&id);
        drop(lock);
        if let Some(rt) = rt {
            if let Some(_) = rt.child {
                tokio::fs::remove_file(rt.runtime.connection_file)
                    .await
                    .unwrap();
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
                        Self::child_reaper_cleanup(runtime_map.clone(), &id).await;
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
