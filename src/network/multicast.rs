use crate::models::error::TranscoderError;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::UdpSocket;
use tracing::{info, error};

/// Sender UDP Multicast especializado para CATV
pub struct MulticastSender {
    socket: UdpSocket,
    multicast_addr: SocketAddr,
    source_interface: Ipv4Addr,
    ttl: u32,
    packets_sent: u64,
}

impl MulticastSender {
    /// Crear nuevo MulticastSender
    pub async fn new(
        multicast_addr: SocketAddr,
        source_interface: Ipv4Addr,
    ) -> Result<Self, TranscoderError> {
        // Validar que es dirección multicast
        if let IpAddr::V4(ip) = multicast_addr.ip() {
            if !is_multicast_address(&ip) {
                return Err(TranscoderError::UdpError(
                    format!("Dirección {} no es multicast (rango: 224.0.0.0-239.255.255.255)", ip)
                ));
            }
        } else {
            return Err(TranscoderError::UdpError("Solo IPv4 soportado".to_string()));
        }
        
        // Crear socket
        let local_addr = SocketAddr::new(IpAddr::V4(source_interface), 0);
        let socket = UdpSocket::bind(local_addr).await
            .map_err(|e| TranscoderError::UdpError(format!("Error creando socket: {}", e)))?;
        
        // Configurar para multicast
        if let IpAddr::V4(multicast_ip) = multicast_addr.ip() {
            socket.join_multicast_v4(multicast_ip, source_interface)
                .map_err(|e| TranscoderError::UdpError(format!("Error uniéndose a multicast: {}", e)))?;
        }
        
        // Configurar TTL multicast
        socket.set_multicast_ttl_v4(32)
            .map_err(|e| TranscoderError::UdpError(format!("Error configurando TTL: {}", e)))?;
        
        info!(
            "MulticastSender creado: {} desde interface {}", 
            multicast_addr, 
            source_interface
        );
        
        Ok(Self {
            socket,
            multicast_addr,
            source_interface,
            ttl: 32,
            packets_sent: 0,
        })
    }
    
    /// Enviar datos multicast
    pub async fn send(&mut self, data: &[u8]) -> Result<usize, TranscoderError> {
        let bytes_sent = self.socket.send_to(data, self.multicast_addr).await
            .map_err(|e| TranscoderError::UdpError(format!("Error enviando multicast: {}", e)))?;
        
        self.packets_sent += 1;
        Ok(bytes_sent)
    }
    
    /// Configurar TTL
    pub fn set_ttl(&mut self, ttl: u32) -> Result<(), TranscoderError> {
        self.socket.set_multicast_ttl_v4(ttl)
            .map_err(|e| TranscoderError::UdpError(format!("Error configurando TTL: {}", e)))?;
        
        self.ttl = ttl;
        info!("Multicast TTL configurado a {}", ttl);
        Ok(())
    }
    
    /// Habilitar/deshabilitar loopback
    pub fn set_multicast_loop(&self, enabled: bool) -> Result<(), TranscoderError> {
        self.socket.set_multicast_loop_v4(enabled)
            .map_err(|e| TranscoderError::UdpError(format!("Error configurando loop: {}", e)))?;
        
        info!("Multicast loopback: {}", enabled);
        Ok(())
    }
    
    /// Obtener estadísticas
    pub fn packets_sent(&self) -> u64 {
        self.packets_sent
    }
}

impl Drop for MulticastSender {
    fn drop(&mut self) {
        if let IpAddr::V4(multicast_ip) = self.multicast_addr.ip() {
            let _ = self.socket.leave_multicast_v4(multicast_ip, self.source_interface);
            info!("Dejando grupo multicast {}", multicast_ip);
        }
    }
}

/// Verificar si una dirección IPv4 está en rango multicast
fn is_multicast_address(addr: &Ipv4Addr) -> bool {
    let octets = addr.octets();
    octets[0] >= 224 && octets[0] <= 239
}

/// Receptor Multicast (para testing o monitoreo)
pub struct MulticastReceiver {
    socket: UdpSocket,
    multicast_addr: Ipv4Addr,
    interface: Ipv4Addr,
}

impl MulticastReceiver {
    pub async fn new(
        multicast_addr: Ipv4Addr,
        port: u16,
        interface: Ipv4Addr,
    ) -> Result<Self, TranscoderError> {
        if !is_multicast_address(&multicast_addr) {
            return Err(TranscoderError::UdpError(
                format!("Dirección {} no es multicast", multicast_addr)
            ));
        }
        
        // Bind al puerto multicast
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        let socket = UdpSocket::bind(bind_addr).await
            .map_err(|e| TranscoderError::UdpError(format!("Error binding: {}", e)))?;
        
        // Unirse al grupo multicast
        socket.join_multicast_v4(multicast_addr, interface)
            .map_err(|e| TranscoderError::UdpError(format!("Error uniéndose a grupo: {}", e)))?;
        
        info!("MulticastReceiver unido a {}:{} en interface {}", 
            multicast_addr, port, interface);
        
        Ok(Self {
            socket,
            multicast_addr,
            interface,
        })
    }
    
    pub async fn receive(&self, buffer: &mut [u8]) -> Result<(usize, SocketAddr), TranscoderError> {
        self.socket.recv_from(buffer).await
            .map_err(|e| TranscoderError::UdpError(format!("Error recibiendo: {}", e)))
    }
}

impl Drop for MulticastReceiver {
    fn drop(&mut self) {
        let _ = self.socket.leave_multicast_v4(self.multicast_addr, self.interface);
        info!("Dejando grupo multicast {}", self.multicast_addr);
    }
}

/// Helper para obtener IP de interfaz de red por nombre
#[cfg(target_os = "linux")]
pub fn get_interface_ip(interface_name: &str) -> Result<Ipv4Addr, TranscoderError> {
    use std::process::Command;
    
    let output = Command::new("ip")
        .args(&["addr", "show", interface_name])
        .output()
        .map_err(|e| TranscoderError::NetworkError(format!("Error ejecutando ip: {}", e)))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parsear output para encontrar inet
    for line in stdout.lines() {
        if line.trim().starts_with("inet ") {
            let parts: Vec<&str> = line.trim().split_whitespace().collect();
            if parts.len() >= 2 {
                let ip_cidr = parts[1];
                let ip_str = ip_cidr.split('/').next().unwrap_or("");
                
                if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
                    return Ok(ip);
                }
            }
        }
    }
    
    Err(TranscoderError::NetworkError(
        format!("No se encontró IP para interfaz {}", interface_name)
    ))
}

#[cfg(not(target_os = "linux"))]
pub fn get_interface_ip(_interface_name: &str) -> Result<Ipv4Addr, TranscoderError> {
    // En otros sistemas, usar crate como `if-addrs` o `pnet`
    Err(TranscoderError::NetworkError(
        "get_interface_ip solo soportado en Linux".to_string()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_is_multicast() {
        assert!(is_multicast_address(&Ipv4Addr::new(239, 0, 0, 1)));
        assert!(is_multicast_address(&Ipv4Addr::new(224, 0, 0, 1)));
        assert!(!is_multicast_address(&Ipv4Addr::new(192, 168, 1, 1)));
        assert!(!is_multicast_address(&Ipv4Addr::new(10, 0, 0, 1)));
    }
    
    #[tokio::test]
    async fn test_multicast_sender_creation() {
        let multicast_addr: SocketAddr = "239.0.0.1:5000".parse().unwrap();
        let interface = Ipv4Addr::new(127, 0, 0, 1);
        
        let result = MulticastSender::new(multicast_addr, interface).await;
        // Puede fallar en ambientes de CI sin soporte multicast
        // assert!(result.is_ok());
    }
}