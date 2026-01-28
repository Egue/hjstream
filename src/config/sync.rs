use super::loader::{ConfigLoader, ChannelConfig};
use crate::api::client::ApiClient;
use anyhow::Result;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};

pub struct ConfigSyncService {
    loader: ConfigLoader,
    api_client: ApiClient,
    sync_interval: Duration,
}

impl ConfigSyncService {
    pub fn new(
        loader: ConfigLoader,
        api_client: ApiClient,
        sync_interval_seconds: u64,
    ) -> Self {
        Self {
            loader,
            api_client,
            sync_interval: Duration::from_secs(sync_interval_seconds),
        }
    }
    
    pub async fn start(&self) {
        let mut interval = interval(self.sync_interval);
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.sync_configs().await {
                error!("Error sincronizando configuraciones: {}", e);
            }
        }
    }
    
    async fn sync_configs(&self) -> Result<()> {
        info!("Iniciando sincronización de configuraciones con backend");
        
        // 1. Obtener configuraciones del backend
        let backend_configs = self.api_client.get_channel_configs().await?;
        
        // 2. Cargar configuraciones locales
        let local_configs = self.loader.load_all_channels()?;
        
        // 3. Comparar y actualizar
        for backend_config in backend_configs {
            let local_config = local_configs.iter()
                .find(|c| c.id == backend_config.id);
            
            match local_config {
                Some(local) => {
                    // Comparar versiones o timestamps
                    if self.needs_update(local, &backend_config) {
                        info!("Actualizando config local del canal: {}", backend_config.id);
                        self.loader.save_channel_config(&backend_config)?;
                    }
                }
                None => {
                    // Nueva configuración del backend
                    info!("Nueva configuración del backend: {}", backend_config.id);
                    self.loader.save_channel_config(&backend_config)?;
                }
            }
        }
        
        // 4. Reportar configuraciones locales que no están en backend
        for local_config in local_configs {
            if !backend_configs.iter().any(|b| b.id == local_config.id) {
                warn!("Config local no está en backend: {}", local_config.id);
                // Opcionalmente, enviar al backend
                self.api_client.upload_channel_config(&local_config).await?;
            }
        }
        
        info!("Sincronización completada");
        Ok(())
    }
    
    fn needs_update(&self, local: &ChannelConfig, backend: &ChannelConfig) -> bool {
        // Implementar lógica de comparación
        // Por ejemplo, comparar hash de configuración o timestamp
        serde_json::to_string(local).unwrap() != serde_json::to_string(backend).unwrap()
    }
}