use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use tracing::{info, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub client_id: String,
    pub name: String,
    pub location: String,
    pub backend: BackendConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub performance: PerformanceConfig,
    pub auto_start_channels: bool,
    pub config_backup_enabled: bool,
    pub config_sync_interval_seconds: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            client_id: uuid::Uuid::new_v4().to_string(),
            name: "Default Client".to_string(),
            location: "Unknown".to_string(),
            backend: BackendConfig::default(),
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
            performance: PerformanceConfig::default(),
            auto_start_channels: false,
            config_backup_enabled: true,
            config_sync_interval_seconds: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub url: String,
    pub api_key: String,
    pub heartbeat_interval_seconds: u64,
    pub reconnect_attempts: u32,
    pub reconnect_delay_seconds: u64,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8080".to_string(),
            api_key: "default-api-key".to_string(),
            heartbeat_interval_seconds: 30,
            reconnect_attempts: 5,
            reconnect_delay_seconds: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_metrics: bool,
    pub metrics_port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            enable_metrics: true,
            metrics_port: 9090,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: String,
    pub max_size_mb: u64,
    pub max_files: u32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file: "logs/hjstream.log".to_string(),
            max_size_mb: 100,
            max_files: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub max_channels: u32,
    pub worker_threads: u32,
    pub buffer_size_mb: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_channels: 16,
            worker_threads: 4,
            buffer_size_mb: 256,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub mode: String,
    pub input: InputConfig,
    pub output: OutputConfig,
    
    // Transcoding es opcional
    #[serde(default)]
    pub transcoding: Option<TranscodingConfig>,
    
    // Monitoreo básico es opcional
    #[serde(default)]
    pub monitoring: Option<MonitoringConfig>,
    
    // Failover es opcional
    #[serde(default)]
    pub failover: Option<FailoverConfig>,
    
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Configuración de transcodificación (opcional)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodingConfig {
    pub enabled: bool,
    pub video: Option<VideoConfig>,
    pub audio: Option<AudioConfig>,
    pub mpegts: Option<MpegTsConfig>,
    pub analysis: Option<AnalysisConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub r#type: String,
    pub url: String,
    pub latency_ms: Option<u32>,
    pub max_bandwidth_mbps: Option<u32>,
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub r#type: String,
    pub url: String,
    pub local_interface: Option<String>,
    pub ttl: Option<u8>,
    pub packet_size: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub codec: String,
    pub profile: String,
    pub level: String,
    pub bitrate_kbps: u32,
    pub max_bitrate_kbps: u32,
    pub buffer_size_kb: u32,
    pub framerate: u32,
    pub gop_size: u32,
    pub preset: String,
    pub tune: String,
    pub rate_control: String,
    pub hardware_acceleration: HardwareAccelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareAccelConfig {
    pub enabled: bool,
    pub r#type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub codec: String,
    pub bitrate_kbps: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpegTsConfig {
    pub pmt_pid: u16,
    pub video_pid: u16,
    pub audio_pid: u16,
    pub pcr_pid: u16,
    pub service_id: u16,
    pub service_name: String,
    pub provider_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub auto_detect: bool,
    pub force_transcode: bool,
    pub passthrough_if_compatible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub report_interval_seconds: u64,
    pub alert_on_error: bool,
    pub alert_on_bitrate_deviation_percent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    pub auto_restart: bool,
    pub max_restart_attempts: u32,
    pub restart_delay_seconds: u64,
    pub backup_input_url: Option<String>,
}

pub struct ConfigLoader {
    config_dir: PathBuf,
    backup_dir: PathBuf,
}

impl Clone for ConfigLoader {
    fn clone(&self) -> Self {
        Self {
            config_dir: self.config_dir.clone(),
            backup_dir: self.backup_dir.clone(),
        }
    }
}

impl ConfigLoader {
    pub fn new(config_dir: impl AsRef<Path>) -> Self {
        let config_dir = config_dir.as_ref().to_path_buf();
        let backup_dir = config_dir.join("backup");
        
        Self {
            config_dir,
            backup_dir,
        }
    }
    
    pub fn load_client_config(&self) -> Result<ClientConfig> {
        let path = self.config_dir.join("client.json");
        info!("Cargando configuración del cliente desde: {:?}", path);
        
        let content = fs::read_to_string(&path)
            .context("Error leyendo archivo de configuración del cliente")?;
        
        let config: ClientConfig = serde_json::from_str(&content)
            .context("Error parseando JSON de configuración del cliente")?;
        
        info!("Configuración del cliente cargada: {}", config.client_id);
        Ok(config)
    }
    
    pub fn load_channel_config(&self, channel_id: &str) -> Result<ChannelConfig> {
        let path = self.config_dir.join("channels").join(format!("{}.json", channel_id));
        info!("Cargando configuración del canal: {:?}", path);
        
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Error leyendo config de canal: {}", channel_id))?;
        
        let config: ChannelConfig = serde_json::from_str(&content)
            .with_context(|| format!("Error parseando JSON del canal: {}", channel_id))?;
        
        info!("Canal configurado: {} ({})", config.id, config.name);
        Ok(config)
    }
    
    pub fn load_all_channels(&self) -> Result<Vec<ChannelConfig>> {
        let channels_dir = self.config_dir.join("channels");
        let mut channels = Vec::new();
        
        if !channels_dir.exists() {
            warn!("Directorio de canales no existe: {:?}", channels_dir);
            return Ok(channels);
        }
        
        for entry in fs::read_dir(&channels_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_channel_from_path(&path) {
                    Ok(config) => {
                        if config.enabled {
                            channels.push(config);
                        } else {
                            info!("Canal deshabilitado: {}", config.id);
                        }
                    }
                    Err(e) => {
                        error!("Error cargando canal desde {:?}: {}", path, e);
                    }
                }
            }
        }
        
        info!("Canales cargados: {}", channels.len());
        Ok(channels)
    }
    
    fn load_channel_from_path(&self, path: &Path) -> Result<ChannelConfig> {
        let content = fs::read_to_string(path)?;
        let config: ChannelConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    pub fn save_channel_config(&self, config: &ChannelConfig) -> Result<()> {
        let path = self.config_dir.join("channels").join(format!("{}.json", config.id));
        
        // Crear backup si está habilitado
        if path.exists() {
            self.backup_config(&path)?;
        }
        
        let json = serde_json::to_string_pretty(config)?;
        fs::write(&path, json)
            .with_context(|| format!("Error guardando config del canal: {}", config.id))?;
        
        info!("Configuración del canal guardada: {}", config.id);
        Ok(())
    }
    
    pub fn delete_channel_config(&self, channel_id: &str) -> Result<()> {
        let path = self.config_dir.join("channels").join(format!("{}.json", channel_id));
        
        if path.exists() {
            self.backup_config(&path)?;
            fs::remove_file(&path)?;
            info!("Configuración del canal eliminada: {}", channel_id);
        }
        
        Ok(())
    }
    
    fn backup_config(&self, path: &Path) -> Result<()> {
        if !self.backup_dir.exists() {
            fs::create_dir_all(&self.backup_dir)?;
        }
        
        let filename = path.file_name().unwrap();
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_name = format!("{}_{}", timestamp, filename.to_string_lossy());
        let backup_path = self.backup_dir.join(backup_name);
        
        fs::copy(path, backup_path)?;
        Ok(())
    }
}