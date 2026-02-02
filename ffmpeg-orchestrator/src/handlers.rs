use crate::models::*;
use crate::process_manager::ProcessManager;
use crate::storage::Storage;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

pub struct AppState {
    pub storage: Arc<Storage>,
    pub process_manager: Arc<ProcessManager>,
}

// Health check
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "hjstream"
    }))
}

// Listar todos los canales
pub async fn list_channels(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ChannelListResponse>, AppError> {
    let channels = state.storage.get_all_channels().await;
    let total = channels.len();

    Ok(Json(ChannelListResponse { channels, total }))
}

// Obtener un canal específico
pub async fn get_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ChannelResponse>, AppError> {
    let channel = state
        .storage
        .get_channel(id)
        .await
        .ok_or(AppError::NotFound("Channel not found".to_string()))?;

    Ok(Json(ChannelResponse { channel }))
}

// Crear un nuevo canal
pub async fn create_channel(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<ChannelResponse>), AppError> {
    // Validar request
    request.validate()?;

    // Verificar que no exista un canal con el mismo nombre
    if state.storage.get_channel_by_name(&request.name).await.is_some() {
        return Err(AppError::BadRequest(format!(
            "Channel with name '{}' already exists",
            request.name
        )));
    }

    let channel = Channel::new(request);
    state.storage.add_channel(channel.clone()).await?;

    tracing::info!("Created channel: {} ({})", channel.name, channel.id);

    Ok((StatusCode::CREATED, Json(ChannelResponse { channel })))
}

// Iniciar un canal
pub async fn start_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ChannelResponse>, AppError> {
    state.process_manager.start_channel(id).await?;

    let channel = state
        .storage
        .get_channel(id)
        .await
        .ok_or(AppError::NotFound("Channel not found".to_string()))?;

    tracing::info!("Starting channel: {} ({})", channel.name, channel.id);

    Ok(Json(ChannelResponse { channel }))
}

// Detener un canal
pub async fn stop_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ChannelResponse>, AppError> {
    state.process_manager.stop_channel(id).await?;

    let channel = state
        .storage
        .get_channel(id)
        .await
        .ok_or(AppError::NotFound("Channel not found".to_string()))?;

    tracing::info!("Stopped channel: {} ({})", channel.name, channel.id);

    Ok(Json(ChannelResponse { channel }))
}

// Reiniciar un canal
pub async fn restart_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ChannelResponse>, AppError> {
    state.process_manager.restart_channel(id).await?;

    let channel = state
        .storage
        .get_channel(id)
        .await
        .ok_or(AppError::NotFound("Channel not found".to_string()))?;

    tracing::info!("Restarted channel: {} ({})", channel.name, channel.id);

    Ok(Json(ChannelResponse { channel }))
}

// Eliminar un canal
pub async fn delete_channel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    // Detener el canal si está corriendo
    let _ = state.process_manager.stop_channel(id).await;

    state.storage.delete_channel(id).await?;

    tracing::info!("Deleted channel: {}", id);

    Ok(StatusCode::NO_CONTENT)
}

// Manejo de errores
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::BadRequest(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}
