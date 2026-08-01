use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use crate::errors::AppError;
use crate::sandbox::isolate;

#[derive(Clone)]
pub struct SandboxManager {
    free_ids: Arc<Mutex<mpsc::Receiver<u32>>>,
    return_id: mpsc::Sender<u32>,
}

pub struct Sandbox {
    pub box_id: u32,
    pub box_path: PathBuf,
    manager: SandboxManager,
}

impl SandboxManager {
    pub fn new(max_sandboxes: u32) -> Self {
        let (tx, rx) = mpsc::channel(max_sandboxes as usize);
        for id in 1..=max_sandboxes {
            tx.try_send(id).unwrap();
        }
        Self {
            free_ids: Arc::new(Mutex::new(rx)),
            return_id: tx,
        }
    }

    pub async fn acquire(&self) -> Result<Sandbox, AppError> {
        let mut rx = self.free_ids.lock().await;
        if let Some(id) = rx.recv().await {
            // Drop lock early to let other requests process while we initialize isolate
            drop(rx);

            match isolate::init(id).await {
                Ok(path) => {
                    Ok(Sandbox {
                        box_id: id,
                        box_path: path,
                        manager: self.clone(),
                    })
                }
                Err(e) => {
                    // Put the ID back
                    let _ = self.return_id.send(id).await;
                    Err(e)
                }
            }
        } else {
            Err(AppError::SandboxError("Sandbox pool exhausted".to_string()))
        }
    }

    pub async fn release(&self, id: u32) {
        if let Err(e) = isolate::cleanup(id).await {
            tracing::error!("Failed to cleanup sandbox box-id {}: {:?}", id, e);
        }
        let _ = self.return_id.send(id).await;
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let id = self.box_id;
        let manager = self.manager.clone();
        tokio::spawn(async move {
            manager.release(id).await;
        });
    }
}
