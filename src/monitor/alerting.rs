use crate::api::client::ApiClient;
use crate::models::stats::{MonitoringAlert, AlertSeverity, AlertType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Gestor de alertas con deduplicación y rate limiting
pub struct AlertManager {
    api_client: ApiClient,
    recent_alerts: Arc<RwLock<HashMap<String, chrono::DateTime<chrono::Utc>>>>,
    min_alert_interval_seconds: i64,
}

impl AlertManager {
    pub fn new(api_client: ApiClient) -> Self {
        Self {
            api_client,
            recent_alerts: Arc::new(RwLock::new(HashMap::new())),
            min_alert_interval_seconds: 300, // 5 minutos por defecto
        }
    }
    
    pub fn with_min_interval(mut self, seconds: i64) -> Self {
        self.min_alert_interval_seconds = seconds;
        self
    }
    
    /// Enviar alerta con deduplicación
    pub async fn send_alert(&self, alert: MonitoringAlert) -> Result<(), String> {
        let alert_key = format!("{}:{:?}", alert.channel_id, alert.alert_type);
        
        // Verificar si ya se envió una alerta similar recientemente
        let mut recent = self.recent_alerts.write().await;
        
        let now = chrono::Utc::now();
        
        if let Some(last_sent) = recent.get(&alert_key) {
            let elapsed = (now - *last_sent).num_seconds();
            
            if elapsed < self.min_alert_interval_seconds {
                info!("Alerta duplicada suprimida: {} (última hace {} segundos)", 
                    alert_key, elapsed);
                return Ok(());
            }
        }
        
        // Enviar alerta
        match self.api_client.send_alert(&alert).await {
            Ok(_) => {
                info!("Alerta enviada: {:?} - {}", alert.alert_type, alert.message);
                recent.insert(alert_key, now);
                Ok(())
            }
            Err(e) => {
                warn!("Error enviando alerta: {}", e);
                Err(e.to_string())
            }
        }
    }
    
    /// Limpiar alertas antiguas del caché
    pub async fn cleanup_old_alerts(&self) {
        let mut recent = self.recent_alerts.write().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(self.min_alert_interval_seconds * 2);
        
        recent.retain(|_, timestamp| *timestamp > cutoff);
    }
    
    /// Crear alerta de FPS bajo
    pub fn alert_low_fps(channel_id: String, fps: f64) -> MonitoringAlert {
        MonitoringAlert::new(
            channel_id,
            AlertSeverity::Warning,
            AlertType::LowFps,
            format!("FPS bajo detectado: {:.1} fps", fps),
        )
    }
    
    /// Crear alerta de frames descartados
    pub fn alert_dropped_frames(channel_id: String, percentage: f64) -> MonitoringAlert {
        let severity = if percentage > 10.0 {
            AlertSeverity::Error
        } else {
            AlertSeverity::Warning
        };
        
        MonitoringAlert::new(
            channel_id,
            severity,
            AlertType::HighDroppedFrames,
            format!("Alto porcentaje de frames descartados: {:.1}%", percentage),
        )
    }
    
    /// Crear alerta de desviación de bitrate
    pub fn alert_bitrate_deviation(
        channel_id: String,
        current: f64,
        target: f64,
        deviation_percent: f64,
    ) -> MonitoringAlert {
        MonitoringAlert::new(
            channel_id,
            AlertSeverity::Warning,
            AlertType::BitrateDeviation,
            format!(
                "Desviación de bitrate: {:.1} kbps (objetivo: {:.1} kbps, {:.1}% desviación)",
                current, target, deviation_percent
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_alert_deduplication() {
        let api_client = ApiClient::new(
            "http://localhost:8080".to_string(),
            "test-key".to_string(),
        );
        
        let manager = AlertManager::new(api_client)
            .with_min_interval(60);
        
        let alert1 = AlertManager::alert_low_fps("ch1".to_string(), 15.0);
        let alert2 = AlertManager::alert_low_fps("ch1".to_string(), 14.0);
        
        // Primera alerta debería intentar enviarse
        let _ = manager.send_alert(alert1).await;
        
        // Segunda alerta (mismo tipo) debería ser suprimida
        let _ = manager.send_alert(alert2).await;
        
        // Verificar que hay una entrada en el caché
        let recent = manager.recent_alerts.read().await;
        assert!(recent.contains_key("ch1:LowFps"));
    }
}