use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;
use uuid::Uuid;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ProgressStats {
    pub total_bytes: u64,
    pub processed_bytes: u64,
    pub start_time: Instant,
    pub current_speed: f64,
    pub percent_complete: f32,
    pub estimated_time_remaining: Duration,
}

#[derive(Debug)]
pub struct ProgressTracker {
    operation: Arc<Mutex<HashMap<Uuid, ProgressStats>>>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self { operation: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub async fn start_operation(&self, total_bytes: u64) -> Uuid {
        let operation_id = Uuid::new_v4();
        let stats = ProgressStats {
            total_bytes,
            processed_bytes: 0,
            start_time: Instant::now(),
            current_speed: 0.0,
            percent_complete: 0.0,
            estimated_time_remaining: Duration::from_secs(0),
        };

        let mut operations = self.operation.lock().await;
        operations.insert(operation_id, stats);
        operation_id
    }

    pub async fn update_progress(&self, operation_id: &Uuid, processed_bytes: u64) -> Option<ProgressStats> {
        let mut operations = self.operation.lock().await;

        if let Some(stats) = operations.get_mut(operation_id) {
            let elapsed = stats.start_time.elapsed();
            let elapsed_secs = elapsed.as_secs_f64();

            stats.processed_bytes = processed_bytes;
            stats.current_speed = if elapsed_secs > 0.0 {
                processed_bytes as f64 / elapsed_secs
            } else {
                0.0
            };

            stats.percent_complete = (processed_bytes as f32 / stats.total_bytes as f32) * 100.0;
            let remaining_bytes = stats.total_bytes - processed_bytes;
            stats.estimated_time_remaining  = if stats.current_speed > 0.0 {
                Duration::from_secs_f64(remaining_bytes as f64 / stats.current_speed)
            }else {
                Duration::from_secs(0)
            };

            Some(stats.clone())
        }else {
            None
        }

    }

    pub async fn complete_operation(&self, operation_id: &Uuid) {
        let mut operations = self.operation.lock().await;
        operations.remove(operation_id);
    }

    pub async fn get_progress(&self, operation_id: &Uuid) -> Option<ProgressStats> {
        let operations = self.operation.lock().await;
        operations.get(operation_id).cloned()
    }
}

pub trait ProgressFormatter {
    fn format_progress(&self) -> String;
    fn format_speed(&self) -> String;
    fn format_time_remaining(&self) -> String;
}

impl ProgressFormatter for ProgressStats {
    fn format_progress(&self) -> String {
        format!("{:.1}% ({}/{} bytes)", 
            self.percent_complete,
            self.processed_bytes,
            self.total_bytes
        )
    }

    fn format_speed(&self) -> String {
        if self.current_speed >= 1_000_000.0 {
            format!("{:.2} MB/s", self.current_speed / 1_000_000.0)
        } else if self.current_speed >= 1_000.0 {
            format!("{:.2} KB/s", self.current_speed / 1_000.0)
        } else {
            format!("{:.0} B/s", self.current_speed)
        }
    }

    fn format_time_remaining(&self) -> String {
        let secs = self.estimated_time_remaining.as_secs();
        if secs >= 3600 {
            format!("{:.1}h remaining", secs as f64 / 3600.0)
        } else if secs >= 60 {
            format!("{:.1}m remaining", secs as f64 / 60.0)
        } else {
            format!("{}s remaining", secs)
        }
    }
}