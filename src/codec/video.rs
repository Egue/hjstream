//! Configuración y parámetros de codificación de video

use serde::{Deserialize, Serialize};

/// Encoder de video con soporte para múltiples codecs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEncoder {
    pub codec: String,         // h264, h265, vp8, vp9, av1
    pub profile: String,       // baseline, main, high
    pub level: String,         // 3.0, 4.0, 5.0, etc
    pub bitrate_kbps: u32,
    pub max_bitrate_kbps: u32,
    pub buffer_size_kb: u32,
    pub framerate: u32,
    pub gop_size: u32,         // Keyframe interval
    pub preset: String,        // ultrafast, superfast, veryfast, faster, fast, medium, slow, slower, veryslow
    pub tune: String,          // film, animation, grain, stillimage
    pub rate_control: String,  // cbr, vbr, crf
    pub pix_fmt: String,       // yuv420p, yuv422p, yuv444p
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl Default for VideoEncoder {
    fn default() -> Self {
        Self {
            codec: "libx264".to_string(),
            profile: "main".to_string(),
            level: "4.0".to_string(),
            bitrate_kbps: 5000,
            max_bitrate_kbps: 6000,
            buffer_size_kb: 2000,
            framerate: 30,
            gop_size: 60,
            preset: "medium".to_string(),
            tune: "film".to_string(),
            rate_control: "cbr".to_string(),
            pix_fmt: "yuv420p".to_string(),
            width: None,
            height: None,
        }
    }
}

impl VideoEncoder {
    /// Crear un encoder de video personalizado
    pub fn new(codec: &str) -> Self {
        let mut encoder = Self::default();
        encoder.codec = codec.to_string();
        encoder
    }

    /// Construir los argumentos de FFmpeg para codificación de video
    pub fn build_ffmpeg_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Codec
        args.push("-c:v".to_string());
        args.push(self.codec.clone());

        // Perfil y nivel
        args.push("-profile:v".to_string());
        args.push(self.profile.clone());

        args.push("-level:v".to_string());
        args.push(self.level.clone());

        // Bitrate
        args.push("-b:v".to_string());
        args.push(format!("{}k", self.bitrate_kbps));

        args.push("-maxrate:v".to_string());
        args.push(format!("{}k", self.max_bitrate_kbps));

        args.push("-bufsize:v".to_string());
        args.push(format!("{}k", self.buffer_size_kb));

        // Framerate
        args.push("-r".to_string());
        args.push(self.framerate.to_string());

        // GOP (keyframe interval)
        args.push("-g".to_string());
        args.push(self.gop_size.to_string());

        // Preset
        args.push("-preset".to_string());
        args.push(self.preset.clone());

        // Tune
        if !self.tune.is_empty() {
            args.push("-tune".to_string());
            args.push(self.tune.clone());
        }

        // Pixel format
        args.push("-pix_fmt".to_string());
        args.push(self.pix_fmt.clone());

        // Escalado si está especificado
        if let (Some(w), Some(h)) = (self.width, self.height) {
            args.push("-vf".to_string());
            args.push(format!("scale={}:{}", w, h));
        }

        args
    }

    /// Validar configuración de video
    pub fn validate(&self) -> Result<(), String> {
        if self.framerate == 0 {
            return Err("Framerate debe ser > 0".to_string());
        }
        if self.bitrate_kbps == 0 {
            return Err("Bitrate debe ser > 0".to_string());
        }
        if self.bitrate_kbps > self.max_bitrate_kbps {
            return Err("Bitrate no puede ser mayor que max_bitrate".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_video_encoder() {
        let encoder = VideoEncoder::default();
        assert_eq!(encoder.codec, "libx264");
        assert_eq!(encoder.framerate, 30);
    }

    #[test]
    fn test_video_encoder_validation() {
        let mut encoder = VideoEncoder::default();
        assert!(encoder.validate().is_ok());

        encoder.framerate = 0;
        assert!(encoder.validate().is_err());
    }

    #[test]
    fn test_ffmpeg_args_generation() {
        let encoder = VideoEncoder::default();
        let args = encoder.build_ffmpeg_args();
        assert!(!args.is_empty());
        assert!(args.contains(&"-c:v".to_string()));
        assert!(args.contains(&"libx264".to_string()));
    }
}
