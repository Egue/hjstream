use crate::config::loader::ChannelConfig;
use crate::core::analyzer::StreamInfo;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Estrategia de transcodificación a aplicar
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TranscodeStrategy {
    /// No transcodificar, solo remuxear (copy)
    PassThrough,
    
    /// Transcodificar solo video, copiar audio
    TranscodeVideo,
    
    /// Transcodificar solo audio, copiar video
    TranscodeAudio,
    
    /// Transcodificar ambos (video y audio)
    TranscodeBoth,
}

impl TranscodeStrategy {
    /// Crear estrategia por defecto basada en la configuración del canal
    pub fn default_for_config(config: &ChannelConfig) -> Self {
        if config.analysis.force_transcode {
            Self::TranscodeBoth
        } else {
            Self::PassThrough
        }
    }
}

/// Decide la estrategia óptima basándose en el análisis del stream
pub fn decide_strategy(stream_info: &StreamInfo, config: &ChannelConfig) -> TranscodeStrategy {
    // Si está forzado a transcodificar, no analizar
    if config.analysis.force_transcode {
        info!("Transcodificación forzada por configuración");
        return TranscodeStrategy::TranscodeBoth;
    }
    
    // Analizar compatibilidad de video
    let video_compatible = is_video_compatible(stream_info, config);
    
    // Analizar compatibilidad de audio
    let audio_compatible = is_audio_compatible(stream_info, config);
    
    // Decidir estrategia
    let strategy = match (video_compatible, audio_compatible) {
        (true, true) => {
            if config.analysis.passthrough_if_compatible {
                info!("Stream compatible, usando PassThrough");
                TranscodeStrategy::PassThrough
            } else {
                info!("Stream compatible pero passthrough deshabilitado");
                TranscodeStrategy::TranscodeBoth
            }
        }
        (false, true) => {
            info!("Video incompatible, transcodificando solo video");
            TranscodeStrategy::TranscodeVideo
        }
        (true, false) => {
            info!("Audio incompatible, transcodificando solo audio");
            TranscodeStrategy::TranscodeAudio
        }
        (false, false) => {
            info!("Video y audio incompatibles, transcodificando ambos");
            TranscodeStrategy::TranscodeBoth
        }
    };
    
    strategy
}

/// Verifica si el video del stream es compatible con la configuración CATV
fn is_video_compatible(stream_info: &StreamInfo, config: &ChannelConfig) -> bool {
    let video_cfg = &config.video;
    
    // 1. Verificar codec
    let codec_ok = stream_info.video_codec == video_cfg.codec 
        || stream_info.video_codec == format!("lib{}", video_cfg.codec);
    
    if !codec_ok {
        info!("Codec de video incompatible: {} != {}", 
            stream_info.video_codec, video_cfg.codec);
        return false;
    }
    
    // 2. Verificar perfil (si está especificado)
    if let Some(stream_profile) = &stream_info.video_profile {
        let profile_ok = stream_profile.to_lowercase().contains(&video_cfg.profile.to_lowercase());
        if !profile_ok {
            info!("Perfil de video incompatible: {:?} != {}", 
                stream_profile, video_cfg.profile);
            return false;
        }
    }
    
    // 3. Verificar bitrate
    let bitrate_ok = check_video_bitrate(stream_info, config);
    if !bitrate_ok {
        return false;
    }
    
    // 4. Verificar resolución
    if stream_info.width > 1920 || stream_info.height > 1080 {
        info!("Resolución demasiado alta: {}x{}", stream_info.width, stream_info.height);
        return false;
    }
    
    // 5. Verificar framerate
    if stream_info.fps > video_cfg.framerate as f64 + 1.0 {
        info!("Framerate demasiado alto: {} > {}", stream_info.fps, video_cfg.framerate);
        return false;
    }
    
    info!("Video compatible con configuración CATV");
    true
}

/// Verifica el bitrate del video
fn check_video_bitrate(stream_info: &StreamInfo, config: &ChannelConfig) -> bool {
    let video_cfg = &config.video;
    
    // Para CATV necesitamos CBR o bitrate muy cercano al objetivo
    if video_cfg.rate_control == "cbr" {
        // Tolerancia del 15% para CBR
        let tolerance = 0.15;
        let min_acceptable = (video_cfg.bitrate_kbps as f64 * (1.0 - tolerance)) as u32;
        let max_acceptable = (video_cfg.bitrate_kbps as f64 * (1.0 + tolerance)) as u32;
        
        if stream_info.video_bitrate_kbps < min_acceptable 
            || stream_info.video_bitrate_kbps > max_acceptable {
            info!("Bitrate fuera de rango CBR: {} kbps (objetivo: {} ±{}%)", 
                stream_info.video_bitrate_kbps, 
                video_cfg.bitrate_kbps,
                (tolerance * 100.0) as u32);
            return false;
        }
    } else {
        // Para VBR, solo verificar que no exceda el máximo
        if stream_info.video_bitrate_kbps > video_cfg.max_bitrate_kbps {
            info!("Bitrate excede máximo: {} > {} kbps", 
                stream_info.video_bitrate_kbps, 
                video_cfg.max_bitrate_kbps);
            return false;
        }
    }
    
    true
}

/// Verifica si el audio del stream es compatible
fn is_audio_compatible(stream_info: &StreamInfo, config: &ChannelConfig) -> bool {
    let audio_cfg = &config.audio;
    
    // 1. Verificar codec
    let codec_ok = stream_info.audio_codec == audio_cfg.codec;
    if !codec_ok {
        info!("Codec de audio incompatible: {} != {}", 
            stream_info.audio_codec, audio_cfg.codec);
        return false;
    }
    
    // 2. Verificar sample rate
    if stream_info.sample_rate != audio_cfg.sample_rate {
        info!("Sample rate incompatible: {} != {}", 
            stream_info.sample_rate, audio_cfg.sample_rate);
        return false;
    }
    
    // 3. Verificar canales
    if stream_info.channels != audio_cfg.channels {
        info!("Número de canales incompatible: {} != {}", 
            stream_info.channels, audio_cfg.channels);
        return false;
    }
    
    // 4. Verificar bitrate (tolerancia del 20%)
    let tolerance = 0.20;
    let min_acceptable = (audio_cfg.bitrate_kbps as f64 * (1.0 - tolerance)) as u32;
    let max_acceptable = (audio_cfg.bitrate_kbps as f64 * (1.0 + tolerance)) as u32;
    
    if stream_info.audio_bitrate_kbps < min_acceptable 
        || stream_info.audio_bitrate_kbps > max_acceptable {
        info!("Bitrate de audio fuera de rango: {} kbps (objetivo: {} ±{}%)", 
            stream_info.audio_bitrate_kbps, 
            audio_cfg.bitrate_kbps,
            (tolerance * 100.0) as u32);
        return false;
    }
    
    info!("Audio compatible con configuración CATV");
    true
}

/// Información sobre por qué se eligió una estrategia
#[derive(Debug, Clone, Serialize)]
pub struct StrategyReason {
    pub strategy: TranscodeStrategy,
    pub video_compatible: bool,
    pub audio_compatible: bool,
    pub reasons: Vec<String>,
}

/// Decide estrategia con información detallada del razonamiento
pub fn decide_strategy_with_reasons(
    stream_info: &StreamInfo,
    config: &ChannelConfig,
) -> StrategyReason {
    let mut reasons = Vec::new();
    
    if config.analysis.force_transcode {
        reasons.push("Transcodificación forzada por configuración".to_string());
        return StrategyReason {
            strategy: TranscodeStrategy::TranscodeBoth,
            video_compatible: false,
            audio_compatible: false,
            reasons,
        };
    }
    
    // Analizar video
    let video_compatible = is_video_compatible(stream_info, config);
    if !video_compatible {
        reasons.push(format!(
            "Video incompatible: codec={}, bitrate={}kbps, {}x{}@{}fps",
            stream_info.video_codec,
            stream_info.video_bitrate_kbps,
            stream_info.width,
            stream_info.height,
            stream_info.fps
        ));
    }
    
    // Analizar audio
    let audio_compatible = is_audio_compatible(stream_info, config);
    if !audio_compatible {
        reasons.push(format!(
            "Audio incompatible: codec={}, bitrate={}kbps, {}Hz {}ch",
            stream_info.audio_codec,
            stream_info.audio_bitrate_kbps,
            stream_info.sample_rate,
            stream_info.channels
        ));
    }
    
    let strategy = match (video_compatible, audio_compatible) {
        (true, true) => {
            if config.analysis.passthrough_if_compatible {
                reasons.push("Stream totalmente compatible, usando PassThrough".to_string());
                TranscodeStrategy::PassThrough
            } else {
                reasons.push("Passthrough deshabilitado en configuración".to_string());
                TranscodeStrategy::TranscodeBoth
            }
        }
        (false, true) => {
            reasons.push("Transcodificando solo video".to_string());
            TranscodeStrategy::TranscodeVideo
        }
        (true, false) => {
            reasons.push("Transcodificando solo audio".to_string());
            TranscodeStrategy::TranscodeAudio
        }
        (false, false) => {
            reasons.push("Transcodificando video y audio".to_string());
            TranscodeStrategy::TranscodeBoth
        }
    };
    
    StrategyReason {
        strategy,
        video_compatible,
        audio_compatible,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::VideoConfig;
    
    #[test]
    fn test_video_compatibility() {
        // Crear configuración de prueba
        let config = create_test_config();
        
        // Stream compatible
        let compatible_stream = StreamInfo {
            container_format: "mpegts".to_string(),
            duration_seconds: None,
            bitrate_kbps: 4500,
            video_codec: "h264".to_string(),
            video_codec_name: "H.264 / AVC".to_string(),
            video_profile: Some("Main".to_string()),
            video_level: Some("4.0".to_string()),
            width: 1920,
            height: 1080,
            fps: 30.0,
            video_bitrate_kbps: 4000,
            pixel_format: "yuv420p".to_string(),
            audio_codec: "aac".to_string(),
            audio_codec_name: "AAC".to_string(),
            audio_bitrate_kbps: 128,
            sample_rate: 48000,
            channels: 2,
            audio_profile: Some("LC".to_string()),
        };
        
        assert!(is_video_compatible(&compatible_stream, &config));
        
        // Stream con bitrate muy alto
        let high_bitrate_stream = StreamInfo {
            video_bitrate_kbps: 10000,
            ..compatible_stream.clone()
        };
        
        assert!(!is_video_compatible(&high_bitrate_stream, &config));
    }
    
    fn create_test_config() -> ChannelConfig {
        // Implementar creación de config de prueba
        todo!()
    }
}