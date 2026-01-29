use crate::core::manager::TranscoderManager;
use crate::api::client::ApiClient;
use crate::models::stats::{ChannelStatus, MonitoringAlert, AlertSeverity, AlertType};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};

/// Servicio de health checking automático
pub struct HealthChecker {
    manager: Arc<TranscoderManager>,
    api_client: ApiClient,
    check_interval: Duration,
    restart_on_failure: bool,
    max_restart_attempts: u32,
}

impl HealthChecker {
    pub fn new(manager: Arc<TranscoderManager>, api_client: ApiClient) -> Self {
        Self {
            manager,
            api_client,
            check_interval: Duration::from_secs(30),
            restart_on_failure: true,
            max_restart_attempts: 3,
        }
    }
    
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }
    
    pub fn with_auto_restart(mut self, enabled: bool) -> Self {
        self.restart_on_failure = enabled;
        self
    }
    
    /// Iniciar servicio de health checking
    pub async fn start(self) {
        info!("Iniciando Health Checker (intervalo: {:?})", self.check_interval);
        
        let mut interval = interval(self.check_interval);
        let mut restart_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
        
        loop {
            interval.tick().await;
            
            // Verificar salud de todos los canales
            let health_results = self.manager.check_all_health().await;
            
            for (channel_id, is_healthy) in health_results {
                if !is_healthy {
                    warn!("Canal {} no está saludable", channel_id);
                    
                    // Obtener estadísticas para diagnóstico
                    if let Some(stats) = self.manager.get_channel_stats(&channel_id).await {
                        self.diagnose_channel(&channel_id, &stats).await;
                        
                        // Intentar reiniciar si está habilitado
                        if self.restart_on_failure {
                            let restart_count = restart_counts.entry(channel_id.clone()).or_insert(0);
                            
                            if *restart_count < self.max_restart_attempts {
                                info!("Intentando reiniciar canal {} (intento {}/{})", 
                                    channel_id, *restart_count + 1, self.max_restart_attempts);
                                
                                match self.manager.restart_channel(&channel_id).await {
                                    Ok(_) => {
                                        info!("Canal {} reiniciado exitosamente", channel_id);
                                        *restart_count += 1;
                                    }
                                    Err(e) => {
                                        error!("Error reiniciando canal {}: {}", channel_id, e);
                                        
                                        // Crear alerta crítica
                                        let alert = MonitoringAlert::new(
                                            channel_id.clone(),
                                            AlertSeverity::Critical,
                                            AlertType::ProcessCrash,
                                            format!("No se pudo reiniciar canal después de {} intentos: {}", 
                                                *restart_count, e),
                                        );
                                        
                                        if let Err(e) = self.api_client.send_alert(&alert).await {
                                            error!("Error enviando alerta: {}", e);
                                        }
                                    }
                                }
                            } else {
                                error!("Canal {} ha alcanzado el máximo de reintentos", channel_id);
                                
                                // Alerta de máximo de reintentos alcanzado
                                let alert = MonitoringAlert::new(
                                    channel_id.clone(),
                                    AlertSeverity::Critical,
                                    AlertType::ProcessCrash,
                                    format!("Canal alcanzó el máximo de {} reintentos", 
                                        self.max_restart_attempts),
                                );
                                
                                if let Err(e) = self.api_client.send_alert(&alert).await {
                                    error!("Error enviando alerta: {}", e);
                                }
                            }
                        }
                    }
                } else {
                    // Canal saludable, resetear contador de reintentos
                    restart_counts.remove(&channel_id);
                }
            }
        }
    }
    
    /// Diagnosticar problemas específicos de un canal
    async fn diagnose_channel(&self, channel_id: &str, stats: &crate::models::stats::ChannelStats) {
        let mut issues: Vec<String> = Vec::new();
        
        // Verificar FPS bajo
        if stats.current_fps < 10.0 && stats.uptime_seconds > 30 {
            issues.push("FPS extremadamente bajo".to_string());
            
            let alert = MonitoringAlert::new(
                channel_id.to_string(),
                AlertSeverity::Error,
                AlertType::LowFps,
                format!("FPS bajo: {:.1} fps", stats.current_fps),
            );
            
            let _ = self.api_client.send_alert(&alert).await;
        }
        
        // Verificar frames descartados
        if stats.dropped_frames_percentage() > 5.0 {
            issues.push("Alto porcentaje de frames descartados".to_string());
            
            let alert = MonitoringAlert::new(
                channel_id.to_string(),
                AlertSeverity::Warning,
                AlertType::HighDroppedFrames,
                format!("Frames descartados: {:.1}%", stats.dropped_frames_percentage()),
            );
            
            let _ = self.api_client.send_alert(&alert).await;
        }
        
        // Verificar velocidad de procesamiento
        if stats.speed > 0.0 && stats.speed < 0.9 {
            issues.push("Velocidad de procesamiento baja".to_string());
            
            let alert = MonitoringAlert::new(
                channel_id.to_string(),
                AlertSeverity::Warning,
                AlertType::ProcessCrash,
                format!("Velocidad de procesamiento: {:.2}x", stats.speed),
            );
            
            let _ = self.api_client.send_alert(&alert).await;
        }
        
        // Verificar error en status
        if let ChannelStatus::Error(ref error_msg) = stats.status {
            issues.push(format!("Error: {}", error_msg));
        }
        
        if !issues.is_empty() {
            warn!("Problemas detectados en canal {}:", channel_id);
            for issue in issues {
                warn!("  - {}", issue);
            }
        }
    }
}

/// Health check simplificado para endpoints HTTP
pub struct SimpleHealthCheck;

impl SimpleHealthCheck {
    /// Verificar salud básica del sistema
    pub async fn check() -> HealthCheckResult {
        let mut result = HealthCheckResult {
            healthy: true,
            checks: Vec::new(),
        };
        
        // 1. Verificar FFmpeg
        result.checks.push(ComponentHealth {
            component: "ffmpeg".to_string(),
            healthy: crate::utils::ffmpeg::FfmpegHelper::is_installed(),
            message: if crate::utils::ffmpeg::FfmpegHelper::is_installed() {
                "FFmpeg disponible".to_string()
            } else {
                "FFmpeg no encontrado".to_string()
            },
        });
        
        // 2. Verificar memoria disponible
        #[cfg(target_os = "linux")]
        {
            if let Ok(mem_info) = Self::check_memory() {
                result.checks.push(mem_info);
            }
        }
        
        // 3. Verificar espacio en disco
        if let Ok(disk_info) = Self::check_disk_space() {
            result.checks.push(disk_info);
        }
        
        // Determinar salud general
        result.healthy = result.checks.iter().all(|c| c.healthy);
        
        result
    }
    
    #[cfg(target_os = "linux")]
    fn check_memory() -> Result<ComponentHealth, std::io::Error> {
        use std::fs;
        
        let meminfo = fs::read_to_string("/proc/meminfo")?;
        
        let mut mem_total = 0u64;
        let mut mem_available = 0u64;
        
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                mem_total = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            } else if line.starts_with("MemAvailable:") {
                mem_available = line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
            }
        }
        
        let percent_available = if mem_total > 0 {
            (mem_available as f64 / mem_total as f64) * 100.0
        } else {
            0.0
        };
        
        let healthy = percent_available > 10.0; // Al menos 10% libre
        
        Ok(ComponentHealth {
            component: "memory".to_string(),
            healthy,
            message: format!("{:.1}% disponible ({} MB)", 
                percent_available, mem_available / 1024),
        })
    }
    
    fn check_disk_space() -> Result<ComponentHealth, std::io::Error> {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            
            let output = Command::new("df")
                .args(&["-h", "/"])
                .output()?;
            
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            
            if lines.len() >= 2 {
                let parts: Vec<&str> = lines[1].split_whitespace().collect();
                if parts.len() >= 5 {
                    let usage = parts[4].trim_end_matches('%');
                    if let Ok(usage_percent) = usage.parse::<f64>() {
                        let healthy = usage_percent < 90.0;
                        
                        return Ok(ComponentHealth {
                            component: "disk".to_string(),
                            healthy,
                            message: format!("{}% usado", usage_percent),
                        });
                    }
                }
            }
        }
        
        Ok(ComponentHealth {
            component: "disk".to_string(),
            healthy: true,
            message: "No verificado".to_string(),
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthCheckResult {
    pub healthy: bool,
    pub checks: Vec<ComponentHealth>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ComponentHealth {
    pub component: String,
    pub healthy: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_simple_health_check() {
        let result = SimpleHealthCheck::check().await;
        println!("Health check result: {:?}", result);
        // El test pasa siempre, solo para verificar que no crashea
    }
}