use thiserror::Error;

/// Errores del sistema de transcodificación
#[derive(Error, Debug)]
pub enum TranscoderError {
    #[error("Canal no encontrado: {0}")]
    ChannelNotFound(String),
    
    #[error("Canal ya existe: {0}")]
    ChannelAlreadyExists(String),
    
    #[error("Canal ya está corriendo: {0}")]
    ChannelAlreadyRunning(String),
    
    #[error("Número máximo de canales alcanzado: {0}")]
    MaxChannelsReached(u32),
    
    #[error("Error de configuración: {0}")]
    ConfigError(String),
    
    #[error("Error cargando configuración: {0}")]
    ConfigLoadError(String),
    
    #[error("Error guardando configuración: {0}")]
    ConfigSaveError(String),
    
    #[error("Configuración inválida: {0}")]
    InvalidConfig(String),
    
    #[error("Error de análisis de stream: {0}")]
    AnalysisFailed(String),
    
    #[error("Error spawneando proceso: {0}")]
    ProcessSpawnFailed(String),
    
    #[error("Error matando proceso: {0}")]
    ProcessKillFailed(String),
    
    #[error("Proceso terminó inesperadamente: {0}")]
    ProcessDied(String),
    
    #[error("Error de red: {0}")]
    NetworkError(String),
    
    #[error("Error SRT: {0}")]
    SrtError(String),
    
    #[error("Error UDP: {0}")]
    UdpError(String),
    
    #[error("Error de codec: {0}")]
    CodecError(String),
    
    #[error("Error de muxing: {0}")]
    MuxError(String),
    
    #[error("Error de demuxing: {0}")]
    DemuxError(String),
    
    #[error("Error de pipeline: {0}")]
    PipelineError(String),
    
    #[error("Error de API: {0}")]
    ApiError(String),
    
    #[error("Error de comunicación con backend: {0}")]
    BackendError(String),
    
    #[error("Error de autenticación: {0}")]
    AuthError(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("Recurso no disponible: {0}")]
    ResourceUnavailable(String),
    
    #[error("Error de hardware: {0}")]
    HardwareError(String),
    
    #[error("Aceleración hardware no disponible: {0}")]
    HardwareAccelNotAvailable(String),
    
    #[error("Error de I/O: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Error de serialización JSON: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("Error desconocido: {0}")]
    Unknown(String),
}

impl TranscoderError {
    /// Verifica si el error es recuperable
    pub fn is_recoverable(&self) -> bool {
        match self {
            // Errores recuperables (reintentar)
            TranscoderError::NetworkError(_) |
            TranscoderError::SrtError(_) |
            TranscoderError::UdpError(_) |
            TranscoderError::Timeout(_) |
            TranscoderError::BackendError(_) |
            TranscoderError::ProcessDied(_) => true,
            
            // Errores no recuperables
            TranscoderError::InvalidConfig(_) |
            TranscoderError::MaxChannelsReached(_) |
            TranscoderError::ChannelAlreadyExists(_) |
            TranscoderError::HardwareAccelNotAvailable(_) => false,
            
            // Por defecto, intentar recuperar
            _ => true,
        }
    }
    
    /// Obtiene la severidad del error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            TranscoderError::ConfigError(_) |
            TranscoderError::InvalidConfig(_) |
            TranscoderError::MaxChannelsReached(_) => ErrorSeverity::Critical,
            
            TranscoderError::ProcessDied(_) |
            TranscoderError::HardwareError(_) |
            TranscoderError::CodecError(_) => ErrorSeverity::High,
            
            TranscoderError::NetworkError(_) |
            TranscoderError::BackendError(_) |
            TranscoderError::Timeout(_) => ErrorSeverity::Medium,
            
            TranscoderError::ChannelNotFound(_) |
            TranscoderError::ApiError(_) => ErrorSeverity::Low,
            
            _ => ErrorSeverity::Medium,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl ErrorSeverity {
    pub fn as_str(&self) -> &str {
        match self {
            ErrorSeverity::Low => "low",
            ErrorSeverity::Medium => "medium",
            ErrorSeverity::High => "high",
            ErrorSeverity::Critical => "critical",
        }
    }
}

/// Resultado personalizado para el sistema
pub type Result<T> = std::result::Result<T, TranscoderError>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_recoverability() {
        let network_err = TranscoderError::NetworkError("Connection lost".to_string());
        assert!(network_err.is_recoverable());
        
        let config_err = TranscoderError::InvalidConfig("Bad format".to_string());
        assert!(!config_err.is_recoverable());
    }
    
    #[test]
    fn test_error_severity() {
        let critical = TranscoderError::MaxChannelsReached(10);
        assert_eq!(critical.severity(), ErrorSeverity::Critical);
        
        let low = TranscoderError::ChannelNotFound("ch1".to_string());
        assert_eq!(low.severity(), ErrorSeverity::Low);
    }
}