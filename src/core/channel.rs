use crate::config::loader::ChannelConfig;
use crate::core::transcoder::Transcoder;
use crate::core::analyzer::StreamAnalyzer;
use crate::core::strategy::decide_strategy;
use crate::models::stats::{ChannelStats, ChannelStatus};
use crate::models::error::TranscoderError;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{info, warn, error};

/// Representa un canal de transcodificación individual
pub struct Channel {
    config: ChannelConfig,
    transcoder: Option<Transcoder>,
    stats: Arc<RwLock<ChannelStats>>,
    task_handle: Option<JoinHandle<()>>,
}

impl Channel {
    pub fn new(config: ChannelConfig) -> Result<Self, TranscoderError> {
        info!("Creando canal: {} ({})", config.id, config.name);
        
        let stats = Arc::new(RwLock::new(ChannelStats {
            channel_id: config.id.clone(),
            status: ChannelStatus::Stopped,
            frames_processed: 0,
            current_bitrate_kbps: 0.0,
            current_fps: 0.0,
            uptime_seconds: 0,
            last_error: None,
            started_at: None,
            mode: config.mode.clone(),
            dropped_frames: 0,
            speed: 0.0,
            input_bitrate_kbps: 0.0,
            output_bitrate_kbps: 0.0,
        }));
        
        Ok(Self {
            config,
            transcoder: None,
            stats,
            task_handle: None,
        })
    }
    
    pub async fn start(&mut self) -> Result<(), TranscoderError> {
        if self.transcoder.is_some() {
            return Err(TranscoderError::ChannelAlreadyRunning(self.config.id.clone()));
        }
        
        info!("Iniciando canal: {}", self.config.id);
        
        {
            let mut stats = self.stats.write().await;
            stats.status = ChannelStatus::Starting;
            stats.started_at = Some(chrono::Utc::now());
        }
        
        // Analizar stream de entrada (si auto_detect está habilitado)
        let strategy = if self.config.analysis.auto_detect {
            info!("Analizando stream de entrada...");
            match StreamAnalyzer::analyze(&self.config.input.url).await {
                Ok(stream_info) => {
                    info!("Stream detectado: {:?}", stream_info);
                    decide_strategy(&stream_info, &self.config)
                }
                Err(e) => {
                    warn!("Error analizando stream, usando estrategia por defecto: {}", e);
                    crate::core::strategy::TranscodeStrategy::default_for_config(&self.config)
                }
            }
        } else {
            crate::core::strategy::TranscodeStrategy::default_for_config(&self.config)
        };
        
        info!("Estrategia seleccionada: {:?}", strategy);
        
        // Crear transcoder con la estrategia decidida
        let transcoder = Transcoder::new(self.config.clone(), strategy)?;
        
        // Iniciar transcoder
        let stats_clone = self.stats.clone();
        let mut transcoder_clone = transcoder.clone();
        
        let task_handle = tokio::spawn(async move {
            if let Err(e) = transcoder_clone.run(stats_clone).await {
                error!("Error en transcoder: {}", e);
            }
        });
        
        self.transcoder = Some(transcoder);
        self.task_handle = Some(task_handle);
        
        {
            let mut stats = self.stats.write().await;
            stats.status = ChannelStatus::Running;
        }
        
        info!("Canal {} iniciado exitosamente", self.config.id);
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<(), TranscoderError> {
        info!("Deteniendo canal: {}", self.config.id);
        
        if let Some(mut transcoder) = self.transcoder.take() {
            transcoder.stop().await?;
        }
        
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        
        {
            let mut stats = self.stats.write().await;
            stats.status = ChannelStatus::Stopped;
        }
        
        info!("Canal {} detenido", self.config.id);
        Ok(())
    }
    
    pub async fn get_stats(&self) -> ChannelStats {
        let mut stats = self.stats.read().await.clone();
        
        // Calcular uptime
        if let Some(started_at) = stats.started_at {
            stats.uptime_seconds = (chrono::Utc::now() - started_at).num_seconds() as u64;
        }
        
        stats
    }
    
    pub async fn check_health(&mut self) -> bool {
        // Verificar si la tarea está viva
        if let Some(handle) = &self.task_handle {
            if handle.is_finished() {
                warn!("Tarea del canal {} ha terminado inesperadamente", self.config.id);
                
                let mut stats = self.stats.write().await;
                stats.status = ChannelStatus::Error("Proceso terminado".to_string());
                
                return false;
            }
        }
        
        // Verificar estadísticas
        let stats = self.stats.read().await;
        match &stats.status {
            ChannelStatus::Running => {
                // Verificar que está procesando frames
                if stats.current_fps == 0.0 && stats.uptime_seconds > 30 {
                    warn!("Canal {} no está procesando frames", self.config.id);
                    return false;
                }
                true
            }
            ChannelStatus::Error(_) => false,
            _ => true,
        }
    }
    
    pub async fn update_config(&mut self, new_config: ChannelConfig) -> Result<(), TranscoderError> {
        info!("Actualizando configuración del canal: {}", self.config.id);
        
        // Detener canal actual
        self.stop().await?;
        
        // Actualizar configuración
        self.config = new_config;
        
        // Reiniciar con nueva configuración
        self.start().await?;
        
        Ok(())
    }
    
    pub fn get_config(&self) -> &ChannelConfig {
        &self.config
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        info!("Destruyendo canal: {}", self.config.id);
    }
}