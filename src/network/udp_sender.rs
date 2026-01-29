use crate::models::error::TranscoderError;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{info, debug, error};

/// Sender UDP para transmisión de MPEG-TS
pub struct UdpSender {
    socket: UdpSocket,
    destination: SocketAddr,
    local_interface: Option<String>,
    ttl: u32,
    packet_size: usize,
    packets_sent: u64,
    bytes_sent: u64,
}

impl UdpSender {
    /// Crear nuevo UdpSender
    pub async fn new(destination: SocketAddr) -> Result<Self, TranscoderError> {
        // Bind a cualquier puerto local
        let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let socket = UdpSocket::bind(local_addr).await
            .map_err(|e| TranscoderError::UdpError(format!("Error creando socket UDP: {}", e)))?;
        
        info!("UDP socket creado, destino: {}", destination);
        
        Ok(Self {
            socket,
            destination,
            local_interface: None,
            ttl: 32,
            packet_size: 1316, // 7 paquetes MPEG-TS (188 * 7)
            packets_sent: 0,
            bytes_sent: 0,
        })
    }
    
    /// Crear desde URL (ej: udp://239.0.0.1:5000?localaddr=192.168.1.10&ttl=16)
    pub async fn from_url(url: &str) -> Result<Self, TranscoderError> {
        let parsed = parse_udp_url(url)?;
        
        let mut sender = Self::new(parsed.destination).await?;
        
        if let Some(local_if) = parsed.local_interface {
            sender = sender.with_local_interface(local_if)?;
        }
        
        if let Some(ttl) = parsed.ttl {
            sender.set_ttl(ttl)?;
        }
        
        if let Some(pkt_size) = parsed.packet_size {
            sender.packet_size = pkt_size;
        }
        
        Ok(sender)
    }
    
    /// Configurar interfaz local
    pub fn with_local_interface(mut self, interface: String) -> Result<Self, TranscoderError> {
        // En Linux, usar SO_BINDTODEVICE
        // En producción, necesitarías usar socket2 crate para esto
        info!("Configurando interfaz local: {}", interface);
        self.local_interface = Some(interface);
        Ok(self)
    }
    
    /// Configurar TTL
    pub fn set_ttl(&mut self, ttl: u32) -> Result<(), TranscoderError> {
        self.socket.set_ttl(ttl)
            .map_err(|e| TranscoderError::UdpError(format!("Error configurando TTL: {}", e)))?;
        
        self.ttl = ttl;
        info!("TTL configurado a {}", ttl);
        Ok(())
    }
    
    /// Habilitar broadcast
    pub fn enable_broadcast(&self) -> Result<(), TranscoderError> {
        self.socket.set_broadcast(true)
            .map_err(|e| TranscoderError::UdpError(format!("Error habilitando broadcast: {}", e)))?;
        
        info!("Broadcast habilitado");
        Ok(())
    }
    
    /// Enviar datos
    pub async fn send(&mut self, data: &[u8]) -> Result<usize, TranscoderError> {
        let bytes_sent = self.socket.send_to(data, self.destination).await
            .map_err(|e| TranscoderError::UdpError(format!("Error enviando datos: {}", e)))?;
        
        self.packets_sent += 1;
        self.bytes_sent += bytes_sent as u64;
        
        debug!("Enviados {} bytes por UDP", bytes_sent);
        Ok(bytes_sent)
    }
    
    /// Enviar múltiples paquetes
    pub async fn send_batch(&mut self, packets: Vec<Vec<u8>>) -> Result<usize, TranscoderError> {
        let mut total_sent = 0;
        
        for packet in packets {
            total_sent += self.send(&packet).await?;
        }
        
        Ok(total_sent)
    }
    
    /// Obtener estadísticas
    pub fn stats(&self) -> UdpStats {
        UdpStats {
            packets_sent: self.packets_sent,
            bytes_sent: self.bytes_sent,
            destination: self.destination,
            ttl: self.ttl,
        }
    }
    
    /// Resetear estadísticas
    pub fn reset_stats(&mut self) {
        self.packets_sent = 0;
        self.bytes_sent = 0;
    }
}

/// Estadísticas de envío UDP
#[derive(Debug, Clone)]
pub struct UdpStats {
    pub packets_sent: u64,
    pub bytes_sent: u64,
    pub destination: SocketAddr,
    pub ttl: u32,
}

impl UdpStats {
    pub fn bitrate_kbps(&self, duration_secs: f64) -> f64 {
        if duration_secs > 0.0 {
            (self.bytes_sent as f64 * 8.0) / (duration_secs * 1000.0)
        } else {
            0.0
        }
    }
}

// ============================================================================
// PARSEO DE URL UDP
// ============================================================================

struct ParsedUdpUrl {
    destination: SocketAddr,
    local_interface: Option<String>,
    ttl: Option<u32>,
    packet_size: Option<usize>,
}

fn parse_udp_url(url: &str) -> Result<ParsedUdpUrl, TranscoderError> {
    // Formato: udp://239.0.0.1:5000?localaddr=192.168.1.10&ttl=16&pkt_size=1316
    
    let url_without_scheme = url.strip_prefix("udp://")
        .ok_or_else(|| TranscoderError::UdpError("URL debe comenzar con udp://".to_string()))?;
    
    // Separar dirección de parámetros
    let parts: Vec<&str> = url_without_scheme.split('?').collect();
    let addr_str = parts[0];
    
    let destination: SocketAddr = addr_str.parse()
        .map_err(|e| TranscoderError::UdpError(format!("Dirección inválida: {}", e)))?;
    
    let mut local_interface = None;
    let mut ttl = None;
    let mut packet_size = None;
    
    // Parsear parámetros
    if parts.len() > 1 {
        for param in parts[1].split('&') {
            let kv: Vec<&str> = param.split('=').collect();
            if kv.len() == 2 {
                match kv[0] {
                    "localaddr" => local_interface = Some(kv[1].to_string()),
                    "ttl" => ttl = kv[1].parse().ok(),
                    "pkt_size" => packet_size = kv[1].parse().ok(),
                    _ => {}
                }
            }
        }
    }
    
    Ok(ParsedUdpUrl {
        destination,
        local_interface,
        ttl,
        packet_size,
    })
}

/// Builder para UdpSender
pub struct UdpSenderBuilder {
    destination: SocketAddr,
    local_interface: Option<String>,
    ttl: u32,
    packet_size: usize,
    broadcast: bool,
}

impl UdpSenderBuilder {
    pub fn new(destination: SocketAddr) -> Self {
        Self {
            destination,
            local_interface: None,
            ttl: 32,
            packet_size: 1316,
            broadcast: false,
        }
    }
    
    pub fn local_interface(mut self, interface: String) -> Self {
        self.local_interface = Some(interface);
        self
    }
    
    pub fn ttl(mut self, ttl: u32) -> Self {
        self.ttl = ttl;
        self
    }
    
    pub fn packet_size(mut self, size: usize) -> Self {
        self.packet_size = size;
        self
    }
    
    pub fn broadcast(mut self, enabled: bool) -> Self {
        self.broadcast = enabled;
        self
    }
    
    pub async fn build(self) -> Result<UdpSender, TranscoderError> {
        let mut sender = UdpSender::new(self.destination).await?;
        
        if let Some(interface) = self.local_interface {
            sender = sender.with_local_interface(interface)?;
        }
        
        sender.set_ttl(self.ttl)?;
        sender.packet_size = self.packet_size;
        
        if self.broadcast {
            sender.enable_broadcast()?;
        }
        
        Ok(sender)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_udp_url() {
        let parsed = parse_udp_url("udp://239.0.0.1:5000?ttl=16&pkt_size=1316").unwrap();
        assert_eq!(parsed.destination.port(), 5000);
        assert_eq!(parsed.ttl, Some(16));
        assert_eq!(parsed.packet_size, Some(1316));
    }
    
    #[tokio::test]
    async fn test_udp_sender_creation() {
        let addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        let sender = UdpSender::new(addr).await.unwrap();
        assert_eq!(sender.destination, addr);
    }
}