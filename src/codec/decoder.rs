//! Decoder para análisis de streams de entrada

use serde::{Deserialize, Serialize};

/// Información de un stream decodificado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub codec_type: String,  // video, audio, subtitle
    pub codec_name: String,  // h264, aac, etc
    pub codec_long_name: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub framerate: Option<f32>,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub duration_seconds: Option<f64>,
}

/// Información general de un contenedor/archivo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub format_name: String,
    pub format_long_name: String,
    pub duration_seconds: Option<f64>,
    pub bitrate: Option<u32>,
    pub streams: Vec<StreamInfo>,
}

/// Decoder para extraer información de streams
#[derive(Debug, Clone)]
pub struct Decoder {
    pub input_url: String,
}

impl Decoder {
    /// Crear un decoder para una URL de entrada
    pub fn new(input_url: &str) -> Self {
        Self {
            input_url: input_url.to_string(),
        }
    }

    /// Construir comando FFprobe para analizar stream
    pub fn build_ffprobe_command(&self) -> Vec<String> {
        vec![
            "ffprobe".to_string(),
            "-v".to_string(),
            "quiet".to_string(),
            "-print_format".to_string(),
            "json".to_string(),
            "-show_format".to_string(),
            "-show_streams".to_string(),
            self.input_url.clone(),
        ]
    }

    /// Obtener información del video (primer stream de video)
    pub fn extract_video_info(container: &ContainerInfo) -> Option<StreamInfo> {
        container
            .streams
            .iter()
            .find(|s| s.codec_type == "video")
            .cloned()
    }

    /// Obtener información del audio (primer stream de audio)
    pub fn extract_audio_info(container: &ContainerInfo) -> Option<StreamInfo> {
        container
            .streams
            .iter()
            .find(|s| s.codec_type == "audio")
            .cloned()
    }

    /// Obtener todas las pistas de audio
    pub fn extract_all_audio_streams(container: &ContainerInfo) -> Vec<StreamInfo> {
        container
            .streams
            .iter()
            .filter(|s| s.codec_type == "audio")
            .cloned()
            .collect()
    }

    /// Obtener información de subtítulos
    pub fn extract_subtitle_info(container: &ContainerInfo) -> Vec<StreamInfo> {
        container
            .streams
            .iter()
            .filter(|s| s.codec_type == "subtitle")
            .cloned()
            .collect()
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_creation() {
        let decoder = Decoder::new("srt://localhost:5000");
        assert_eq!(decoder.input_url, "srt://localhost:5000");
    }

    #[test]
    fn test_ffprobe_command_building() {
        let decoder = Decoder::new("srt://localhost:5000");
        let cmd = decoder.build_ffprobe_command();
        assert!(cmd.contains(&"ffprobe".to_string()));
        assert!(cmd.contains(&"srt://localhost:5000".to_string()));
    }

    #[test]
    fn test_stream_extraction() {
        let container = ContainerInfo {
            format_name: "mpegts".to_string(),
            format_long_name: "MPEG-TS (MPEG-2 Transport Stream)".to_string(),
            duration_seconds: Some(3600.0),
            bitrate: Some(5000),
            streams: vec![
                StreamInfo {
                    codec_type: "video".to_string(),
                    codec_name: "h264".to_string(),
                    codec_long_name: "H.264 / AVC / MPEG-4 AVC".to_string(),
                    width: Some(1920),
                    height: Some(1080),
                    framerate: Some(29.97),
                    bitrate: Some(4000),
                    sample_rate: None,
                    channels: None,
                    duration_seconds: None,
                },
                StreamInfo {
                    codec_type: "audio".to_string(),
                    codec_name: "aac".to_string(),
                    codec_long_name: "AAC (Advanced Audio Coding)".to_string(),
                    width: None,
                    height: None,
                    framerate: None,
                    bitrate: Some(128),
                    sample_rate: Some(48000),
                    channels: Some(2),
                    duration_seconds: None,
                },
            ],
        };

        let video = Decoder::extract_video_info(&container);
        assert!(video.is_some());
        assert_eq!(video.unwrap().codec_name, "h264");

        let audio = Decoder::extract_audio_info(&container);
        assert!(audio.is_some());
        assert_eq!(audio.unwrap().codec_name, "aac");
    }
}
