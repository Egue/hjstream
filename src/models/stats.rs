use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Estado de un canal de transcodificación
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "message")]
pub enum ChannelStatus {
    /// Canal detenido
    Stopped,
    
    /// Canal iniciando
    Starting,
    
    /// Canal corriendo normalmente
    Running,
    
    /// Canal pausado
    Paused,
    
    /// Canal en estado de error
    Error(String),
    
    /// Canal reiniciando
    Restarting,
    
    /// Canal en proceso de apagado
    Stopping,
}

impl ChannelStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, ChannelStatus::Running | ChannelStatus::Starting)
    }
    
    pub fn is_error(&self) -> bool {
        matches!(self, ChannelStatus::Error(_))
    }
    
    pub fn as_str(&self) -> &str {
        match self {
            ChannelStatus::Stopped => "stopped",
            ChannelStatus::Starting => "starting",
            ChannelStatus::Running => "running",
            ChannelStatus::Paused => "paused",
            ChannelStatus::Error(_) => "error",
            ChannelStatus::Restarting => "restarting",
            ChannelStatus::Stopping => "stopping",
        }
    }
}

/// Estadísticas de un canal de transcodificación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStats {
    /// ID del canal
    pub channel_id: String,
    
    /// Estado actual
    pub status: ChannelStatus,
    
    /// Número de frames procesados
    pub frames_processed: u64,
    
    /// Bitrate actual de salida (kbps)
    pub current_bitrate_kbps: f64,
    
    /// FPS actual
    pub current_fps: f64,
    
    /// Tiempo de actividad en segundos
    pub uptime_seconds: u64,
    
    /// Último error (si existe)
    pub last_error: Option<String>,
    
    /// Timestamp de inicio
    pub started_at: Option<DateTime<Utc>>,
    
    /// Modo de operación
    pub mode: String,
    
    /// Frames descartados
    pub dropped_frames: u64,
    
    /// Velocidad de procesamiento (1.0 = tiempo real)
    pub speed: f64,
    
    /// Bitrate de entrada (kbps)
    pub input_bitrate_kbps: f64,
    
    /// Bitrate de salida (kbps)
    pub output_bitrate_kbps: f64,
}

impl ChannelStats {
    pub fn new(channel_id: String, mode: String) -> Self {
        Self {
            channel_id,
            status: ChannelStatus::Stopped,
            frames_processed: 0,
            current_bitrate_kbps: 0.0,
            current_fps: 0.0,
            uptime_seconds: 0,
            last_error: None,
            started_at: None,
            mode,
            dropped_frames: 0,
            speed: 0.0,
            input_bitrate_kbps: 0.0,
            output_bitrate_kbps: 0.0,
        }
    }
    
    /// Calcula el porcentaje de frames descartados
    pub fn dropped_frames_percentage(&self) -> f64 {
        if self.frames_processed == 0 {
            0.0
        } else {
            (self.dropped_frames as f64 / self.frames_processed as f64) * 100.0
        }
    }
    
    /// Verifica si el canal está saludable
    pub fn is_healthy(&self) -> bool {
        if !self.status.is_active() {
            return false;
        }
        
        // Si lleva más de 30 segundos y no procesa frames, no está saludable
        if self.uptime_seconds > 30 && self.current_fps < 1.0 {
            return false;
        }
        
        // Si tiene muchos frames descartados (>5%), no está saludable
        if self.dropped_frames_percentage() > 5.0 {
            return false;
        }
        
        // Si la velocidad es muy baja (<0.95x), hay problemas
        if self.speed > 0.0 && self.speed < 0.95 {
            return false;
        }
        
        true
    }
    
    /// Obtiene un resumen en texto del estado
    pub fn summary(&self) -> String {
        format!(
            "{} - {} - {:.1} fps - {:.1} kbps - {} frames ({} dropped)",
            self.channel_id,
            self.status.as_str(),
            self.current_fps,
            self.output_bitrate_kbps,
            self.frames_processed,
            self.dropped_frames
        )
    }
}

/// Estadísticas agregadas de múltiples canales
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedStats {
    pub total_channels: usize,
    pub running_channels: usize,
    pub stopped_channels: usize,
    pub error_channels: usize,
    pub total_fps: f64,
    pub total_bitrate_kbps: f64,
    pub total_frames_processed: u64,
    pub total_dropped_frames: u64,
    pub average_speed: f64,
    pub timestamp: DateTime<Utc>,
}

impl AggregatedStats {
    pub fn from_channels(stats: &[ChannelStats]) -> Self {
        let total_channels = stats.len();
        let running_channels = stats.iter().filter(|s| s.status.is_active()).count();
        let stopped_channels = stats.iter().filter(|s| matches!(s.status, ChannelStatus::Stopped)).count();
        let error_channels = stats.iter().filter(|s| s.status.is_error()).count();
        
        let total_fps: f64 = stats.iter().map(|s| s.current_fps).sum();
        let total_bitrate_kbps: f64 = stats.iter().map(|s| s.output_bitrate_kbps).sum();
        let total_frames_processed: u64 = stats.iter().map(|s| s.frames_processed).sum();
        let total_dropped_frames: u64 = stats.iter().map(|s| s.dropped_frames).sum();
        
        let average_speed = if running_channels > 0 {
            stats.iter()
                .filter(|s| s.status.is_active() && s.speed > 0.0)
                .map(|s| s.speed)
                .sum::<f64>() / running_channels as f64
        } else {
            0.0
        };
        
        Self {
            total_channels,
            running_channels,
            stopped_channels,
            error_channels,
            total_fps,
            total_bitrate_kbps,
            total_frames_processed,
            total_dropped_frames,
            average_speed,
            timestamp: Utc::now(),
        }
    }
}

/// Historial de estadísticas para análisis temporal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsHistory {
    pub channel_id: String,
    pub entries: Vec<StatsEntry>,
    pub max_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsEntry {
    pub timestamp: DateTime<Utc>,
    pub fps: f64,
    pub bitrate_kbps: f64,
    pub dropped_frames: u64,
    pub speed: f64,
}

impl StatsHistory {
    pub fn new(channel_id: String, max_entries: usize) -> Self {
        Self {
            channel_id,
            entries: Vec::with_capacity(max_entries),
            max_entries,
        }
    }
    
    pub fn add_entry(&mut self, stats: &ChannelStats) {
        let entry = StatsEntry {
            timestamp: Utc::now(),
            fps: stats.current_fps,
            bitrate_kbps: stats.output_bitrate_kbps,
            dropped_frames: stats.dropped_frames,
            speed: stats.speed,
        };
        
        self.entries.push(entry);
        
        // Mantener solo las últimas N entradas
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }
    
    pub fn average_fps(&self) -> f64 {
        if self.entries.is_empty() {
            0.0
        } else {
            self.entries.iter().map(|e| e.fps).sum::<f64>() / self.entries.len() as f64
        }
    }
    
    pub fn average_bitrate(&self) -> f64 {
        if self.entries.is_empty() {
            0.0
        } else {
            self.entries.iter().map(|e| e.bitrate_kbps).sum::<f64>() / self.entries.len() as f64
        }
    }
    
    pub fn bitrate_variance(&self) -> f64 {
        if self.entries.len() < 2 {
            return 0.0;
        }
        
        let avg = self.average_bitrate();
        let variance = self.entries.iter()
            .map(|e| (e.bitrate_kbps - avg).powi(2))
            .sum::<f64>() / self.entries.len() as f64;
        
        variance.sqrt()
    }
}

/// Alerta de monitoreo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringAlert {
    pub channel_id: String,
    pub severity: AlertSeverity,
    pub alert_type: AlertType,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    LowFps,
    HighDroppedFrames,
    BitrateDeviation,
    ProcessCrash,
    NetworkIssue,
    ConfigurationError,
    HardwareError,
}

impl MonitoringAlert {
    pub fn new(
        channel_id: String,
        severity: AlertSeverity,
        alert_type: AlertType,
        message: String,
    ) -> Self {
        Self {
            channel_id,
            severity,
            alert_type,
            message,
            timestamp: Utc::now(),
            acknowledged: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_channel_stats_health() {
        let mut stats = ChannelStats::new("test".to_string(), "transcoder".to_string());
        stats.status = ChannelStatus::Running;
        stats.uptime_seconds = 60;
        stats.current_fps = 30.0;
        stats.frames_processed = 1800;
        stats.dropped_frames = 10;
        stats.speed = 1.0;
        
        assert!(stats.is_healthy());
        
        // Simular muchos frames descartados
        stats.dropped_frames = 200; // >10%
        assert!(!stats.is_healthy());
    }
    
    #[test]
    fn test_dropped_frames_percentage() {
        let mut stats = ChannelStats::new("test".to_string(), "transcoder".to_string());
        stats.frames_processed = 1000;
        stats.dropped_frames = 50;
        
        assert_eq!(stats.dropped_frames_percentage(), 5.0);
    }
    
    #[test]
    fn test_aggregated_stats() {
        let stats1 = ChannelStats {
            channel_id: "ch1".to_string(),
            status: ChannelStatus::Running,
            frames_processed: 1000,
            current_fps: 30.0,
            output_bitrate_kbps: 4000.0,
            dropped_frames: 10,
            speed: 1.0,
            ..ChannelStats::new("ch1".to_string(), "transcoder".to_string())
        };
        
        let stats2 = ChannelStats {
            channel_id: "ch2".to_string(),
            status: ChannelStatus::Running,
            frames_processed: 2000,
            current_fps: 30.0,
            output_bitrate_kbps: 3500.0,
            dropped_frames: 20,
            speed: 0.99,
            ..ChannelStats::new("ch2".to_string(), "transcoder".to_string())
        };
        
        let agg = AggregatedStats::from_channels(&[stats1, stats2]);
        
        assert_eq!(agg.total_channels, 2);
        assert_eq!(agg.running_channels, 2);
        assert_eq!(agg.total_fps, 60.0);
        assert_eq!(agg.total_bitrate_kbps, 7500.0);
        assert_eq!(agg.total_frames_processed, 3000);
        assert_eq!(agg.total_dropped_frames, 30);
    }
}