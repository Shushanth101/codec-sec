use async_trait::async_trait;
use tokio::sync::mpsc;
use crate::errors::AppError;

#[async_trait]
pub trait JobQueue: Send + Sync {
    async fn enqueue(&self, submission_id: String) -> Result<(), AppError>;
    async fn dequeue(&self) -> Result<String, AppError>;
}

pub struct InMemoryQueue {
    sender: mpsc::Sender<String>,
    receiver: tokio::sync::Mutex<mpsc::Receiver<String>>,
}

impl InMemoryQueue {
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = mpsc::channel(capacity);
        Self {
            sender: tx,
            receiver: tokio::sync::Mutex::new(rx),
        }
    }
}

#[async_trait]
impl JobQueue for InMemoryQueue {
    async fn enqueue(&self, submission_id: String) -> Result<(), AppError> {
        self.sender
            .send(submission_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to enqueue job: {}", e)))
    }

    async fn dequeue(&self) -> Result<String, AppError> {
        let mut rx = self.receiver.lock().await;
        rx.recv()
            .await
            .ok_or_else(|| AppError::Internal("Queue receiver closed".to_string()))
    }
}
