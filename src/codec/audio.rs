//! Configuración y parámetros de codificación de audio

use serde::{Deserialize, Serialize};

/// Encoder de audio con soporte para múltiples codecs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEncoder {
    pub codec: String,           // aac, libmp3lame, libopus, libvorbis, eac3, ac3
    pub bitrate_kbps: u32,
    pub sample_rate: u32,        // 48000, 44100, 32000
    pub channels: u8,            // 1 (mono), 2 (estéreo), 6 (5.1)
    pub channel_layout: String,  // mono, stereo, 5.1, etc
    pub profile: Option<String>, // Para AAC: aac_low, aac_he, aac_he_v2
}

impl Default for AudioEncoder {
    fn default() -> Self {
        Self {
            codec: "aac".to_string(),
            bitrate_kbps: 128,
            sample_rate: 48000,
            channels: 2,
            channel_layout: "stereo".to_string(),
            profile: Some("aac_low".to_string()),
        }
    }
}

impl AudioEncoder {
    /// Crear un encoder de audio personalizado
    pub fn new(codec: &str) -> Self {
        let mut encoder = Self::default();
        encoder.codec = codec.to_string();
        encoder
    }

    /// Encoder para AC3 (5.1 surround, común en CATV)
    pub fn ac3_surround() -> Self {
        Self {
            codec: "ac3".to_string(),
            bitrate_kbps: 448,
            sample_rate: 48000,
            channels: 6,
            channel_layout: "5.1".to_string(),
            profile: None,
        }
    }

    /// Encoder para E-AC3 (versión mejorada, más comprimida)
    pub fn eac3_surround() -> Self {
        Self {
            codec: "eac3".to_string(),
            bitrate_kbps: 256,
            sample_rate: 48000,
            channels: 6,
            channel_layout: "5.1".to_string(),
            profile: None,
        }
    }

    /// Encoder para AAC estéreo (comprimido)
    pub fn aac_stereo() -> Self {
        Self::default()
    }

    /// Construir los argumentos de FFmpeg para codificación de audio
    pub fn build_ffmpeg_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Codec
        args.push("-c:a".to_string());
        args.push(self.codec.clone());

        // Bitrate
        args.push("-b:a".to_string());
        args.push(format!("{}k", self.bitrate_kbps));

        // Sample rate
        args.push("-ar".to_string());
        args.push(self.sample_rate.to_string());

        // Canales y layout
        args.push("-ac".to_string());
        args.push(self.channels.to_string());

        args.push("-channel_layout".to_string());
        args.push(self.channel_layout.clone());

        // Profile (si aplica)
        if let Some(ref profile) = self.profile {
            args.push("-profile:a".to_string());
            args.push(profile.clone());
        }

        args
    }

    /// Validar configuración de audio
    pub fn validate(&self) -> Result<(), String> {
        if self.bitrate_kbps == 0 {
            return Err("Bitrate de audio debe ser > 0".to_string());
        }
        if self.channels == 0 {
            return Err("Canales de audio debe ser > 0".to_string());
        }
        match self.sample_rate {
            48000 | 44100 | 32000 | 22050 | 16000 => Ok(()),
            _ => Err("Sample rate no soportado".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_audio_encoder() {
        let encoder = AudioEncoder::default();
        assert_eq!(encoder.codec, "aac");
        assert_eq!(encoder.channels, 2);
    }

    #[test]
    fn test_audio_encoder_presets() {
        let ac3 = AudioEncoder::ac3_surround();
        assert_eq!(ac3.codec, "ac3");
        assert_eq!(ac3.channels, 6);

        let eac3 = AudioEncoder::eac3_surround();
        assert_eq!(eac3.codec, "eac3");

        let aac = AudioEncoder::aac_stereo();
        assert_eq!(aac.codec, "aac");
    }

    #[test]
    fn test_audio_validation() {
        let encoder = AudioEncoder::default();
        assert!(encoder.validate().is_ok());
    }

    #[test]
    fn test_ffmpeg_args_generation() {
        let encoder = AudioEncoder::default();
        let args = encoder.build_ffmpeg_args();
        assert!(!args.is_empty());
        assert!(args.contains(&"-c:a".to_string()));
    }
}
