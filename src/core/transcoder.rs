use crate::config::loader::ChannelConfig;
use crate::models::stats::ChannelStats;
use crate::models::error::TranscoderError;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{warn, error, debug};

/// Motor de transcodificación que ejecuta FFmpeg
pub struct Transcoder {
    config: ChannelConfig,
    process: Arc<RwLock<Option<Child>>>,
    restart_count: Arc<RwLock<u32>>,
}

impl Clone for Transcoder {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            process: Arc::clone(&self.process),
            restart_count: Arc::clone(&self.restart_count),
        }
    }
}

impl Transcoder {
    pub fn new(config: ChannelConfig) -> Result<Self, TranscoderError> {
        Ok(Self {
            config,
            process: Arc::new(RwLock::new(None)),
            restart_count: Arc::new(RwLock::new(0)),
        })
    }
    
    pub async fn run(&mut self, stats: Arc<RwLock<ChannelStats>>) -> Result<(), TranscoderError> {
        loop {
            if let Err(e) = self.run_once(stats.clone()).await {
                error!("Error en FFmpeg (canal {}): {}", self.config.id, e);
                
                let mut count = self.restart_count.write().await;
                *count += 1;
                
                if *count > 10 {
                    error!("Canal {} alcanzó máximo número de reintentos", self.config.id);
                    return Err(e);
                }
                
                drop(count);
                
                let wait_time = Duration::from_secs((*self.restart_count.read().await as u64).min(30));
                sleep(wait_time).await;
            } else {
                break;
            }
        }
        Ok(())
    }
    
    async fn run_once(&mut self, stats: Arc<RwLock<ChannelStats>>) -> Result<(), TranscoderError> {
        let mut cmd = self.build_ffmpeg_command();
        
        let mut child = cmd.spawn()
            .map_err(|e| TranscoderError::ProcessSpawnFailed(e.to_string()))?;
        
        if let Some(stderr) = child.stderr.take() {
            let channel_id = self.config.id.clone();
            tokio::spawn(async move {
                Self::parse_ffmpeg_output(stderr, stats, channel_id).await;
            });
        }
        
        {
            let mut process = self.process.write().await;
            *process = Some(child);
        }
        
        let mut process = self.process.write().await;
        if let Some(child) = process.as_mut() {
            child.wait().await
                .map_err(|e| TranscoderError::ProcessSpawnFailed(e.to_string()))?;
        }
        
        Ok(())
    }
    
    pub async fn stop(&mut self) -> Result<(), TranscoderError> {
        let mut process = self.process.write().await;
        
        if let Some(child) = process.as_mut() {
            let _ = child.kill().await;
        }
        
        *process = None;
        Ok(())
    }
    
    fn build_ffmpeg_command(&self) -> Command {
        let mut cmd = Command::new("/usr/bin/ffmpeg");
        
        cmd.args(&[
            "-loglevel", "info",
            "-stats",
            "-i", &self.config.input.url,
            "-c", "copy",
            "-f", "mpegts",
        ]);
        
        self.add_output_args(&mut cmd);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        cmd
    }
    
    fn add_output_args(&self, cmd: &mut Command) {
        let output = &self.config.output;
        let mut output_url = output.url.clone();
        
        if let Some(local_if) = &output.local_interface {
            output_url.push_str(&format!("?localaddr={}", local_if));
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