use crate::api::client::ApiClient;
use crate::config::loader::ClientConfig;
use crate::models::error::TranscoderError;
use tracing::{info, warn};
use tokio::time::{sleep, Duration};

/// Servicio de registro y re-registro automático
pub struct RegistrationService {
    api_client: ApiClient,
    client_config: ClientConfig,
}

impl RegistrationService {
    pub fn new(api_client: ApiClient, client_config: ClientConfig) -> Self {
        Self {
            api_client,
            client_config,
        }
    }
    
    /// Intentar registro con reintentos
    pub async fn register_with_retry(&self) -> Result<(), TranscoderError> {
        let max_attempts = self.client_config.backend.reconnect_attempts;
        let delay = Duration::from_secs(self.client_config.backend.reconnect_delay_seconds);
        
        for attempt in 1..=max_attempts {
            info!("Intento de registro #{}", attempt);
            
            match self.api_client.register_client(&self.client_config).await {
                Ok(_) => {
                    info!("Registro exitoso");
                    return Ok(());
                }
                Err(e) => {
                    warn!("Error en registro (intento {}): {}", attempt, e);
                    
                    if attempt < max_attempts {
                        info!("Reintentando en {} segundos...", delay.as_secs());
                        sleep(delay).await;
                    }
                }
            }
        }
        
        Err(TranscoderError::BackendError(
            format!("No se pudo registrar después de {} intentos", max_attempts)
        ))
    }
}