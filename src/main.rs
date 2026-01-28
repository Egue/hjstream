// Cargo.toml dependencies needed:
// [dependencies]
// tokio = { version = "1", features = ["full"] }
// axum = "0.7"
// tower-http = { version = "0.5", features = ["cors"] }
// tracing = "0.1"
// tracing-subscriber = "0.3"
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// chrono = "0.4"

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
use tokio::io::{AsyncBufReadExt, BufReader};
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
        local_addr: Option<String>,
    },
    #[serde(rename = "srt_transcoder")]
    SrtTranscoder {
        srt_input: String,
        udp_output: String,
        video_bitrate: String,
        audio_bitrate: String,
        preset: String,
        local_addr: Option<String>,
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
    pub current_fps: f64,
    pub uptime_seconds: u64,
    pub last_error: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub mode: String,
    pub dropped_frames: u64,
    pub speed: f64,
}

// ============================================================================
// CANAL - MANEJO DE PROCESOS FFMPEG
// ============================================================================

pub struct Channel {
    config: ChannelConfig,
    process: Option<Child>,
    stats: Arc<RwLock<ChannelStats>>,
}

impl Channel {
    pub fn new(config: ChannelConfig) -> Self {
        let mode_str = match &config.mode {
            ChannelMode::Transcoder { .. } => "transcoder".to_string(),
            ChannelMode::Receiver { .. } => "receiver".to_string(),
            ChannelMode::SrtTranscoder { .. } => "srt_transcoder".to_string(),
        };

        Self {
            config,
            process: None,
            stats: Arc::new(RwLock::new(ChannelStats {
                status: ChannelStatus::Stopped,
                frames_processed: 0,
                current_bitrate: 0.0,
                current_fps: 0.0,
                uptime_seconds: 0,
                last_error: None,
                started_at: chrono::Utc::now(),
                mode: mode_str,
                dropped_frames: 0,
                speed: 0.0,
            })),
        }
    }

    fn build_ffmpeg_command(&self) -> Command {
        let mut cmd = Command::new("ffmpeg");

        cmd.args(&["-stats", "-loglevel", "info"]);
        
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
                local_addr,
            } => {
                // Construir URL UDP con localaddr si está presente
                let udp_url = if let Some(addr) = local_addr {
                    if udp_output.contains('?') {
                        format!("{}&localaddr={}", udp_output, addr)
                    } else {
                        format!("{}?localaddr={}", udp_output, addr)
                    }
                } else {
                    udp_output.clone()
                };

                // Modo Receiver: SRT → UDP (con recodificación de audio si es necesario)
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
                    &udp_url,                 // Output UDP
                ]);
            }
            ChannelMode::SrtTranscoder {
                srt_input,
                udp_output,
                video_bitrate,
                audio_bitrate,
                preset,
                local_addr,
            } => {
                // Construir URL UDP con localaddr si está presente
                let udp_url = if let Some(addr) = local_addr {
                    if udp_output.contains('?') {
                        format!("{}&localaddr={}", udp_output, addr)
                    } else {
                        format!("{}?localaddr={}", udp_output, addr)
                    }
                } else {
                    udp_output.clone()
                };

                // Modo SRT Transcoder: SRT → H.264/MPEG-4 → UDP
                cmd.args(&[
                    "-fflags", "+genpts",     // Generate presentation timestamps
                    "-i", srt_input,          // Input SRT
                    "-c:v", "libx264",        // Video codec H.264 (MPEG-4 Part 10)
                    "-preset", preset,        // Encoding preset
                    "-tune", "zerolatency",   // Low latency
                    "-b:v", video_bitrate,    // Video bitrate
                    "-maxrate", video_bitrate,
                    "-bufsize", "2M",
                    "-g", "50",               // GOP size
                    "-keyint_min", "50",
                    "-c:a", "aac",            // Audio codec
                    "-b:a", audio_bitrate,    // Audio bitrate
                    "-ar", "48000",           // Sample rate
                    "-ac", "2",               // Stereo
                    "-f", "mpegts",           // Format
                    "-mpegts_copyts", "1",
                    "-y",
                    &udp_url,                 // Output UDP
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

        let mode = {
            let stats = self.stats.read().await;
            stats.mode.clone()
        };

        info!("Starting channel: {} ({})", self.config.name, mode);
        
        {
            let mut stats = self.stats.write().await;
            stats.status = ChannelStatus::Starting;
            stats.started_at = chrono::Utc::now();
        }

        let mut cmd = self.build_ffmpeg_command();
        
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;

        // Capturar stderr para parsear estadísticas de FFmpeg
        if let Some(stderr) = child.stderr.take() {
            let stats_clone = self.stats.clone();
            let channel_name = self.config.name.clone();
            
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                
                while let Ok(Some(line)) = lines.next_line().await {
                    // Parsear línea de progreso de FFmpeg
                    // Formato: frame= 150 fps= 30 q=28.0 size= 512kB time=00:00:05.00 bitrate= 838.4kbits/s speed=1.0x
                    if line.contains("frame=") && line.contains("fps=") {
                        if let Some((frames, fps, bitrate, speed)) = parse_ffmpeg_stats(&line) {
                            let mut stats = stats_clone.write().await;
                            stats.frames_processed = frames;
                            stats.current_fps = fps;
                            stats.current_bitrate = bitrate;
                            stats.speed = speed;
                        }
                    } else if line.contains("error") || line.contains("Error") {
                        warn!("FFmpeg [{}]: {}", channel_name, line);
                    }
                }
            });
        }

        self.process = Some(child);
        
        {
            let mut stats = self.stats.write().await;
            stats.status = ChannelStatus::Running;
        }
        
        info!("Channel {} started successfully", self.config.name);
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), String> {
        if let Some(mut process) = self.process.take() {
            info!("Stopping channel: {}", self.config.name);
            
            process.kill().await
                .map_err(|e| format!("Failed to kill process: {}", e))?;
            
            let mut stats = self.stats.write().await;
            stats.status = ChannelStatus::Stopped;
            
            info!("Channel {} stopped", self.config.name);
            Ok(())
        } else {
            Err("Channel not running".to_string())
        }
    }

    pub async fn get_stats(&self) -> ChannelStats {
        let mut stats = self.stats.read().await.clone();
        let started_at = stats.started_at;
        stats.uptime_seconds = (chrono::Utc::now() - started_at).num_seconds() as u64;
        stats
    }

    pub async fn check_health(&mut self) -> bool {
        if let Some(process) = &mut self.process {
            match process.try_wait() {
                Ok(Some(status)) => {
                    warn!("Channel {} exited with status: {:?}", self.config.name, status);
                    
                    let mut stats = self.stats.write().await;
                    stats.status = ChannelStatus::Error(format!("Process exited: {:?}", status));
                    stats.last_error = Some(format!("Exit status: {:?}", status));
                    
                    self.process = None;
                    false
                }
                Ok(None) => true,
                Err(e) => {
                    error!("Error checking channel {}: {}", self.config.name, e);
                    
                    let mut stats = self.stats.write().await;
                    stats.status = ChannelStatus::Error(e.to_string());
                    stats.last_error = Some(e.to_string());
                    
                    false
                }
            }
        } else {
            false
        }
    }
}

// ============================================================================
// PARSER DE ESTADÍSTICAS DE FFMPEG
// ============================================================================

fn parse_ffmpeg_stats(line: &str) -> Option<(u64, f64, f64, f64)> {
    // Parsear línea como: frame= 150 fps= 30 q=28.0 size= 512kB time=00:00:05.00 bitrate= 838.4kbits/s speed=1.0x
    let mut frames = 0u64;
    let mut fps = 0.0f64;
    let mut bitrate = 0.0f64;
    let mut speed = 0.0f64;

    for part in line.split_whitespace() {
        if part.starts_with("frame=") {
            if let Ok(val) = part.trim_start_matches("frame=").parse::<u64>() {
                frames = val;
            }
        } else if part.starts_with("fps=") {
            if let Ok(val) = part.trim_start_matches("fps=").parse::<f64>() {
                fps = val;
            }
        } else if part.starts_with("bitrate=") {
            // Formato: bitrate= 838.4kbits/s
            let bitrate_str = part.trim_start_matches("bitrate=")
                .trim_end_matches("kbits/s")
                .trim_end_matches("Mbits/s");
            if let Ok(val) = bitrate_str.parse::<f64>() {
                bitrate = if part.contains("Mbits/s") {
                    val * 1000.0
                } else {
                    val
                };
            }
        } else if part.starts_with("speed=") {
            let speed_str = part.trim_start_matches("speed=").trim_end_matches('x');
            if let Ok(val) = speed_str.parse::<f64>() {
                speed = val;
            }
        }
    }

    if frames > 0 || fps > 0.0 {
        Some((frames, fps, bitrate, speed))
    } else {
        None
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
        if let Some(channel) = channels.get(channel_id) {
            Some(channel.get_stats().await)
        } else {
            None
        }
    }

    pub async fn get_all_stats(&self) -> HashMap<String, ChannelStats> {
        let channels = self.channels.read().await;
        let mut result = HashMap::new();
        
        for (id, channel) in channels.iter() {
            result.insert(id.clone(), channel.get_stats().await);
        }
        
        result
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

// GET /channels/:id/monitor - Monitoreo en tiempo real (última actualización)
async fn monitor_channel(
    State(manager): State<TranscoderManager>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match manager.get_channel_stats(&id).await {
        Some(stats) => {
            let health_status = if matches!(stats.status, ChannelStatus::Running) {
                if stats.current_fps > 0.0 {
                    "healthy"
                } else {
                    "starting"
                }
            } else {
                "unhealthy"
            };

            let response = serde_json::json!({
                "channel_id": id,
                "health": health_status,
                "stats": stats,
                "timestamp": chrono::Utc::now(),
            });
            
            Json(ApiResponse::success(response))
        }
        None => Json(ApiResponse::<serde_json::Value>::error(format!("Channel {} not found", id))),
    }
}

// GET /monitor/summary - Resumen de todos los canales
async fn monitor_summary(
    State(manager): State<TranscoderManager>,
) -> impl IntoResponse {
    let all_stats = manager.get_all_stats().await;
    
    let mut summary = serde_json::json!({
        "total_channels": all_stats.len(),
        "running": 0,
        "stopped": 0,
        "error": 0,
        "total_fps": 0.0,
        "total_bitrate": 0.0,
        "channels": []
    });

    let mut channels_array = Vec::new();

    for (id, stats) in all_stats {
        match stats.status {
            ChannelStatus::Running => summary["running"] = (summary["running"].as_u64().unwrap() + 1).into(),
            ChannelStatus::Stopped => summary["stopped"] = (summary["stopped"].as_u64().unwrap() + 1).into(),
            ChannelStatus::Error(_) => summary["error"] = (summary["error"].as_u64().unwrap() + 1).into(),
            _ => {}
        }

        summary["total_fps"] = (summary["total_fps"].as_f64().unwrap() + stats.current_fps).into();
        summary["total_bitrate"] = (summary["total_bitrate"].as_f64().unwrap() + stats.current_bitrate).into();

        channels_array.push(serde_json::json!({
            "id": id,
            "status": stats.status,
            "fps": stats.current_fps,
            "bitrate_mbps": stats.current_bitrate / 1000.0,
            "uptime": stats.uptime_seconds,
        }));
    }

    summary["channels"] = channels_array.into();
    summary["timestamp"] = chrono::Utc::now().to_rfc3339().into();

    Json(ApiResponse::success(summary))
}

// ============================================================================
// MAIN - INICIALIZACIÓN DEL SERVIDOR
// ============================================================================

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
        .route("/channels/:id/monitor", get(monitor_channel))
        .route("/monitor/summary", get(monitor_summary))
        .layer(CorsLayer::permissive())
        .with_state(manager);

    // Iniciar servidor HTTP
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("API REST escuchando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}