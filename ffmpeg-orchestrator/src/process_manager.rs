use crate::models::{Channel, ChannelStatus};
use crate::storage::Storage;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

pub struct ProcessManager {
    processes: Arc<RwLock<HashMap<Uuid, Child>>>,
    storage: Arc<Storage>,
}

impl ProcessManager {
    pub fn new(storage: Arc<Storage>) -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            storage,
        }
    }

    pub async fn start_channel(&self, channel_id: Uuid) -> Result<()> {
        let channel = self
            .storage
            .get_channel(channel_id)
            .await
            .context("Channel not found")?;

        // Verificar si ya está corriendo
        {
            let processes = self.processes.read().await;
            if processes.contains_key(&channel_id) {
                anyhow::bail!("Channel is already running");
            }
        }

        // Actualizar estado a Starting
        self.storage
            .update_channel(channel_id, |c| {
                c.status = ChannelStatus::Starting;
                c.error_message = None;
            })
            .await?;

        // Crear directorio de logs si no existe
        if let Some(parent) = std::path::Path::new(&channel.get_log_file()).parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Iniciar el proceso FFmpeg con auto-reconexión
        let process_manager = self.clone();
        let channel_clone = channel.clone();

        tokio::spawn(async move {
            if let Err(e) = process_manager
                .run_ffmpeg_with_retry(channel_clone)
                .await
            {
                tracing::error!("Failed to run FFmpeg for channel {}: {:?}", channel_id, e);
                
                let _ = process_manager
                    .storage
                    .update_channel(channel_id, |c| {
                        c.status = ChannelStatus::Error;
                        c.error_message = Some(e.to_string());
                    })
                    .await;
            }
        });

        Ok(())
    }

    async fn run_ffmpeg_with_retry(&self, channel: Channel) -> Result<()> {
        let mut retry_count = 0;
        
        loop {
            retry_count += 1;
            
            tracing::info!(
                "Starting FFmpeg for channel: {} (attempt #{})",
                channel.name,
                retry_count
            );

            match self.spawn_ffmpeg(&channel).await {
                Ok(child) => {
                    let pid = child.id();
                    
                    // Guardar el proceso
                    {
                        let mut processes = self.processes.write().await;
                        processes.insert(channel.id, child);
                    }

                    // Actualizar estado a Running
                    self.storage
                        .update_channel(channel.id, |c| {
                            c.status = ChannelStatus::Running;
                            c.pid = pid;
                            c.error_message = None;
                        })
                        .await?;

                    // Esperar a que el proceso termine
                    let mut child = {
                        let mut processes = self.processes.write().await;
                        processes.remove(&channel.id).unwrap()
                    };

                    let exit_status = child.wait().await?;
                    
                    #[cfg(unix)]
                    let signal_info = exit_status.signal().map(|s| format!(", Signal: {:?}", s)).unwrap_or_default();
                    #[cfg(not(unix))]
                    let signal_info = String::new();
                    
                    let exit_msg = format!(
                        "[{}] FFmpeg process terminated - Exit code: {:?}{}",
                        chrono::Utc::now(),
                        exit_status.code(),
                        signal_info
                    );
                    
                    // Log del exit status
                    self.log_to_file(&channel.get_log_file(), &exit_msg).await?;
                    
                    tracing::warn!(
                        "FFmpeg stopped for channel {} - Exit code: {:?}{}, Retry count: {}",
                        channel.name,
                        exit_status.code(),
                        signal_info,
                        retry_count
                    );

                    // Verificar si el canal debe seguir corriendo
                    if let Some(current_channel) = self.storage.get_channel(channel.id).await {
                        if current_channel.status != ChannelStatus::Running
                            && current_channel.status != ChannelStatus::Reconnecting
                        {
                            tracing::info!("Channel {} was stopped manually, not reconnecting", channel.name);
                            break;
                        }
                    } else {
                        tracing::info!("Channel {} was deleted, stopping", channel.name);
                        break;
                    }

                    // Determinar el mensaje de error basado en el exit code
                    let error_msg = match exit_status.code() {
                        Some(0) => "Process exited normally (code 0)".to_string(),
                        Some(1) => "General error (code 1) - Check input URL and network".to_string(),
                        Some(255) | Some(-1) => "Connection error or killed (code 255) - Check SRT/network connection".to_string(),
                        Some(code) => format!("Process exited with code {}", code),
                        None => {
                            #[cfg(unix)]
                            {
                                match exit_status.signal() {
                                    Some(signal) => format!("Process killed by signal {}", signal),
                                    None => "Process terminated with unknown status".to_string(),
                                }
                            }
                            #[cfg(not(unix))]
                            {
                                "Process terminated with unknown status".to_string()
                            }
                        }
                    };

                    // Actualizar estado a Reconnecting
                    self.storage
                        .update_channel(channel.id, |c| {
                            c.status = ChannelStatus::Reconnecting;
                            c.pid = None;
                            c.error_message = Some(error_msg.clone());
                        })
                        .await?;

                    // Log de reconexión
                    let reconnect_msg = format!(
                        "[{}] FFmpeg stopped: {}. Reconnecting in 5s... (attempt #{})",
                        chrono::Utc::now(),
                        error_msg,
                        retry_count + 1
                    );
                    
                    self.log_to_file(&channel.get_log_file(), &reconnect_msg).await?;

                    sleep(Duration::from_secs(5)).await;
                }
                Err(e) => {
                    tracing::error!("Failed to spawn FFmpeg for channel {}: {:?}", channel.name, e);
                    
                    let error_msg = format!("Failed to spawn FFmpeg: {}", e);
                    
                    self.storage
                        .update_channel(channel.id, |c| {
                            c.status = ChannelStatus::Error;
                            c.error_message = Some(error_msg.clone());
                        })
                        .await?;

                    self.log_to_file(
                        &channel.get_log_file(),
                        &format!("[{}] Spawn error: {}. Retrying in 10s...", chrono::Utc::now(), error_msg),
                    ).await?;

                    sleep(Duration::from_secs(10)).await;
                }
            }
        }

        Ok(())
    }

    async fn spawn_ffmpeg(&self, channel: &Channel) -> Result<Child> {
        let input_url = channel.get_input_url();
        let output_url = channel.get_output_url();
        let log_file = channel.get_log_file();

        // Log de inicio con información de conexión
        let start_msg = format!(
            "[{}] Starting FFmpeg for {}\n  Input: {}\n  Output: {}",
            chrono::Utc::now(),
            channel.name,
            input_url,
            output_url
        );
        
        self.log_to_file(&log_file, &start_msg).await?;
        
        tracing::info!(
            "Starting FFmpeg - Channel: {}, Input: {}, Output: {}",
            channel.name,
            input_url,
            output_url
        );

        #[cfg(unix)]
        let mut child = {
            Command::new("ffmpeg")
                .arg("-loglevel")
                .arg("info")
                .arg("-stats")
                .arg("-i")
                .arg(&input_url)
                .arg("-c")
                .arg("copy")
                .arg("-f")
                .arg("mpegts")
                .arg(&output_url)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .context("Failed to spawn FFmpeg process")?
        };
        
        #[cfg(not(unix))]
        let mut child = {
            Command::new("ffmpeg")
                .arg("-loglevel")
                .arg("info")
                .arg("-stats")
                .arg("-i")
                .arg(&input_url)
                .arg("-c")
                .arg("copy")
                .arg("-f")
                .arg("mpegts")
                .arg(&output_url)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .context("Failed to spawn FFmpeg process")?
        };

        tracing::info!(
            "Spawned FFmpeg for channel {} (PID: {:?})",
            channel.name,
            child.id()
        );

        // Capturar stderr en tiempo real
        if let Some(stderr) = child.stderr.take() {
            let log_file_clone = log_file.clone();
            let channel_name = channel.name.clone();
            
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                
                while let Ok(Some(line)) = lines.next_line().await {
                    // Loggear a archivo
                    let log_msg = format!("[{}] FFmpeg: {}", chrono::Utc::now(), line);
                    
                    if let Ok(mut file) = tokio::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_file_clone)
                        .await
                    {
                        let _ = file.write_all(format!("{}\n", log_msg).as_bytes()).await;
                    }
                    
                    // También loggear errores críticos a tracing
                    if line.contains("error") || line.contains("Error") || line.contains("ERROR") {
                        tracing::error!("FFmpeg error in channel {}: {}", channel_name, line);
                    } else if line.contains("warning") || line.contains("Warning") {
                        tracing::warn!("FFmpeg warning in channel {}: {}", channel_name, line);
                    }
                }
            });
        }

        // Capturar stdout también (stats)
        if let Some(stdout) = child.stdout.take() {
            let log_file_clone = log_file.clone();
            
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.trim().is_empty() {
                        let log_msg = format!("[{}] Stats: {}", chrono::Utc::now(), line);
                        
                        if let Ok(mut file) = tokio::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&log_file_clone)
                            .await
                        {
                            let _ = file.write_all(format!("{}\n", log_msg).as_bytes()).await;
                        }
                    }
                }
            });
        }

        Ok(child)
    }

    pub async fn stop_channel(&self, channel_id: Uuid) -> Result<()> {
        // Actualizar estado primero para prevenir reconexión
        self.storage
            .update_channel(channel_id, |c| {
                c.status = ChannelStatus::Stopped;
                c.pid = None;
            })
            .await?;

        // Matar el proceso si existe
        let mut processes = self.processes.write().await;
        if let Some(mut child) = processes.remove(&channel_id) {
            // Intentar terminación graceful
            let _ = child.kill().await;
            
            tracing::info!("Stopped channel: {:?}", channel_id);
        }

        Ok(())
    }

    pub async fn restart_channel(&self, channel_id: Uuid) -> Result<()> {
        self.stop_channel(channel_id).await?;
        sleep(Duration::from_secs(1)).await;
        self.start_channel(channel_id).await?;
        Ok(())
    }

    pub async fn is_running(&self, channel_id: Uuid) -> bool {
        let processes = self.processes.read().await;
        processes.contains_key(&channel_id)
    }

    async fn log_to_file(&self, path: &str, message: &str) -> Result<()> {
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        file.write_all(format!("{}\n", message).as_bytes()).await?;
        Ok(())
    }
}

impl Clone for ProcessManager {
    fn clone(&self) -> Self {
        Self {
            processes: Arc::clone(&self.processes),
            storage: Arc::clone(&self.storage),
        }
    }
}