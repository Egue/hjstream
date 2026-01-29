use crate::api::client::ApiClient;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, error};

/// Servicio de heartbeat periódico al backend
pub struct HeartbeatService {
    api_client: Arc<ApiClient>,
    client_id: String,
    interval_seconds: u64,
}

impl HeartbeatService {
    pub fn new(api_client: Arc<ApiClient>, client_id: String, interval_seconds: u64) -> Self {
        Self {
            api_client,
            client_id,
            interval_seconds,
        }
    }
    
    /// Iniciar servicio de heartbeat
    pub async fn start(self) {
        info!("Iniciando servicio de heartbeat (intervalo: {}s)", self.interval_seconds);
        
        let mut interval = interval(Duration::from_secs(self.interval_seconds));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.api_client.send_heartbeat(&self.client_id).await {
                error!("Error enviando heartbeat: {}", e);
            }
        }
    }
}