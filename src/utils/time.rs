use chrono::{DateTime, Duration, Utc};

/// Utilidades para manejo de tiempo y timestamps
pub struct TimeHelper;

impl TimeHelper {
    /// Formatear duración en formato humano legible
    pub fn format_duration(seconds: u64) -> String {
        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;
        
        if days > 0 {
            format!("{}d {}h {}m {}s", days, hours, minutes, secs)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, secs)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, secs)
        } else {
            format!("{}s", secs)
        }
    }
    
    /// Convertir timestamp MPEG-TS (90kHz) a segundos
    pub fn mpegts_to_seconds(pts: i64) -> f64 {
        pts as f64 / 90_000.0
    }
    
    /// Convertir segundos a timestamp MPEG-TS
    pub fn seconds_to_mpegts(seconds: f64) -> i64 {
        (seconds * 90_000.0) as i64
    }
    
    /// Calcular uptime desde un timestamp
    pub fn uptime_since(started_at: DateTime<Utc>) -> Duration {
        Utc::now() - started_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_format_duration() {
        assert_eq!(TimeHelper::format_duration(30), "30s");
        assert_eq!(TimeHelper::format_duration(90), "1m 30s");
        assert_eq!(TimeHelper::format_duration(3665), "1h 1m 5s");
        assert_eq!(TimeHelper::format_duration(90000), "1d 1h 0m 0s");
    }
    
    #[test]
    fn test_mpegts_conversion() {
        let pts = 900_000; // 10 segundos
        let seconds = TimeHelper::mpegts_to_seconds(pts);
        assert_eq!(seconds, 10.0);
        
        let back = TimeHelper::seconds_to_mpegts(seconds);
        assert_eq!(back, pts);
    }
}