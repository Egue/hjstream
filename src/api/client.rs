use crate::config::loader::{ClientConfig, ChannelConfig};
use crate::models::stats::ChannelStats;
use crate::models::error::TranscoderError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};

/// Cliente HTTP para comunicación con el backend Quarkus
#[derive(Clone)]
pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ApiClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        
        Self {
            client,
            base_url,
            api_key,
        }
    }
    
    // ========================================================================
    // REGISTRO DE CLIENTE
    // ========================================================================
    
    /// Registrar el cliente en el backend
    pub async fn register_client(&self, config: &ClientConfig) -> Result<(), TranscoderError> {
        info!("Registrando cliente en el backend: {}", config.client_id);
        
        let request = ClientRegistrationRequest {
            client_id: config.client_id.clone(),
            name: config.name.clone(),
            location: config.location.clone(),
            max_channels: config.performance.max_channels,
            version: env!("CARGO_PKG_VERSION").to_string(),
        };
        
        let url = format!("{}/api/clients/register", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| TranscoderError::BackendError(format!("Error registrando cliente: {}", e)))?;
        
        if response.status().is_success() {
            info!("Cliente registrado exitosamente");
            Ok(())
        } else {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            Err(TranscoderError::BackendError(
                format!("Error registrando cliente: {} - {}", status, error_body)
            ))
        }
    }
    
    // ========================================================================
    // HEARTBEAT
    // ========================================================================
    
    /// Enviar heartbeat al backend
    pub async fn send_heartbeat(&self, client_id: &str) -> Result<(), TranscoderError> {
        let url = format!("{}/api/clients/{}/heartbeat", self.base_url, client_id);
        
        let response = self.client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| TranscoderError::BackendError(format!("Error enviando heartbeat: {}", e)))?;
        
        if !response.status().is_success() {
            warn!("Error en heartbeat: {}", response.status());
        }
        
        Ok(())
    }
    
    // ========================================================================
    // CONFIGURACIÓN DE CANALES
    // ========================================================================
    
    /// Obtener todas las configuraciones de canales desde el backend
    pub async fn get_channel_configs(&self) -> Result<Vec<ChannelConfig>, TranscoderError> {
        let url = format!("{}/api/channels/configs", self.base_url);
        
        let response = self.client
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| TranscoderError::BackendError(format!("Error obteniendo configs: {}", e)))?;
        
        if response.status().is_success() {
            let configs: Vec<ChannelConfig> = response.json().await
                .map_err(|e| TranscoderError::BackendError(format!("Error parseando configs: {}", e)))?;
            Ok(configs)
        } else {
            Err(TranscoderError::BackendError(
                format!("Error obteniendo configs: {}", response.status())
            ))
        }
    }
    
    /// Obtener configuración de un canal específico
    pub async fn get_channel_config(&self, channel_id: &str) -> Result<ChannelConfig, TranscoderError> {
        let url = format!("{}/api/channels/{}/config", self.base_url, channel_id);
        
        let response = self.client
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| TranscoderError::BackendError(format!("Error obteniendo config: {}", e)))?;
        
        if response.status().is_success() {
            let config: ChannelConfig = response.json().await
                .map_err(|e| TranscoderError::BackendError(format!("Error parseando config: {}", e)))?;
            Ok(config)
        } else {
            Err(TranscoderError::BackendError(
                format!("Error obteniendo config: {}", response.status())
            ))
        }
    }
    
    /// Subir configuración de un canal al backend
    pub async fn upload_channel_config(&self, config: &ChannelConfig) -> Result<(), TranscoderError> {
        let url = format!("{}/api/channels/{}/config", self.base_url, config.id);
        
        let response = self.client
            .put(&url)
            .header("X-API-Key", &self.api_key)
            .json(config)
            .send()
            .await
            .map_err(|e| TranscoderError::BackendError(format!("Error subiendo config: {}", e)))?;
        
        if response.status().is_success() {
            info!("Configuración del canal {} subida al backend", config.id);
            Ok(())
        } else {
            Err(TranscoderError::BackendError(
                format!("Error subiendo config: {}", response.status())
            ))
        }
    }
    
    // ========================================================================
    // ESTADÍSTICAS
    // ========================================================================
    
    /// Enviar estadísticas de un canal al backend
    pub async fn send_channel_stats(&self, stats: &ChannelStats) -> Result<(), TranscoderError> {
        let url = format!("{}/api/channels/{}/stats", self.base_url, stats.channel_id);
        
        let response = self.client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(stats)
            .send()
            .await
            .map_err(|e| TranscoderError::BackendError(format!("Error enviando stats: {}", e)))?;
        
        if !response.status().is_success() {
            warn!("Error enviando estadísticas: {}", response.status());
        }
        
        Ok(())
    }
    
    /// Enviar estadísticas de múltiples canales
    pub async fn send_bulk_stats(&self, stats: Vec<ChannelStats>) -> Result<(), TranscoderError> {
        let url = format!("{}/api/stats/bulk", self.base_url);
        
        let request = BulkStatsRequest { stats };
        
        let response = self.client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| TranscoderError::BackendError(format!("Error enviando bulk stats: {}", e)))?;
        
        if !response.status().is_success() {
            warn!("Error enviando estadísticas en bulk: {}", response.status());
        }
        
        Ok(())
    }
    
    // ========================================================================
    // COMANDOS REMOTOS
    // ========================================================================
    
    /// Verificar si hay comandos pendientes del backend
    pub async fn check_pending_commands(&self, client_id: &str) -> Result<Vec<RemoteCommand>, TranscoderError> {
        let url = format!("{}/api/clients/{}/commands", self.base_url, client_id);
        
        let response = self.client
            .get(&url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| TranscoderError::BackendError(format!("Error obteniendo comandos: {}", e)))?;
        
        if response.status().is_success() {
            let commands: Vec<RemoteCommand> = response.json().await
                .map_err(|e| TranscoderError::BackendError(format!("Error parseando comandos: {}", e)))?;
            Ok(commands)
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Confirmar ejecución de un comando
    pub async fn acknowledge_command(&self, command_id: &str, success: bool, message: Option<String>) -> Result<(), TranscoderError> {
        let url = format!("{}/api/commands/{}/acknowledge", self.base_url, command_id);
        
        let request = CommandAcknowledgment {
            success,
            message,
            timestamp: chrono::Utc::now(),
        };
        
        let response = self.client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&request)
            .send()
            .await
            .map_err(|e| TranscoderError::BackendError(format!("Error confirmando comando: {}", e)))?;
        
        if !response.status().is_success() {
            warn!("Error confirmando comando: {}", response.status());
        }
        
        Ok(())
    }
    
    // ========================================================================
    // ALERTAS
    // ========================================================================
    
    /// Enviar alerta al backend
    pub async fn send_alert(&self, alert: &crate::models::stats::MonitoringAlert) -> Result<(), TranscoderError> {
        let url = format!("{}/api/alerts", self.base_url);
        
        let response = self.client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(alert)
            .send()
            .await
            .map_err(|e| TranscoderError::BackendError(format!("Error enviando alerta: {}", e)))?;
        
        if response.status().is_success() {
            info!("Alerta enviada al backend");
            Ok(())
        } else {
            Err(TranscoderError::BackendError(
                format!("Error enviando alerta: {}", response.status())
            ))
        }
    }
}

// ============================================================================
// ESTRUCTURAS DE REQUEST/RESPONSE
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct ClientRegistrationRequest {
    client_id: String,
    name: String,
    location: String,
    max_channels: u32,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BulkStatsRequest {
    stats: Vec<ChannelStats>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoteCommand {
    pub id: String,
    pub command_type: RemoteCommandType,
    pub channel_id: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RemoteCommandType {
    StartChannel,
    StopChannel,
    RestartChannel,
    UpdateConfig,
    ReloadConfig,
    GetStatus,
    RestartClient,
}

#[derive(Debug, Serialize, Deserialize)]
struct CommandAcknowledgment {
    success: bool,
    message: Option<String>,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_api_client_creation() {
        let client = ApiClient::new(
            "http://localhost:8080".to_string(),
            "test-key".to_string(),
        );
        
        assert_eq!(client.base_url, "http://localhost:8080");
        assert_eq!(client.api_key, "test-key");
    }
}