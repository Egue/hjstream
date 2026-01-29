use crate::core::manager::TranscoderManager;
use crate::models::stats::ChannelStats;
use axum::{
    extract::{Path, State, ws::WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, error};

// ============================================================================
// RESPUESTAS ESTANDARIZADAS
// ============================================================================

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }
    
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now(),
        }
    }
}

// ============================================================================
// HEALTH CHECK
// ============================================================================

pub async fn health_check() -> impl IntoResponse {
    #[derive(Serialize)]
    struct HealthResponse {
        status: String,
        version: String,
        uptime: String,
    }
    
    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: "TODO".to_string(), // Implementar cálculo de uptime
    };
    
    Json(ApiResponse::success(response))
}

// ============================================================================
// CANALES - LISTADO Y CONSULTA
// ============================================================================

pub async fn list_channels(
    State(manager): State<Arc<TranscoderManager>>,
) -> impl IntoResponse {
    let channels = manager.list_channels().await;
    Json(ApiResponse::success(channels))
}

pub async fn get_channel(
    State(manager): State<Arc<TranscoderManager>>,
    Path(channel_id): Path<String>,
) -> impl IntoResponse {
    match manager.get_channel_stats(&channel_id).await {
        Some(stats) => Json(ApiResponse::success(stats)),
        None => {
            let response: ApiResponse<ChannelStats> = ApiResponse::error(
                format!("Canal {} no encontrado", channel_id)
            );
            Json(response)
        }
    }
}

// ============================================================================
// ESTADÍSTICAS
// ============================================================================

pub async fn get_all_stats(
    State(manager): State<Arc<TranscoderManager>>,
) -> impl IntoResponse {
    let stats = manager.get_all_stats().await;
    Json(ApiResponse::success(stats))
}

pub async fn get_channel_stats(
    State(manager): State<Arc<TranscoderManager>>,
    Path(channel_id): Path<String>,
) -> impl IntoResponse {
    match manager.get_channel_stats(&channel_id).await {
        Some(stats) => Json(ApiResponse::success(stats)),
        None => {
            let response: ApiResponse<ChannelStats> = ApiResponse::error(
                format!("Canal {} no encontrado", channel_id)
            );
            Json(response)
        }
    }
}

// ============================================================================
// CONTROL DE CANALES
// ============================================================================

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    pub config: crate::config::loader::ChannelConfig,
}

pub async fn create_channel(
    State(manager): State<Arc<TranscoderManager>>,
    Json(request): Json<CreateChannelRequest>,
) -> impl IntoResponse {
    info!("Creando canal: {}", request.config.id);
    
    match manager.add_channel(request.config).await {
        Ok(_) => {
            (
                StatusCode::CREATED,
                Json(ApiResponse::success("Canal creado exitosamente".to_string()))
            )
        }
        Err(e) => {
            error!("Error creando canal: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(e.to_string()))
            )
        }
    }
}

pub async fn delete_channel(
    State(manager): State<Arc<TranscoderManager>>,
    Path(channel_id): Path<String>,
) -> impl IntoResponse {
    info!("Eliminando canal: {}", channel_id);
    
    match manager.remove_channel(&channel_id).await {
        Ok(_) => Json(ApiResponse::success("Canal eliminado exitosamente".to_string())),
        Err(e) => {
            error!("Error eliminando canal: {}", e);
            Json(ApiResponse::error(e.to_string()))
        }
    }
}

pub async fn restart_channel(
    State(manager): State<Arc<TranscoderManager>>,
    Path(channel_id): Path<String>,
) -> impl IntoResponse {
    info!("Reiniciando canal: {}", channel_id);
    
    match manager.restart_channel(&channel_id).await {
        Ok(_) => Json(ApiResponse::success("Canal reiniciado exitosamente".to_string())),
        Err(e) => {
            error!("Error reiniciando canal: {}", e);
            Json(ApiResponse::error(e.to_string()))
        }
    }
}

// ============================================================================
// MONITOREO AGREGADO
// ============================================================================

pub async fn get_summary(
    State(manager): State<Arc<TranscoderManager>>,
) -> impl IntoResponse {
    let all_stats = manager.get_all_stats().await;
    
    #[derive(Serialize)]
    struct Summary {
        total_channels: usize,
        running: usize,
        stopped: usize,
        error: usize,
        total_fps: f64,
        total_bitrate_kbps: f64,
        channels: Vec<ChannelSummary>,
    }
    
    #[derive(Serialize)]
    struct ChannelSummary {
        id: String,
        status: String,
        fps: f64,
        bitrate_kbps: f64,
        uptime_seconds: u64,
        healthy: bool,
    }
    
    let mut summary = Summary {
        total_channels: all_stats.len(),
        running: 0,
        stopped: 0,
        error: 0,
        total_fps: 0.0,
        total_bitrate_kbps: 0.0,
        channels: Vec::new(),
    };
    
    for (id, stats) in all_stats {
        use crate::models::stats::ChannelStatus;
        
        match &stats.status {
            ChannelStatus::Running => summary.running += 1,
            ChannelStatus::Stopped => summary.stopped += 1,
            ChannelStatus::Error(_) => summary.error += 1,
            _ => {}
        }
        
        summary.total_fps += stats.current_fps;
        summary.total_bitrate_kbps += stats.output_bitrate_kbps;
        
        summary.channels.push(ChannelSummary {
            id: id.clone(),
            status: stats.status.as_str().to_string(),
            fps: stats.current_fps,
            bitrate_kbps: stats.output_bitrate_kbps,
            uptime_seconds: stats.uptime_seconds,
            healthy: stats.is_healthy(),
        });
    }
    
    Json(ApiResponse::success(summary))
}

// ============================================================================
// WEBSOCKET HANDLER
// ============================================================================

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(manager): State<Arc<TranscoderManager>>,
) -> Response {
    ws.on_upgrade(move |socket| crate::api::websocket::handle_socket(socket, manager))
}

// ============================================================================
// CONFIGURACIÓN
// ============================================================================

#[derive(Deserialize)]
pub struct UpdateConfigRequest {
    pub config: crate::config::loader::ChannelConfig,
}

pub async fn update_channel_config(
    State(manager): State<Arc<TranscoderManager>>,
    Path(channel_id): Path<String>,
    Json(request): Json<UpdateConfigRequest>,
) -> impl IntoResponse {
    info!("Actualizando configuración del canal: {}", channel_id);
    
    match manager.reload_channel_config(&channel_id, request.config).await {
        Ok(_) => Json(ApiResponse::success("Configuración actualizada".to_string())),
        Err(e) => {
            error!("Error actualizando configuración: {}", e);
            Json(ApiResponse::error(e.to_string()))
        }
    }
}

// ============================================================================
// HEALTH CHECK DE CANALES
// ============================================================================

pub async fn check_channels_health(
    State(manager): State<Arc<TranscoderManager>>,
) -> impl IntoResponse {
    let health_results = manager.check_all_health().await;
    
    #[derive(Serialize)]
    struct HealthReport {
        total: usize,
        healthy: usize,
        unhealthy: usize,
        channels: HashMap<String, bool>,
    }
    
    let mut report = HealthReport {
        total: health_results.len(),
        healthy: 0,
        unhealthy: 0,
        channels: HashMap::new(),
    };
    
    for (id, healthy) in health_results {
        if healthy {
            report.healthy += 1;
        } else {
            report.unhealthy += 1;
        }
        report.channels.insert(id, healthy);
    }
    
    Json(ApiResponse::success(report))
}