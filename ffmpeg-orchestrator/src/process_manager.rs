use crate::models::{Channel, ChannelStatus};
use crate::storage::Storage;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

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
        loop {
            tracing::info!("Starting FFmpeg for channel: {}", channel.name);

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

                    let status = child.wait().await?;
                    
                    tracing::warn!(
                        "FFmpeg stopped for channel {}: {:?}",
                        channel.name,
                        status
                    );

                    // Verificar si el canal debe seguir corriendo
                    if let Some(current_channel) = self.storage.get_channel(channel.id).await {
                        if current_channel.status != ChannelStatus::Running
                            && current_channel.status != ChannelStatus::Reconnecting
                        {
                            tracing::info!("Channel {} was stopped, not reconnecting", channel.name);
                            break;
                        }
                    } else {
                        tracing::info!("Channel {} was deleted, stopping", channel.name);
                        break;
                    }

                    // Actualizar estado a Reconnecting
                    self.storage
                        .update_channel(channel.id, |c| {
                            c.status = ChannelStatus::Reconnecting;
                            c.pid = None;
                        })
                        .await?;

                    // Log de reconexión
                    self.log_to_file(
                        &channel.get_log_file(),
                        &format!("[{}] FFmpeg stopped. Reconnecting in 5s...", chrono::Utc::now()),
                    )
                    .await?;

                    sleep(Duration::from_secs(5)).await;
                }
                Err(e) => {
                    tracing::error!("Failed to spawn FFmpeg for channel {}: {:?}", channel.name, e);
                    
                    self.storage
                        .update_channel(channel.id, |c| {
                            c.status = ChannelStatus::Error;
                            c.error_message = Some(e.to_string());
                        })
                        .await?;

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

        // Log de inicio
        self.log_to_file(
            &log_file,
            &format!("[{}] Starting FFmpeg for {}", chrono::Utc::now(), channel.name),
        )
        .await?;

        let child = Command::new("ffmpeg")
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
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn FFmpeg process")?;

        tracing::info!(
            "Spawned FFmpeg for channel {} (PID: {:?})",
            channel.name,
            child.id()
        );

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

    /*pub async fn is_running(&self, channel_id: Uuid) -> bool {
        let processes = self.processes.read().await;
        processes.contains_key(&channel_id)
    }*/

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
