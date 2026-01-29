use serde::{Deserialize, Serialize};

/// Versión simplificada de configuración para transferencia
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    pub client_id: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub channels: Vec<ChannelConfigSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfigSnapshot {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub enabled: bool,
    pub config_hash: String,
}

impl ConfigSnapshot {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now(),
            channels: Vec::new(),
        }
    }
    
    pub fn add_channel(&mut self, snapshot: ChannelConfigSnapshot) {
        self.channels.push(snapshot);
    }
    
    /// Calcula hash de toda la configuración
    pub fn calculate_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        for channel in &self.channels {
            channel.config_hash.hash(&mut hasher);
        }
        
        format!("{:x}", hasher.finish())
    }
}

/// Validador de configuración
pub struct ConfigValidator;

impl ConfigValidator {
    /// Valida una configuración de canal
    pub fn validate_channel_config(config: &crate::config::loader::ChannelConfig) -> Result<(), String> {
        // Validar ID
        if config.id.is_empty() {
            return Err("El ID del canal no puede estar vacío".to_string());
        }
        
        // Validar nombre
        if config.name.is_empty() {
            return Err("El nombre del canal no puede estar vacío".to_string());
        }
        
        // Validar URLs
        if config.input.url.is_empty() {
            return Err("La URL de entrada no puede estar vacía".to_string());
        }
        
        if config.output.url.is_empty() {
            return Err("La URL de salida no puede estar vacía".to_string());
        }
        
        // Validar bitrates
        if config.video.bitrate_kbps == 0 {
            return Err("El bitrate de video debe ser mayor a 0".to_string());
        }
        
        if config.video.max_bitrate_kbps < config.video.bitrate_kbps {
            return Err("El bitrate máximo debe ser mayor o igual al bitrate normal".to_string());
        }
        
        if config.audio.bitrate_kbps == 0 {
            return Err("El bitrate de audio debe ser mayor a 0".to_string());
        }
        
        // Validar resolución
        if config.video.framerate == 0 {
            return Err("El framerate debe ser mayor a 0".to_string());
        }
        
        // Validar MPEG-TS PIDs
        if config.mpegts.video_pid == config.mpegts.audio_pid {
            return Err("Los PIDs de video y audio deben ser diferentes".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_snapshot() {
        let mut snapshot = ConfigSnapshot::new("client-01".to_string());
        
        snapshot.add_channel(ChannelConfigSnapshot {
            id: "ch1".to_string(),
            name: "Channel 1".to_string(),
            mode: "transcoder".to_string(),
            enabled: true,
            config_hash: "abc123".to_string(),
        });
        
        assert_eq!(snapshot.channels.len(), 1);
        assert!(!snapshot.calculate_hash().is_empty());
    }
}