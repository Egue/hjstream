
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{ get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};

// ============================================================================
// CONFIGURACIÓN Y TIPOS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode")]
pub enum ChannelMode {
    #[serde(rename = "transcoder")]
    Transcoder {
        hls_input: String,
        srt_output: String,
        video_bitrate: String,
        audio_bitrate: String,
        preset: String,
    },
    #[serde(rename = "receiver")]
    Receiver {
        srt_input: String,
        udp_output: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub mode: ChannelMode,
}

#[derive(Debug, Clone, Serialize)]
pub enum ChannelStatus {
    Starting,
    Running,
    Stopped,
    Error(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelStats {
    pub status: ChannelStatus,
    pub frames_processed: u64,
    pub current_bitrate: f64,
    pub uptime_seconds: u64,
    pub last_error: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub mode: String,
}

// ============================================================================
// CANAL - MANEJO DE PROCESOS FFMPEG
// ============================================================================

pub struct Channel {
    config: ChannelConfig,
    process: Option<Child>,
    stats: ChannelStats,
}

impl Channel {
    pub fn new(config: ChannelConfig) -> Self {
        let mode_str = match &config.mode {
            ChannelMode::Transcoder { .. } => "transcoder".to_string(),
            ChannelMode::Receiver { .. } => "receiver".to_string(),
        };

        Self {
            config,
            process: None,
            stats: ChannelStats {
                status: ChannelStatus::Stopped,
                frames_processed: 0,
                current_bitrate: 0.0,
                uptime_seconds: 0,
                last_error: None,
                started_at: chrono::Utc::now(),
                mode: mode_str,
            },
        }
    }

    fn build_ffmpeg_command(&self) -> Command {
        let mut cmd = Command::new("ffmpeg");
        
        match &self.config.mode {
            ChannelMode::Transcoder {
                hls_input,
                srt_output,
                video_bitrate,
                audio_bitrate,
                preset,
            } => {
                // Modo Transcoder: HLS → H.264 → SRT
                cmd.args(&[
                    "-re",                    // Real-time
                    "-i", hls_input,          // Input HLS
                    "-c:v", "libx264",        // Video codec
                    "-preset", preset,        // Encoding preset
                    "-tune", "zerolatency",   // Low latency
                    "-b:v", video_bitrate,    // Video bitrate
                    "-maxrate", video_bitrate,
                    "-bufsize", "2M",
                    "-g", "50",               // GOP size
                    "-keyint_min", "50",      // Keyframe interval
                    "-c:a", "aac",            // Audio codec
                    "-b:a", audio_bitrate,    // Audio bitrate
                    "-ar", "48000",           // Sample rate
                    "-f", "mpegts",           // Format
                    "-muxdelay", "0.1",       // Mux delay
                    srt_output,               // Output SRT (e.g., srt://ip:port)
                ]);
            }
            ChannelMode::Receiver {
                srt_input,
                udp_output,
            } => {
                // Modo Receiver: SRT → UDP (sin transcodificación)
                cmd.args(&[
                    "-fflags", "+genpts",     // Generate presentation timestamps
                    "-i", srt_input,          // Input SRT
                    "-c:v", "copy",           // Copy video (no re-encode)
                    "-c:a", "aac",            // Re-encode audio to AAC
                    "-b:a", "128k",           // Audio bitrate
                    "-ar", "48000",           // Sample rate 48kHz
                    "-ac", "2",               // Stereo
                    "-f", "mpegts",           // Format
                    "-mpegts_copyts", "1",    // Copy timestamps
                    "-y",                     // Overwrite
                    udp_output,               // Output UDP
                ]);
            }
        }

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        cmd
    }

    pub async fn start(&mut self) -> Result<(), String> {
        if self.process.is_some() {
            return Err("Channel already running".to_string());
        }

        info!("Starting channel: {} ({})", self.config.name, self.stats.mode);
        self.stats.status = ChannelStatus::Starting;
        self.stats.started_at = chrono::Utc::now();

        let mut cmd = self.build_ffmpeg_command();
        
        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;

        self.process = Some(child);
        self.stats.status = ChannelStatus::Running;
        
        info!("Channel {} started successfully", self.config.name);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(mut process) = self.process.take() {
            info!("Stopping channel: {}", self.config.name);
            
            process.kill().await
                .map_err(|e| format!("Failed to kill process: {}", e))?;
            
            self.stats.status = ChannelStatus::Stopped;
            info!("Channel {} stopped", self.config.name);
            Ok(())
        } else {
            Err("Channel not running".to_string())
        }
    }

    pub fn get_stats(&self) -> ChannelStats {
        let mut stats = self.stats.clone();
        stats.uptime_seconds = (chrono::Utc::now() - self.stats.started_at).num_seconds() as u64;
        stats
    }

    pub async fn check_health(&mut self) -> bool {
        if let Some(process) = &mut self.process {
            match process.try_wait() {
                Ok(Some(status)) => {
                    warn!("Channel {} exited with status: {:?}", self.config.name, status);
                    self.stats.status = ChannelStatus::Error(format!("Process exited: {:?}", status));
                    self.stats.last_error = Some(format!("Exit status: {:?}", status));
                    self.process = None;
                    false
                }
                Ok(None) => true,
                Err(e) => {
                    error!("Error checking channel {}: {}", self.config.name, e);
                    self.stats.status = ChannelStatus::Error(e.to_string());
                    self.stats.last_error = Some(e.to_string());
                    false
                }
            }
        } else {
            false
        }
    }
}

// ============================================================================
// MANAGER - GESTIÓN DE MÚLTIPLES CANALES
// ============================================================================

#[derive(Clone)]
pub struct TranscoderManager {
    channels: Arc<RwLock<HashMap<String, Channel>>>,
}

impl TranscoderManager {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_channel(&self, config: ChannelConfig) -> Result<(), String> {
        let mut channels = self.channels.write().await;
        
        if channels.contains_key(&config.id) {
            return Err(format!("Channel {} already exists", config.id));
        }

        let mut channel = Channel::new(config.clone());
        channel.start().await?;
        
        channels.insert(config.id.clone(), channel);
        info!("Channel {} added and started", config.id);
        Ok(())
    }

    pub async fn remove_channel(&self, channel_id: &str) -> Result<(), String> {
        let mut channels = self.channels.write().await;
        
        if let Some(mut channel) = channels.remove(channel_id) {
            channel.stop().await?;
            info!("Channel {} removed", channel_id);
            Ok(())
        } else {
            Err(format!("Channel {} not found", channel_id))
        }
    }

    pub async fn get_channel_stats(&self, channel_id: &str) -> Option<ChannelStats> {
        let channels = self.channels.read().await;
        channels.get(channel_id).map(|c| c.get_stats())
    }

    pub async fn get_all_stats(&self) -> HashMap<String, ChannelStats> {
        let channels = self.channels.read().await;
        channels.iter()
            .map(|(id, channel)| (id.clone(), channel.get_stats()))
            .collect()
    }

    pub async fn restart_channel(&self, channel_id: &str) -> Result<(), String> {
        let mut channels = self.channels.write().await;
        
        if let Some(channel) = channels.get_mut(channel_id) {
            channel.stop().await?;
            tokio::time::sleep(Duration::from_secs(2)).await;
            channel.start().await?;
            Ok(())
        } else {
            Err(format!("Channel {} not found", channel_id))
        }
    }

    pub async fn start_health_monitor(&self) {
        let manager = self.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                let mut channels = manager.channels.write().await;
                
                for (id, channel) in channels.iter_mut() {
                    if !channel.check_health().await {
                        warn!("Channel {} is unhealthy, attempting restart...", id);
                        
                        if let Err(e) = channel.start().await {
                            error!("Failed to restart channel {}: {}", id, e);
                        } else {
                            info!("Channel {} restarted successfully", id);
                        }
                    }
                }
            }
        });
    }
}

// ============================================================================
// API REST - ENDPOINTS
// ============================================================================

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

// GET /health - Health check
async fn health_check() -> impl IntoResponse {
    Json(ApiResponse::success("OK"))
}

// GET /channels - Listar todos los canales
async fn list_channels(
    State(manager): State<TranscoderManager>,
) -> impl IntoResponse {
    let stats = manager.get_all_stats().await;
    Json(ApiResponse::success(stats))
}

// GET /channels/:id - Obtener estadísticas de un canal
async fn get_channel(
    State(manager): State<TranscoderManager>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match manager.get_channel_stats(&id).await {
        Some(stats) => Json(ApiResponse::success(stats)),
        None => Json(ApiResponse::<ChannelStats>::error(format!("Channel {} not found", id))),
    }
}

// POST /channels - Crear nuevo canal
async fn create_channel(
    State(manager): State<TranscoderManager>,
    Json(config): Json<ChannelConfig>,
) -> impl IntoResponse {
    match manager.add_channel(config).await {
    Ok(_) => (StatusCode::CREATED, Json(ApiResponse::success("Channel created".to_string()))),
    Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::error(e))),
}
}

// DELETE /channels/:id - Eliminar canal
async fn delete_channel(
    State(manager): State<TranscoderManager>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match manager.remove_channel(&id).await {
    Ok(_) => Json(ApiResponse::success("Channel deleted".to_string())),
    Err(e) => Json(ApiResponse::error(e)),
}
}

// POST /channels/:id/restart - Reiniciar canal
async fn restart_channel(
    State(manager): State<TranscoderManager>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match manager.restart_channel(&id).await {
    Ok(_) => Json(ApiResponse::success("Channel restarted".to_string())),
    Err(e) => Json(ApiResponse::error(e)),
}
}

// ============================================================================
// MAIN - INICIALIZACIÓN DEL SERVIDOR
// ============================================================================
/*
#[tokio::main]
async fn main() {
    // Inicializar logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Iniciando sistema de transcodificación multi-canal");

    let manager = TranscoderManager::new();

    // Iniciar monitor de salud
    manager.start_health_monitor().await;

    // Configurar rutas de la API
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/channels", get(list_channels).post(create_channel))
        .route("/channels/:id", get(get_channel).delete(delete_channel))
        .route("/channels/:id/restart", post(restart_channel))
        .layer(CorsLayer::permissive())
        .with_state(manager);

    // Iniciar servidor HTTP
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("API REST escuchando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}*/