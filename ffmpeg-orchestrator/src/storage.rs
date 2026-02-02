use crate::models::Channel;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::RwLock;
use std::sync::Arc;
use uuid::Uuid;

const CHANNELS_FILE: &str = "/etc/hjstream/channels.json";

pub struct Storage {
    channels: Arc<RwLock<Vec<Channel>>>,
    file_path: PathBuf,
}

impl Storage {
    pub async fn new(custom_path: Option<PathBuf>) -> Result<Self> {
        let file_path = custom_path.unwrap_or_else(|| PathBuf::from(CHANNELS_FILE));
        
        // Crear el directorio si no existe
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create storage directory")?;
        }

        let channels = if file_path.exists() {
            Self::load_from_file(&file_path).await?
        } else {
            Vec::new()
        };

        Ok(Self {
            channels: Arc::new(RwLock::new(channels)),
            file_path,
        })
    }

    async fn load_from_file(path: &PathBuf) -> Result<Vec<Channel>> {
        let content = fs::read_to_string(path)
            .await
            .context("Failed to read channels file")?;
        
        let channels: Vec<Channel> = serde_json::from_str(&content)
            .context("Failed to parse channels file")?;
        
        tracing::info!("Loaded {} channels from storage", channels.len());
        Ok(channels)
    }

    async fn save_to_file(&self) -> Result<()> {
        let channels = self.channels.read().await;
        let json = serde_json::to_string_pretty(&*channels)
            .context("Failed to serialize channels")?;
        
        fs::write(&self.file_path, json)
            .await
            .context("Failed to write channels file")?;
        
        tracing::debug!("Saved {} channels to storage", channels.len());
        Ok(())
    }

    pub async fn add_channel(&self, channel: Channel) -> Result<()> {
        let mut channels = self.channels.write().await;
        channels.push(channel);
        drop(channels);
        
        self.save_to_file().await?;
        Ok(())
    }

    pub async fn get_channel(&self, id: Uuid) -> Option<Channel> {
        let channels = self.channels.read().await;
        channels.iter().find(|c| c.id == id).cloned()
    }

    pub async fn get_all_channels(&self) -> Vec<Channel> {
        let channels = self.channels.read().await;
        channels.clone()
    }

    pub async fn update_channel(&self, id: Uuid, updater: impl FnOnce(&mut Channel)) -> Result<()> {
        let mut channels = self.channels.write().await;
        
        if let Some(channel) = channels.iter_mut().find(|c| c.id == id) {
            updater(channel);
            channel.updated_at = chrono::Utc::now();
        } else {
            anyhow::bail!("Channel not found");
        }
        
        drop(channels);
        self.save_to_file().await?;
        Ok(())
    }

    pub async fn delete_channel(&self, id: Uuid) -> Result<()> {
        let mut channels = self.channels.write().await;
        
        let initial_len = channels.len();
        channels.retain(|c| c.id != id);
        
        if channels.len() == initial_len {
            anyhow::bail!("Channel not found");
        }
        
        drop(channels);
        self.save_to_file().await?;
        Ok(())
    }

    pub async fn get_channel_by_name(&self, name: &str) -> Option<Channel> {
        let channels = self.channels.read().await;
        channels.iter().find(|c| c.name == name).cloned()
    }
}
