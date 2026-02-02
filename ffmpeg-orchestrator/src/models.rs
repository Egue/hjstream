use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InputFormat {
    SRT,
    RTMP,
    HLS,
    HTTP,
    RTSP,
}

impl fmt::Display for InputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputFormat::SRT => write!(f, "srt"),
            InputFormat::RTMP => write!(f, "rtmp"),
            InputFormat::HLS => write!(f, "hls"),
            InputFormat::HTTP => write!(f, "http"),
            InputFormat::RTSP => write!(f, "rtsp"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub format: InputFormat,
    pub url: String,
    
    // Opciones específicas para SRT
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>, // caller, listener
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpOutputConfig {
    pub multicast_ip: String,
    pub port: u16,
    pub local_addr: String,
    
    #[serde(default = "default_pkt_size")]
    pub pkt_size: u32,
    
    #[serde(default = "default_ttl")]
    pub ttl: u8,
}

fn default_pkt_size() -> u32 {
    1316
}

fn default_ttl() -> u8 {
    64
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateChannelRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    
    pub input: InputConfig,
    pub output: UdpOutputConfig,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChannelStatus {
    Starting,
    Running,
    Stopped,
    Error,
    Reconnecting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Uuid,
    pub name: String,
    pub input: InputConfig,
    pub output: UdpOutputConfig,
    pub description: Option<String>,
    pub status: ChannelStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
}

impl Channel {
    pub fn new(request: CreateChannelRequest) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: request.name,
            input: request.input,
            output: request.output,
            description: request.description,
            status: ChannelStatus::Stopped,
            created_at: now,
            updated_at: now,
            error_message: None,
            pid: None,
        }
    }

    pub fn get_log_file(&self) -> String {
        format!("/var/log/hjstream/{}.log", self.name)
    }

    pub fn get_input_url(&self) -> String {
        match self.input.format {
            InputFormat::SRT => {
                let mut url = self.input.url.clone();
                if !url.starts_with("srt://") {
                    url = format!("srt://{}", url);
                }
                
                let has_params = url.contains('?');
                let mut params = vec![];
                
                if let Some(mode) = &self.input.mode {
                    params.push(format!("mode={}", mode));
                }
                if let Some(latency) = self.input.latency {
                    params.push(format!("latency={}", latency));
                }
                
                if !params.is_empty() {
                    if has_params {
                        format!("{}&{}", url, params.join("&"))
                    } else {
                        format!("{}?{}", url, params.join("&"))
                    }
                } else {
                    url
                }
            }
            _ => self.input.url.clone(),
        }
    }

    pub fn get_output_url(&self) -> String {
        format!(
            "udp://{}:{}?pkt_size={}&localaddr={}&ttl={}",
            self.output.multicast_ip,
            self.output.port,
            self.output.pkt_size,
            self.output.local_addr,
            self.output.ttl
        )
    }
}

#[derive(Debug, Serialize)]
pub struct ChannelResponse {
    pub channel: Channel,
}

#[derive(Debug, Serialize)]
pub struct ChannelListResponse {
    pub channels: Vec<Channel>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
