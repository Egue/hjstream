use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Modelo de canal (para sincronización con backend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub client_id: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub config: ChannelConfigModel,
    pub metadata: Option<serde_json::Value>,
}

/// Configuración del canal (versión simplificada para API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfigModel {
    pub mode: String,
    pub input_url: String,
    pub output_url: String,
    pub video_bitrate_kbps: u32,
    pub audio_bitrate_kbps: u32,
    pub preset: String,
}

impl Channel {
    pub fn new(id: String, name: String, client_id: String, config: ChannelConfigModel) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            client_id,
            enabled: true,
            created_at: now,
            updated_at: now,
            config,
            metadata: None,
        }
    }
}

/// Request para crear un canal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub config: ChannelConfigModel,
    pub metadata: Option<serde_json::Value>,
}

/// Request para actualizar un canal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub config: Option<ChannelConfigModel>,
    pub metadata: Option<serde_json::Value>,
}

/// Response con información del canal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelResponse {
    pub channel: Channel,
    pub status: String,
    pub current_stats: Option<crate::models::stats::ChannelStats>,
}