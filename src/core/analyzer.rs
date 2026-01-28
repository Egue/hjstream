use crate::models::error::TranscoderError;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn, error};

/// Información detectada de un stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub container_format: String,
    pub duration_seconds: Option<f64>,
    pub bitrate_kbps: u32,
    
    // Video
    pub video_codec: String,
    pub video_codec_name: String,
    pub video_profile: Option<String>,
    pub video_level: Option<String>,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub video_bitrate_kbps: u32,
    pub pixel_format: String,
    
    // Audio
    pub audio_codec: String,
    pub audio_codec_name: String,
    pub audio_bitrate_kbps: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub audio_profile: Option<String>,
}

/// Analizador de streams usando FFprobe
pub struct StreamAnalyzer;

impl StreamAnalyzer {
    /// Analiza un stream y retorna información detallada
    pub async fn analyze(input_url: &str) -> Result<StreamInfo, TranscoderError> {
        info!("Analizando stream: {}", input_url);
        
        // Ejecutar ffprobe
        let output = Command::new("ffprobe")
            .args(&[
                "-v", "quiet",
                "-print_format", "json",
                "-show_format",
                "-show_streams",
                input_url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| TranscoderError::AnalysisFailed(format!("Error ejecutando ffprobe: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TranscoderError::AnalysisFailed(format!("FFprobe falló: {}", stderr)));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parsear JSON
        let probe_data: ProbeData = serde_json::from_str(&stdout)
            .map_err(|e| TranscoderError::AnalysisFailed(format!("Error parseando JSON: {}", e)))?;
        
        // Extraer información
        Self::extract_stream_info(probe_data)
    }
    
    /// Análisis rápido solo para detectar si el stream está vivo
    pub async fn quick_check(input_url: &str) -> Result<bool, TranscoderError> {
        info!("Quick check del stream: {}", input_url);
        
        let output = Command::new("ffprobe")
            .args(&[
                "-v", "error",
                "-show_entries", "format=duration",
                "-of", "default=noprint_wrappers=1:nokey=1",
                "-timeout", "5000000", // 5 segundos
                input_url,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| TranscoderError::AnalysisFailed(e.to_string()))?;
        
        Ok(output.status.success())
    }
    
    fn extract_stream_info(probe_data: ProbeData) -> Result<StreamInfo, TranscoderError> {
        // Buscar stream de video
        let video_stream = probe_data.streams.iter()
            .find(|s| s.codec_type == "video")
            .ok_or_else(|| TranscoderError::AnalysisFailed("No se encontró stream de video".to_string()))?;
        
        // Buscar stream de audio
        let audio_stream = probe_data.streams.iter()
            .find(|s| s.codec_type == "audio")
            .ok_or_else(|| TranscoderError::AnalysisFailed("No se encontró stream de audio".to_string()))?;
        
        // Parsear FPS
        let fps = Self::parse_fps(&video_stream.r_frame_rate)?;
        
        // Construir StreamInfo
        let info = StreamInfo {
            container_format: probe_data.format.format_name.clone(),
            duration_seconds: probe_data.format.duration.and_then(|d| d.parse::<f64>().ok()),
            bitrate_kbps: probe_data.format.bit_rate
                .and_then(|b| b.parse::<u32>().ok())
                .unwrap_or(0) / 1000,
            
            // Video
            video_codec: video_stream.codec_name.clone(),
            video_codec_name: video_stream.codec_long_name.clone().unwrap_or_default(),
            video_profile: video_stream.profile.clone(),
            video_level: video_stream.level.map(|l| l.to_string()),
            width: video_stream.width.unwrap_or(0),
            height: video_stream.height.unwrap_or(0),
            fps,
            video_bitrate_kbps: video_stream.bit_rate
                .and_then(|b| b.parse::<u32>().ok())
                .unwrap_or(0) / 1000,
            pixel_format: video_stream.pix_fmt.clone().unwrap_or_default(),
            
            // Audio
            audio_codec: audio_stream.codec_name.clone(),
            audio_codec_name: audio_stream.codec_long_name.clone().unwrap_or_default(),
            audio_bitrate_kbps: audio_stream.bit_rate
                .and_then(|b| b.parse::<u32>().ok())
                .unwrap_or(0) / 1000,
            sample_rate: audio_stream.sample_rate
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0),
            channels: audio_stream.channels.unwrap_or(0) as u8,
            audio_profile: audio_stream.profile.clone(),
        };
        
        info!("Stream analizado exitosamente:");
        info!("  Video: {} {}x{} @ {} fps ({} kbps)", 
            info.video_codec, info.width, info.height, info.fps, info.video_bitrate_kbps);
        info!("  Audio: {} {}Hz {}ch ({} kbps)", 
            info.audio_codec, info.sample_rate, info.channels, info.audio_bitrate_kbps);
        
        Ok(info)
    }
    
    fn parse_fps(r_frame_rate: &str) -> Result<f64, TranscoderError> {
        // El formato es "num/den" (ej: "30000/1001" para 29.97 fps)
        let parts: Vec<&str> = r_frame_rate.split('/').collect();
        
        if parts.len() != 2 {
            return Err(TranscoderError::AnalysisFailed(
                format!("Formato de FPS inválido: {}", r_frame_rate)
            ));
        }
        
        let num = parts[0].parse::<f64>()
            .map_err(|_| TranscoderError::AnalysisFailed("Error parseando numerador de FPS".to_string()))?;
        let den = parts[1].parse::<f64>()
            .map_err(|_| TranscoderError::AnalysisFailed("Error parseando denominador de FPS".to_string()))?;
        
        if den == 0.0 {
            return Err(TranscoderError::AnalysisFailed("Denominador de FPS es cero".to_string()));
        }
        
        Ok(num / den)
    }
}

// Estructuras para parsear JSON de ffprobe
#[derive(Debug, Deserialize)]
struct ProbeData {
    streams: Vec<StreamData>,
    format: FormatData,
}

#[derive(Debug, Deserialize)]
struct StreamData {
    codec_type: String,
    codec_name: String,
    codec_long_name: Option<String>,
    profile: Option<String>,
    level: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: String,
    bit_rate: Option<String>,
    pix_fmt: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct FormatData {
    format_name: String,
    duration: Option<String>,
    bit_rate: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_fps() {
        assert_eq!(StreamAnalyzer::parse_fps("30/1").unwrap(), 30.0);
        assert_eq!(StreamAnalyzer::parse_fps("30000/1001").unwrap(), 29.970029970029969);
        assert_eq!(StreamAnalyzer::parse_fps("24000/1001").unwrap(), 23.976023976023978);
    }
}