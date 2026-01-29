use crate::core::manager::TranscoderManager;
use crate::api::client::ApiClient;
use crate::models::stats::{ChannelStats, StatsHistory, ChannelStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, error, debug};

/// Colector de estadísticas que acumula y reporta al backend
pub struct StatsCollector {
    manager: Arc<TranscoderManager>,
    api_client: ApiClient,
    report_interval: Duration,
    history: Arc<RwLock<HashMap<String, StatsHistory>>>,
    max_history_entries: usize,
}

impl StatsCollector {
    pub fn new(
        manager: Arc<TranscoderManager>,
        api_client: ApiClient,
        report_interval_seconds: u64,
    ) -> Self {
        Self {
            manager,
            api_client,
            report_interval: Duration::from_secs(report_interval_seconds),
            history: Arc::new(RwLock::new(HashMap::new())),
            max_history_entries: 360, // 1 hora con intervalo de 10s
        }
    }
    
    /// Iniciar colección y reporte de estadísticas
    pub async fn start(self) {
        info!("Iniciando Stats Collector (intervalo: {:?})", self.report_interval);
        
        let mut interval = interval(self.report_interval);
        
        loop {
            interval.tick().await;
            
            // Recolectar estadísticas actuales
            let stats = self.manager.get_all_stats().await;
            
            if stats.is_empty() {
                debug!("No hay canales activos para recolectar estadísticas");
                continue;
            }
            
            // Actualizar historial
            self.update_history(&stats).await;
            
            // Enviar al backend
            let stats_vec: Vec<ChannelStats> = stats.into_values().collect();
            
            match self.api_client.send_bulk_stats(stats_vec).await {
                Ok(_) => debug!("Estadísticas enviadas al backend"),
                Err(e) => error!("Error enviando estadísticas: {}", e),
            }
        }
    }
    
    /// Actualizar historial de estadísticas
    async fn update_history(&self, stats: &HashMap<String, ChannelStats>) {
        let mut history = self.history.write().await;
        
        for (channel_id, channel_stats) in stats {
            let entry = history
                .entry(channel_id.clone())
                .or_insert_with(|| StatsHistory::new(channel_id.clone(), self.max_history_entries));
            
            entry.add_entry(channel_stats);
        }
    }
    
    /// Obtener historial de un canal
    pub async fn get_history(&self, channel_id: &str) -> Option<StatsHistory> {
        let history = self.history.read().await;
        history.get(channel_id).cloned()
    }
    
    /// Obtener promedios de un canal
    pub async fn get_averages(&self, channel_id: &str) -> Option<ChannelAverages> {
        let history = self.history.read().await;
        
        if let Some(hist) = history.get(channel_id) {
            Some(ChannelAverages {
                average_fps: hist.average_fps(),
                average_bitrate_kbps: hist.average_bitrate(),
                bitrate_variance: hist.bitrate_variance(),
            })
        } else {
            None
        }
    }
    
    /// Limpiar historial antiguo
    pub async fn cleanup_old_history(&self, older_than_hours: u64) {
        let mut history = self.history.write().await;
        
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(older_than_hours as i64);
        
        for hist in history.values_mut() {
            hist.entries.retain(|entry| entry.timestamp > cutoff);
        }
        
        info!("Historial limpiado (más de {} horas)", older_than_hours);
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChannelAverages {
    pub average_fps: f64,
    pub average_bitrate_kbps: f64,
    pub bitrate_variance: f64,
}

/// Agregador de estadísticas
pub struct StatsAggregator;

impl StatsAggregator {
    /// Agregar estadísticas de múltiples canales
    pub fn aggregate(stats: &[ChannelStats]) -> AggregatedChannelStats {
        if stats.is_empty() {
            return AggregatedChannelStats::default();
        }
        
        let total_channels = stats.len();
        let running_channels = stats.iter()
            .filter(|s| s.status.is_active())
            .count();
        
        let total_fps: f64 = stats.iter().map(|s| s.current_fps).sum();
        let total_bitrate: f64 = stats.iter().map(|s| s.output_bitrate_kbps).sum();
        let total_frames: u64 = stats.iter().map(|s| s.frames_processed).sum();
        let total_dropped: u64 = stats.iter().map(|s| s.dropped_frames).sum();
        
        let average_speed = if running_channels > 0 {
            stats.iter()
                .filter(|s| s.status.is_active())
                .map(|s| s.speed)
                .sum::<f64>() / running_channels as f64
        } else {
            0.0
        };
        
        AggregatedChannelStats {
            total_channels,
            running_channels,
            total_fps,
            total_bitrate_kbps: total_bitrate,
            total_bitrate_mbps: total_bitrate / 1000.0,
            total_frames_processed: total_frames,
            total_dropped_frames: total_dropped,
            average_speed,
            timestamp: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct AggregatedChannelStats {
    pub total_channels: usize,
    pub running_channels: usize,
    pub total_fps: f64,
    pub total_bitrate_kbps: f64,
    pub total_bitrate_mbps: f64,
    pub total_frames_processed: u64,
    pub total_dropped_frames: u64,
    pub average_speed: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::stats::ChannelStatus;
    
    #[test]
    fn test_stats_aggregation() {
        let stats = vec![
            ChannelStats {
                channel_id: "ch1".to_string(),
                status: ChannelStatus::Running,
                frames_processed: 1000,
                current_fps: 30.0,
                output_bitrate_kbps: 4000.0,
                dropped_frames: 10,
                speed: 1.0,
                ..Default::default()
            },
            ChannelStats {
                channel_id: "ch2".to_string(),
                status: ChannelStatus::Running,
                frames_processed: 2000,
                current_fps: 30.0,
                output_bitrate_kbps: 3500.0,
                dropped_frames: 20,
                speed: 0.99,
                ..Default::default()
            },
        ];
        
        let aggregated = StatsAggregator::aggregate(&stats);
        
        assert_eq!(aggregated.total_channels, 2);
        assert_eq!(aggregated.running_channels, 2);
        assert_eq!(aggregated.total_fps, 60.0);
        assert_eq!(aggregated.total_bitrate_kbps, 7500.0);
        assert_eq!(aggregated.total_frames_processed, 3000);
    }
}

// Implementar Default para ChannelStats si no existe
impl Default for ChannelStats {
    fn default() -> Self {
        Self {
            channel_id: String::new(),
            status: ChannelStatus::Stopped,
            frames_processed: 0,
            current_bitrate_kbps: 0.0,
            current_fps: 0.0,
            uptime_seconds: 0,
            last_error: None,
            started_at: None,
            mode: String::new(),
            dropped_frames: 0,
            speed: 0.0,
            input_bitrate_kbps: 0.0,
            output_bitrate_kbps: 0.0,
        }
    }
}