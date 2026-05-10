pub mod ws;

use tokio::sync::broadcast;

#[derive(Clone)]
pub struct Hub {
    tx: broadcast::Sender<serde_json::Value>,
}

impl Hub {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(1024);
        Self { tx }
    }

    pub fn sender(&self) -> broadcast::Sender<serde_json::Value> {
        self.tx.clone()
    }
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}
