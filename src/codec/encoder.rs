//! Encoder general que combina video y audio

use super::video::VideoEncoder;
use super::audio::AudioEncoder;
use serde::{Deserialize, Serialize};

/// Encoder completo con configuración de video y audio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encoder {
    pub video: VideoEncoder,
    pub audio: AudioEncoder,
    pub container: String,  // mpegts, mp4, matroska
    pub metadata: EncoderMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderMetadata {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
}

impl Default for Encoder {
    fn default() -> Self {
        Self {
            video: VideoEncoder::default(),
            audio: AudioEncoder::default(),
            container: "mpegts".to_string(),
            metadata: EncoderMetadata {
                name: "default".to_string(),
                description: "Encoder por defecto".to_string(),
                tags: vec!["standard".to_string()],
            },
        }
    }
}

impl Encoder {
    /// Crear un encoder para CATV estándar
    pub fn catv_standard() -> Self {
        Self {
            video: VideoEncoder {
                codec: "libx264".to_string(),
                profile: "main".to_string(),
                level: "4.0".to_string(),
                bitrate_kbps: 5000,
                max_bitrate_kbps: 6000,
                buffer_size_kb: 2000,
                framerate: 29,  // 29.97 redondeado
                gop_size: 58,   // 2 segundos a 29.97fps
                preset: "medium".to_string(),
                tune: "film".to_string(),
                rate_control: "cbr".to_string(),
                pix_fmt: "yuv420p".to_string(),
                width: Some(1920),
                height: Some(1080),
            },
            audio: AudioEncoder::aac_stereo(),
            container: "mpegts".to_string(),
            metadata: EncoderMetadata {
                name: "catv_standard".to_string(),
                description: "Encoder estándar para CATV HD".to_string(),
                tags: vec!["catv".to_string(), "hd".to_string()],
            },
        }
    }

    /// Crear un encoder para CATV con sonido envolvente
    pub fn catv_surround() -> Self {
        Self {
            video: VideoEncoder {
                codec: "libx264".to_string(),
                profile: "main".to_string(),
                level: "4.0".to_string(),
                bitrate_kbps: 5000,
                max_bitrate_kbps: 6000,
                buffer_size_kb: 2000,
                framerate: 29,
                gop_size: 58,
                preset: "medium".to_string(),
                tune: "film".to_string(),
                rate_control: "cbr".to_string(),
                pix_fmt: "yuv420p".to_string(),
                width: Some(1920),
                height: Some(1080),
            },
            audio: AudioEncoder::ac3_surround(),
            container: "mpegts".to_string(),
            metadata: EncoderMetadata {
                name: "catv_surround".to_string(),
                description: "Encoder CATV con audio AC3 5.1".to_string(),
                tags: vec!["catv".to_string(), "surround".to_string()],
            },
        }
    }

    /// Crear un encoder optimizado para streaming (menor bitrate)
    pub fn stream_optimized() -> Self {
        Self {
            video: VideoEncoder {
                bitrate_kbps: 2500,
                max_bitrate_kbps: 3000,
                buffer_size_kb: 1000,
                framerate: 24,
                ..VideoEncoder::default()
            },
            audio: AudioEncoder {
                bitrate_kbps: 96,
                ..AudioEncoder::default()
            },
            container: "mpegts".to_string(),
            metadata: EncoderMetadata {
                name: "stream_optimized".to_string(),
                description: "Encoder optimizado para bajo bitrate".to_string(),
                tags: vec!["streaming".to_string(), "low_bitrate".to_string()],
            },
        }
    }

    /// Crear un encoder de máxima calidad
    pub fn high_quality() -> Self {
        Self {
            video: VideoEncoder {
                codec: "libx265".to_string(),  // H.265 para mejor compresión
                profile: "main10".to_string(),
                level: "5.0".to_string(),
                bitrate_kbps: 8000,
                max_bitrate_kbps: 10000,
                buffer_size_kb: 3000,
                framerate: 60,
                gop_size: 120,
                preset: "slow".to_string(),
                tune: "film".to_string(),
                rate_control: "cbr".to_string(),
                pix_fmt: "yuv420p10le".to_string(),
                width: Some(3840),
                height: Some(2160),
            },
            audio: AudioEncoder::eac3_surround(),
            container: "mpegts".to_string(),
            metadata: EncoderMetadata {
                name: "high_quality".to_string(),
                description: "Encoder de máxima calidad con H.265".to_string(),
                tags: vec!["high_quality".to_string(), "4k".to_string()],
            },
        }
    }

    /// Construir comando FFmpeg completo
    pub fn build_ffmpeg_command(&self) -> Result<Vec<String>, String> {
        self.validate()?;

        let mut cmd = Vec::new();

        // Video arguments
        cmd.extend(self.video.build_ffmpeg_args());

        // Audio arguments
        cmd.extend(self.audio.build_ffmpeg_args());

        // Output format
        cmd.push("-f".to_string());
        cmd.push(self.container.clone());

        Ok(cmd)
    }

    /// Validar configuración completa del encoder
    pub fn validate(&self) -> Result<(), String> {
        self.video.validate()?;
        self.audio.validate()?;
        Ok(())
    }

    /// Obtener descripción detallada del encoder
    pub fn describe(&self) -> String {
        format!(
            "Encoder: {}\n  Video: {} {}p @ {}fps, {}kbps\n  Audio: {} {}ch @ {}kHz, {}kbps\n  Descripción: {}",
            self.metadata.name,
            self.video.codec,
            self.video.height.unwrap_or(0),
            self.video.framerate,
            self.video.bitrate_kbps,
            self.audio.codec,
            self.audio.channels,
            self.audio.sample_rate,
            self.audio.bitrate_kbps,
            self.metadata.description
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_encoder() {
        let encoder = Encoder::default();
        assert!(encoder.validate().is_ok());
    }

    #[test]
    fn test_catv_presets() {
        let std = Encoder::catv_standard();
        assert_eq!(std.video.codec, "libx264");
        assert!(std.validate().is_ok());

        let surr = Encoder::catv_surround();
        assert_eq!(surr.audio.codec, "ac3");
        assert!(surr.validate().is_ok());
    }

    #[test]
    fn test_ffmpeg_command_building() {
        let encoder = Encoder::catv_standard();
        let cmd = encoder.build_ffmpeg_command();
        assert!(cmd.is_ok());
        let args = cmd.unwrap();
        assert!(!args.is_empty());
    }
}
