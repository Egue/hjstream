use crate::config::loader::{ClientConfig, ChannelConfig};
use crate::core::channel::Channel;
use crate::models::stats::ChannelStats;
use crate::models::error::TranscoderError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Manager principal que coordina todos los canales de transcodificación
pub struct TranscoderManager {
    client_config: ClientConfig,
    channels: Arc<RwLock<HashMap<String, Channel>>>,
}

impl TranscoderManager {
    pub fn new(client_config: ClientConfig) -> Self {
        Self {
            client_config,
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Agregar y arrancar un nuevo canal
    pub async fn add_channel(&self, config: ChannelConfig) -> Result<(), TranscoderError> {
        let mut channels = self.channels.write().await;
        
        // Validar límite de canales
        if channels.len() >= self.client_config.performance.max_channels as usize {
            return Err(TranscoderError::MaxChannelsReached(
                self.client_config.performance.max_channels
            ));
        }
        
        // Verificar si ya existe
        if channels.contains_key(&config.id) {
            return Err(TranscoderError::ChannelAlreadyExists(config.id.clone()));
        }
        
        info!("Agregando canal: {} ({})", config.id, config.name);
        
        // Crear canal
        let mut channel = Channel::new(config.clone())?;
        
        // Iniciar canal
        channel.start().await?;
        
        // Insertar en el mapa
        channels.insert(config.id.clone(), channel);
        
        info!("Canal {} agregado y iniciado", config.id);
        Ok(())
    }
    
    /// Remover y detener un canal
    pub async fn remove_channel(&self, channel_id: &str) -> Result<(), TranscoderError> {
        let mut channels = self.channels.write().await;
        
        if let Some(mut channel) = channels.remove(channel_id) {
            info!("Removiendo canal: {}", channel_id);
            channel.stop().await?;
            info!("Canal {} removido", channel_id);
            Ok(())
        } else {
            Err(TranscoderError::ChannelNotFound(channel_id.to_string()))
        }
    }
    
    /// Reiniciar un canal específico
    pub async fn restart_channel(&self, channel_id: &str) -> Result<(), TranscoderError> {
        let mut channels = self.channels.write().await;
        
        if let Some(channel) = channels.get_mut(channel_id) {
            info!("Reiniciando canal: {}", channel_id);
            channel.stop().await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            channel.start().await?;
            info!("Canal {} reiniciado", channel_id);
            Ok(())
        } else {
            Err(TranscoderError::ChannelNotFound(channel_id.to_string()))
        }
    }
    
    /// Obtener estadísticas de un canal
    pub async fn get_channel_stats(&self, channel_id: &str) -> Option<ChannelStats> {
        let channels = self.channels.read().await;
        if let Some(channel) = channels.get(channel_id) {
            Some(channel.get_stats().await)
        } else {
            None
        }
    }
    
    /// Obtener estadísticas de todos los canales
    pub async fn get_all_stats(&self) -> HashMap<String, ChannelStats> {
        let channels = self.channels.read().await;
        let mut stats = HashMap::new();
        
        for (id, channel) in channels.iter() {
            stats.insert(id.clone(), channel.get_stats().await);
        }
        
        stats
    }
    
    /// Obtener lista de IDs de canales activos
    pub async fn list_channels(&self) -> Vec<String> {
        let channels = self.channels.read().await;
        channels.keys().cloned().collect()
    }
    
    /// Verificar salud de todos los canales
    pub async fn check_all_health(&self) -> Vec<(String, bool)> {
        let mut channels = self.channels.write().await;
        let mut results = Vec::new();
        
        for (id, channel) in channels.iter_mut() {
            let healthy = channel.check_health().await;
            results.push((id.clone(), healthy));
            
            if !healthy {
                warn!("Canal {} no está saludable", id);
            }
        }
        
        results
    }
    
    /// Detener todos los canales
    pub async fn stop_all_channels(&self) -> Result<(), TranscoderError> {
        let mut channels = self.channels.write().await;
        
        info!("Deteniendo todos los canales ({})...", channels.len());
        
        for (id, channel) in channels.iter_mut() {
            info!("Deteniendo canal: {}", id);
            if let Err(e) = channel.stop().await {
                error!("Error deteniendo canal {}: {}", id, e);
            }
        }
        
        channels.clear();
        info!("Todos los canales detenidos");
        
        Ok(())
    }
    
    /// Recargar configuración de un canal sin reiniciarlo
    pub async fn reload_channel_config(
        &self,
        channel_id: &str,
        new_config: ChannelConfig,
    ) -> Result<(), TranscoderError> {
        let mut channels = self.channels.write().await;
        
        if let Some(channel) = channels.get_mut(channel_id) {
            info!("Recargando configuración del canal: {}", channel_id);
            channel.update_config(new_config).await?;
            Ok(())
        } else {
            Err(TranscoderError::ChannelNotFound(channel_id.to_string()))
        }
    }
    
    /// Obtener número de canales activos
    pub async fn channel_count(&self) -> usize {
        let channels = self.channels.read().await;
        channels.len()
    }
    
    /// Verificar si un canal existe
    pub async fn has_channel(&self, channel_id: &str) -> bool {
        let channels = self.channels.read().await;
        channels.contains_key(channel_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_manager_creation() {
        // Test básico de creación
        let config = ClientConfig::default();
        let manager = TranscoderManager::new(config);
        assert_eq!(manager.channel_count().await, 0);
    }
}