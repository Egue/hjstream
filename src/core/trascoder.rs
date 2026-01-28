use crate::config::loader::ChannelConfig;
use crate::core::strategy::TranscodeStrategy;
use crate::models::stats::ChannelStats;
use crate::models::error::TranscoderError;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

/// Motor de transcodificación que ejecuta FFmpeg
#[derive(Clone)]
pub struct Transcoder {
    config: ChannelConfig,
    strategy: TranscodeStrategy,
    process: Arc<RwLock<Option<Child>>>,
}

impl Transcoder {
    pub fn new(config: ChannelConfig, strategy: TranscodeStrategy) -> Result<Self, TranscoderError> {
        Ok(Self {
            config,
            strategy,
            process: Arc::new(RwLock::new(None)),
        })
    }
    
    pub async fn run(&mut self, stats: Arc<RwLock<ChannelStats>>) -> Result<(), TranscoderError> {
        info!("Iniciando transcoder para canal: {}", self.config.id);
        
        // Construir comando FFmpeg según estrategia
        let mut cmd = self.build_ffmpeg_command();
        
        // Spawn proceso
        let mut child = cmd.spawn()
            .map_err(|e| TranscoderError::ProcessSpawnFailed(e.to_string()))?;
        
        // Capturar stderr para estadísticas
        if let Some(stderr) = child.stderr.take() {
            let channel_id = self.config.id.clone();
            tokio::spawn(async move {
                Self::parse_ffmpeg_output(stderr, stats, channel_id).await;
            });
        }
        
        // Guardar proceso
        {
            let mut process = self.process.write().await;
            *process = Some(child);
        }
        
        // Esperar a que termine (o sea detenido)
        let mut process = self.process.write().await;
        if let Some(child) = process.as_mut() {
            match child.wait().await {
                Ok(status) => {
                    info!("Proceso FFmpeg terminó con status: {:?}", status);
                }
                Err(e) => {
                    error!("Error esperando proceso: {}", e);
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<(), TranscoderError> {
        let mut process = self.process.write().await;
        
        if let Some(child) = process.as_mut() {
            info!("Deteniendo proceso FFmpeg");
            child.kill().await
                .map_err(|e| TranscoderError::ProcessKillFailed(e.to_string()))?;
        }
        
        *process = None;
        Ok(())
    }
    
    fn build_ffmpeg_command(&self) -> Command {
        let mut cmd = Command::new("ffmpeg");
        
        // Argumentos base
        cmd.args(&["-hide_banner", "-stats", "-loglevel", "info"]);
        
        // Construir según estrategia
        match &self.strategy {
            TranscodeStrategy::PassThrough => {
                self.build_passthrough_command(&mut cmd);
            }
            TranscodeStrategy::TranscodeVideo => {
                self.build_transcode_video_command(&mut cmd);
            }
            TranscodeStrategy::TranscodeAudio => {
                self.build_transcode_audio_command(&mut cmd);
            }
            TranscodeStrategy::TranscodeBoth => {
                self.build_transcode_both_command(&mut cmd);
            }
        }
        
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());
        
        cmd
    }
    
    fn build_passthrough_command(&self, cmd: &mut Command) {
        info!("Construyendo comando PassThrough (copy)");
        
        cmd.args(&[
            "-fflags", "+genpts",
            "-i", &self.config.input.url,
            "-c", "copy",
            "-f", "mpegts",
            "-mpegts_copyts", "1",
            "-y",
        ]);
        
        // Output UDP
        self.add_output_args(cmd);
    }
    
    fn build_transcode_video_command(&self, cmd: &mut Command) {
        info!("Construyendo comando de transcodificación de video");
        
        cmd.args(&[
            "-fflags", "+genpts",
            "-i", &self.config.input.url,
        ]);
        
        // Video encoding
        self.add_video_encoding_args(cmd);
        
        // Audio copy
        cmd.args(&["-c:a", "copy"]);
        
        // Output
        cmd.args(&["-f", "mpegts", "-y"]);
        self.add_output_args(cmd);
    }
    
    fn build_transcode_audio_command(&self, cmd: &mut Command) {
        info!("Construyendo comando de transcodificación de audio");
        
        cmd.args(&[
            "-fflags", "+genpts",
            "-i", &self.config.input.url,
            "-c:v", "copy",
        ]);
        
        // Audio encoding
        self.add_audio_encoding_args(cmd);
        
        // Output
        cmd.args(&["-f", "mpegts", "-y"]);
        self.add_output_args(cmd);
    }
    
    fn build_transcode_both_command(&self, cmd: &mut Command) {
        info!("Construyendo comando de transcodificación completa");
        
        cmd.args(&[
            "-fflags", "+genpts",
            "-i", &self.config.input.url,
        ]);
        
        // Video encoding
        self.add_video_encoding_args(cmd);
        
        // Audio encoding
        self.add_audio_encoding_args(cmd);
        
        // Output
        cmd.args(&["-f", "mpegts", "-y"]);
        self.add_output_args(cmd);
    }
    
    fn add_video_encoding_args(&self, cmd: &mut Command) {
        let video = &self.config.video;
        
        // Codec
        if video.hardware_acceleration.enabled {
            match video.hardware_acceleration.r#type.as_str() {
                "nvenc" => cmd.args(&["-c:v", "h264_nvenc"]),
                "vaapi" => cmd.args(&["-c:v", "h264_vaapi"]),
                "qsv" => cmd.args(&["-c:v", "h264_qsv"]),
                _ => cmd.args(&["-c:v", &video.codec]),
            };
        } else {
            cmd.args(&["-c:v", &format!("lib{}", video.codec)]);
        }
        
        // Perfil y nivel
        cmd.args(&[
            "-profile:v", &video.profile,
            "-level", &video.level,
        ]);
        
        // Rate control
        match video.rate_control.as_str() {
            "cbr" => {
                cmd.args(&[
                    "-b:v", &format!("{}k", video.bitrate_kbps),
                    "-minrate", &format!("{}k", video.bitrate_kbps),
                    "-maxrate", &format!("{}k", video.bitrate_kbps),
                    "-bufsize", &format!("{}k", video.buffer_size_kb),
                ]);
            }
            "vbr" => {
                cmd.args(&[
                    "-b:v", &format!("{}k", video.bitrate_kbps),
                    "-maxrate", &format!("{}k", video.max_bitrate_kbps),
                    "-bufsize", &format!("{}k", video.buffer_size_kb),
                ]);
            }
            _ => {
                cmd.args(&["-b:v", &format!("{}k", video.bitrate_kbps)]);
            }
        }
        
        // GOP settings
        cmd.args(&[
            "-g", &video.gop_size.to_string(),
            "-keyint_min", &(video.gop_size / 2).to_string(),
        ]);
        
        // Preset y tuning (solo para libx264)
        if !video.hardware_acceleration.enabled {
            cmd.args(&[
                "-preset", &video.preset,
                "-tune", &video.tune,
            ]);
        }
        
        // Framerate
        cmd.args(&["-r", &video.framerate.to_string()]);
    }
    
    fn add_audio_encoding_args(&self, cmd: &mut Command) {
        let audio = &self.config.audio;
        
        cmd.args(&[
            "-c:a", &audio.codec,
            "-b:a", &format!("{}k", audio.bitrate_kbps),
            "-ar", &audio.sample_rate.to_string(),
            "-ac", &audio.channels.to_string(),
        ]);
    }
    
    fn add_output_args(&self, cmd: &mut Command) {
        let output = &self.config.output;
        
        // Construir URL de salida con parámetros
        let mut output_url = output.url.clone();
        
        if let Some(local_if) = &output.local_interface {
            if output_url.contains('?') {
                output_url.push_str(&format!("&localaddr={}", local_if));
            } else {
                output_url.push_str(&format!("?localaddr={}", local_if));
            }
        }
        
        if let Some(ttl) = output.ttl {
            if output_url.contains('?') {
                output_url.push_str(&format!("&ttl={}", ttl));
            } else {
                output_url.push_str(&format!("?ttl={}", ttl));
            }
        }
        
        if let Some(pkt_size) = output.packet_size {
            if output_url.contains('?') {
                output_url.push_str(&format!("&pkt_size={}", pkt_size));
            } else {
                output_url.push_str(&format!("?pkt_size={}", pkt_size));
            }
        }
        
        cmd.arg(&output_url);
    }
    
    async fn parse_ffmpeg_output(
        stderr: tokio::process::ChildStderr,
        stats: Arc<RwLock<ChannelStats>>,
        channel_id: String,
    ) {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        
        while let Ok(Some(line)) = lines.next_line().await {
            debug!("FFmpeg [{}]: {}", channel_id, line);
            
            // Parsear estadísticas
            if line.contains("frame=") && line.contains("fps=") {
                if let Some((frames, fps, bitrate, speed)) = parse_ffmpeg_stats(&line) {
                    let mut stats = stats.write().await;
                    stats.frames_processed = frames;
                    stats.current_fps = fps;
                    stats.output_bitrate_kbps = bitrate;
                    stats.speed = speed;
                }
            } else if line.to_lowercase().contains("error") {
                warn!("FFmpeg error [{}]: {}", channel_id, line);
            }
        }
    }
}

fn parse_ffmpeg_stats(line: &str) -> Option<(u64, f64, f64, f64)> {
    let mut frames = 0u64;
    let mut fps = 0.0f64;
    let mut bitrate = 0.0f64;
    let mut speed = 0.0f64;
    
    for part in line.split_whitespace() {
        if part.starts_with("frame=") {
            if let Ok(val) = part.trim_start_matches("frame=").parse::<u64>() {
                frames = val;
            }
        } else if part.starts_with("fps=") {
            if let Ok(val) = part.trim_start_matches("fps=").parse::<f64>() {
                fps = val;
            }
        } else if part.starts_with("bitrate=") {
            let bitrate_str = part
                .trim_start_matches("bitrate=")
                .trim_end_matches("kbits/s")
                .trim_end_matches("Mbits/s");
            if let Ok(val) = bitrate_str.parse::<f64>() {
                bitrate = if part.contains("Mbits/s") {
                    val * 1000.0
                } else {
                    val
                };
            }
        } else if part.starts_with("speed=") {
            let speed_str = part.trim_start_matches("speed=").trim_end_matches('x');
            if let Ok(val) = speed_str.parse::<f64>() {
                speed = val;
            }
        }
    }
    
    if frames > 0 || fps > 0.0 {
        Some((frames, fps, bitrate, speed))
    } else {
        None
    }
}