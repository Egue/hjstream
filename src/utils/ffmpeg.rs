use crate::models::error::TranscoderError;
use std::process::{Command, Stdio};
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};

/// Helper para trabajar con FFmpeg
pub struct FfmpegHelper;

impl FfmpegHelper {
    /// Verificar si FFmpeg está instalado
    pub fn is_installed() -> bool {
        Command::new("ffmpeg")
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
    
    /// Obtener versión de FFmpeg
    pub fn get_version() -> Result<FfmpegVersion, TranscoderError> {
        let output = Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map_err(|e| TranscoderError::Unknown(format!("Error ejecutando ffmpeg: {}", e)))?;
        
        if !output.status.success() {
            return Err(TranscoderError::Unknown("FFmpeg no disponible".to_string()));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_version(&stdout)
    }
    
    /// Verificar si un codec está disponible
    pub fn has_codec(codec_name: &str) -> bool {
        let output = Command::new("ffmpeg")
            .args(&["-codecs"])
            .output();
        
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains(codec_name)
        } else {
            false
        }
    }
    
    /// Verificar si un encoder está disponible
    pub fn has_encoder(encoder_name: &str) -> bool {
        let output = Command::new("ffmpeg")
            .args(&["-encoders"])
            .output();
        
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains(encoder_name)
        } else {
            false
        }
    }
    
    /// Listar encoders de hardware disponibles
    pub fn list_hw_encoders() -> Vec<String> {
        let mut encoders = Vec::new();
        
        let hw_encoders = vec![
            "h264_nvenc",    // NVIDIA
            "hevc_nvenc",
            "h264_qsv",      // Intel Quick Sync
            "hevc_qsv",
            "h264_vaapi",    // VA-API (Intel/AMD)
            "hevc_vaapi",
            "h264_videotoolbox", // macOS
            "hevc_videotoolbox",
            "h264_amf",      // AMD
            "hevc_amf",
        ];
        
        for encoder in hw_encoders {
            if Self::has_encoder(encoder) {
                encoders.push(encoder.to_string());
            }
        }
        
        encoders
    }
    
    /// Detectar mejor encoder disponible para H.264
    pub fn best_h264_encoder() -> String {
        // Prioridad: NVENC > QSV > VAAPI > Software
        let hw_encoders = Self::list_hw_encoders();
        
        if hw_encoders.contains(&"h264_nvenc".to_string()) {
            info!("Usando NVIDIA NVENC para H.264");
            return "h264_nvenc".to_string();
        }
        
        if hw_encoders.contains(&"h264_qsv".to_string()) {
            info!("Usando Intel QSV para H.264");
            return "h264_qsv".to_string();
        }
        
        if hw_encoders.contains(&"h264_vaapi".to_string()) {
            info!("Usando VA-API para H.264");
            return "h264_vaapi".to_string();
        }
        
        info!("Usando encoder de software libx264");
        "libx264".to_string()
    }
    
    /// Verificar capacidades de FFmpeg
    pub fn check_capabilities() -> FfmpegCapabilities {
        FfmpegCapabilities {
            version: Self::get_version().ok(),
            has_libx264: Self::has_encoder("libx264"),
            has_libx265: Self::has_encoder("libx265"),
            has_aac: Self::has_encoder("aac"),
            has_opus: Self::has_encoder("libopus"),
            has_nvenc: Self::has_encoder("h264_nvenc"),
            has_qsv: Self::has_encoder("h264_qsv"),
            has_vaapi: Self::has_encoder("h264_vaapi"),
            has_srt: Self::has_protocol("srt"),
            hw_encoders: Self::list_hw_encoders(),
        }
    }
    
    /// Verificar si un protocolo está disponible
    pub fn has_protocol(protocol: &str) -> bool {
        let output = Command::new("ffmpeg")
            .args(&["-protocols"])
            .output();
        
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains(protocol)
        } else {
            false
        }
    }
    
    /// Parsear versión de FFmpeg
    fn parse_version(output: &str) -> Result<FfmpegVersion, TranscoderError> {
        // Formato: "ffmpeg version 4.4.2-0ubuntu0.22.04.1"
        let lines: Vec<&str> = output.lines().collect();
        
        if lines.is_empty() {
            return Err(TranscoderError::Unknown("No se pudo parsear versión".to_string()));
        }
        
        let first_line = lines[0];
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        
        if parts.len() < 3 {
            return Err(TranscoderError::Unknown("Formato de versión inválido".to_string()));
        }
        
        let version_str = parts[2];
        let version_parts: Vec<&str> = version_str.split('-').next().unwrap_or("").split('.').collect();
        
        let major = version_parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = version_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = version_parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        
        Ok(FfmpegVersion {
            major,
            minor,
            patch,
            full_string: version_str.to_string(),
        })
    }
    
    /// Validar argumentos de FFmpeg antes de ejecutar
    pub fn validate_args(args: &[String]) -> Result<(), TranscoderError> {
        // Verificar que no haya argumentos peligrosos
        for arg in args {
            if arg.contains("&&") || arg.contains(";") || arg.contains("|") {
                return Err(TranscoderError::InvalidConfig(
                    "Argumentos contienen caracteres peligrosos".to_string()
                ));
            }
        }
        Ok(())
    }
    
    /// Ejecutar FFmpeg y capturar salida
    pub fn execute(args: Vec<String>) -> Result<std::process::Output, TranscoderError> {
        Self::validate_args(&args)?;
        
        Command::new("ffmpeg")
            .args(&args)
            .output()
            .map_err(|e| TranscoderError::ProcessSpawnFailed(e.to_string()))
    }
}

/// Información de versión de FFmpeg
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfmpegVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub full_string: String,
}

impl FfmpegVersion {
    /// Verificar si la versión es al menos la especificada
    pub fn is_at_least(&self, major: u32, minor: u32, patch: u32) -> bool {
        if self.major > major {
            return true;
        }
        if self.major == major && self.minor > minor {
            return true;
        }
        if self.major == major && self.minor == minor && self.patch >= patch {
            return true;
        }
        false
    }
}

impl std::fmt::Display for FfmpegVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Capacidades de FFmpeg detectadas
#[derive(Debug, Clone, Serialize)]
pub struct FfmpegCapabilities {
    pub version: Option<FfmpegVersion>,
    pub has_libx264: bool,
    pub has_libx265: bool,
    pub has_aac: bool,
    pub has_opus: bool,
    pub has_nvenc: bool,
    pub has_qsv: bool,
    pub has_vaapi: bool,
    pub has_srt: bool,
    pub hw_encoders: Vec<String>,
}

impl FfmpegCapabilities {
    pub fn print_summary(&self) {
        info!("=== FFmpeg Capabilities ===");
        if let Some(version) = &self.version {
            info!("Versión: {}", version);
        }
        info!("Software Encoders:");
        info!("  - libx264: {}", if self.has_libx264 { "✓" } else { "✗" });
        info!("  - libx265: {}", if self.has_libx265 { "✓" } else { "✗" });
        info!("  - AAC: {}", if self.has_aac { "✓" } else { "✗" });
        info!("Hardware Encoders:");
        info!("  - NVENC: {}", if self.has_nvenc { "✓" } else { "✗" });
        info!("  - QSV: {}", if self.has_qsv { "✓" } else { "✗" });
        info!("  - VAAPI: {}", if self.has_vaapi { "✓" } else { "✗" });
        info!("Protocolos:");
        info!("  - SRT: {}", if self.has_srt { "✓" } else { "✗" });
        
        if !self.hw_encoders.is_empty() {
            info!("Encoders de hardware disponibles: {}", self.hw_encoders.join(", "));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ffmpeg_installed() {
        // Este test puede fallar en CI sin FFmpeg
        let installed = FfmpegHelper::is_installed();
        println!("FFmpeg instalado: {}", installed);
    }
    
    #[test]
    fn test_version_parsing() {
        let output = "ffmpeg version 4.4.2-0ubuntu0.22.04.1 Copyright (c) 2000-2021";
        let version = FfmpegHelper::parse_version(output).unwrap();
        
        assert_eq!(version.major, 4);
        assert_eq!(version.minor, 4);
        assert_eq!(version.patch, 2);
    }
    
    #[test]
    fn test_version_comparison() {
        let version = FfmpegVersion {
            major: 4,
            minor: 4,
            patch: 2,
            full_string: "4.4.2".to_string(),
        };
        
        assert!(version.is_at_least(4, 4, 2));
        assert!(version.is_at_least(4, 4, 1));
        assert!(version.is_at_least(4, 3, 0));
        assert!(!version.is_at_least(4, 4, 3));
        assert!(!version.is_at_least(5, 0, 0));
    }
}