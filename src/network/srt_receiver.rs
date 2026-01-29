use crate::models::error::TranscoderError;
use bytes::BytesMut;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{info, warn, error, debug};

/// Receptor SRT (Secure Reliable Transport)
/// Nota: Esta es una implementación simplificada.
/// Para producción, considerar usar el crate `srt-tokio` o `srt-rs`
pub struct SrtReceiver {
    url: String,
    mode: SrtMode,
    socket: Option<UdpSocket>,
    buffer_size: usize,
    latency_ms: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum SrtMode {
    Listener,
    Caller,
    Rendezvous,
}

impl SrtReceiver {
    pub fn new(url: String) -> Self {
        // Parsear URL para determinar modo
        let mode = if url.contains("mode=listener") {
            SrtMode::Listener
        } else if url.contains("mode=caller") {
            SrtMode::Caller
        } else {
            SrtMode::Rendezvous
        };
        
        Self {
            url,
            mode,
            socket: None,
            buffer_size: 1316 * 7, // 7 paquetes MPEG-TS típicos
            latency_ms: 120, // Latencia por defecto
        }
    }
    
    pub fn with_latency(mut self, latency_ms: u32) -> Self {
        self.latency_ms = latency_ms;
        self
    }
    
    pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }
    
    /// Conectar al stream SRT
    pub async fn connect(&mut self) -> Result<(), TranscoderError> {
        info!("Conectando a SRT: {} (modo: {:?})", self.url, self.mode);
        
        // Parsear dirección del URL
        let addr = self.parse_srt_url()?;
        
        match self.mode {
            SrtMode::Listener => self.connect_listener(addr).await?,
            SrtMode::Caller => self.connect_caller(addr).await?,
            SrtMode::Rendezvous => self.connect_rendezvous(addr).await?,
        }
        
        info!("Conectado exitosamente a SRT");
        Ok(())
    }
    
    /// Recibir datos del stream
    pub async fn receive(&mut self) -> Result<Vec<u8>, TranscoderError> {
        let socket = self.socket.as_ref()
            .ok_or_else(|| TranscoderError::SrtError("Socket no conectado".to_string()))?;
        
        let mut buffer = vec![0u8; self.buffer_size];
        
        let len = socket.recv(&mut buffer).await
            .map_err(|e| TranscoderError::SrtError(format!("Error recibiendo datos: {}", e)))?;
        
        buffer.truncate(len);
        debug!("Recibidos {} bytes por SRT", len);
        
        Ok(buffer)
    }
    
    /// Recibir datos con timeout
    pub async fn receive_timeout(&mut self, timeout: std::time::Duration) -> Result<Vec<u8>, TranscoderError> {
        tokio::time::timeout(timeout, self.receive())
            .await
            .map_err(|_| TranscoderError::Timeout("Timeout recibiendo datos SRT".to_string()))?
    }
    
    /// Cerrar conexión
    pub async fn close(&mut self) -> Result<(), TranscoderError> {
        if self.socket.take().is_some() {
            info!("Conexión SRT cerrada");
        }
        Ok(())
    }
    
    // ========================================================================
    // MÉTODOS PRIVADOS - MODOS DE CONEXIÓN
    // ========================================================================
    
    async fn connect_listener(&mut self, addr: SocketAddr) -> Result<(), TranscoderError> {
        info!("Iniciando en modo Listener en {}", addr);
        
        let socket = UdpSocket::bind(addr).await
            .map_err(|e| TranscoderError::SrtError(format!("Error binding socket: {}", e)))?;
        
        self.socket = Some(socket);
        Ok(())
    }
    
    async fn connect_caller(&mut self, addr: SocketAddr) -> Result<(), TranscoderError> {
        info!("Conectando en modo Caller a {}", addr);
        
        // Bind a puerto local aleatorio
        let local_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let socket = UdpSocket::bind(local_addr).await
            .map_err(|e| TranscoderError::SrtError(format!("Error binding socket: {}", e)))?;
        
        socket.connect(addr).await
            .map_err(|e| TranscoderError::SrtError(format!("Error conectando: {}", e)))?;
        
        self.socket = Some(socket);
        Ok(())
    }
    
    async fn connect_rendezvous(&mut self, addr: SocketAddr) -> Result<(), TranscoderError> {
        info!("Iniciando en modo Rendezvous");
        
        let socket = UdpSocket::bind(addr).await
            .map_err(|e| TranscoderError::SrtError(format!("Error binding socket: {}", e)))?;
        
        self.socket = Some(socket);
        Ok(())
    }
    
    // ========================================================================
    // PARSEO DE URL
    // ========================================================================
    
    fn parse_srt_url(&self) -> Result<SocketAddr, TranscoderError> {
        // Formato: srt://host:port?mode=listener&latency=120
        
        let url_without_scheme = self.url.strip_prefix("srt://")
            .ok_or_else(|| TranscoderError::SrtError("URL debe comenzar con srt://".to_string()))?;
        
        // Separar dirección de parámetros
        let parts: Vec<&str> = url_without_scheme.split('?').collect();
        let addr_str = parts[0];
        
        // Parsear parámetros si existen
        if parts.len() > 1 {
            self.parse_srt_parameters(parts[1]);
        }
        
        // Parsear dirección
        addr_str.parse()
            .map_err(|e| TranscoderError::SrtError(format!("Dirección inválida: {}", e)))
    }
    
    fn parse_srt_parameters(&self, params: &str) {
        for param in params.split('&') {
            let kv: Vec<&str> = param.split('=').collect();
            if kv.len() == 2 {
                match kv[0] {
                    "latency" => {
                        if let Ok(latency) = kv[1].parse::<u32>() {
                            debug!("Latencia SRT configurada: {} ms", latency);
                        }
                    }
                    "maxbw" => {
                        debug!("Bandwidth máximo: {}", kv[1]);
                    }
                    "passphrase" => {
                        debug!("Passphrase configurado");
                    }
                    _ => {}
                }
            }
        }
    }
}

// ============================================================================
// IMPLEMENTACIÓN AVANZADA CON CRATE SRT-TOKIO (OPCIONAL)
// ============================================================================

#[cfg(feature = "srt-native")]
pub mod srt_native {
    use super::*;
    // Uncomment cuando uses srt-tokio:
    // use srt_tokio::SrtSocket;
    
    pub struct NativeSrtReceiver {
        url: String,
        // socket: Option<SrtSocket>,
    }
    
    impl NativeSrtReceiver {
        pub fn new(url: String) -> Self {
            Self {
                url,
                // socket: None,
            }
        }
        
        pub async fn connect(&mut self) -> Result<(), TranscoderError> {
            info!("Conectando con SRT nativo: {}", self.url);
            
            // Implementación con srt-tokio:
            // let socket = SrtSocket::builder()
            //     .latency(Duration::from_millis(120))
            //     .call(&self.url, None)
            //     .await?;
            // 
            // self.socket = Some(socket);
            
            Ok(())
        }
        
        pub async fn receive(&mut self) -> Result<Vec<u8>, TranscoderError> {
            // Implementación con srt-tokio
            todo!("Implementar con srt-tokio crate")
        }
    }
}

/// Builder para configurar SrtReceiver
pub struct SrtReceiverBuilder {
    url: String,
    latency_ms: u32,
    buffer_size: usize,
    max_bandwidth_mbps: Option<u32>,
    passphrase: Option<String>,
}

impl SrtReceiverBuilder {
    pub fn new(url: String) -> Self {
        Self {
            url,
            latency_ms: 120,
            buffer_size: 1316 * 7,
            max_bandwidth_mbps: None,
            passphrase: None,
        }
    }
    
    pub fn latency(mut self, latency_ms: u32) -> Self {
        self.latency_ms = latency_ms;
        self
    }
    
    pub fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }
    
    pub fn max_bandwidth(mut self, mbps: u32) -> Self {
        self.max_bandwidth_mbps = Some(mbps);
        self
    }
    
    pub fn passphrase(mut self, passphrase: String) -> Self {
        self.passphrase = Some(passphrase);
        self
    }
    
    pub fn build(self) -> SrtReceiver {
        SrtReceiver::new(self.url)
            .with_latency(self.latency_ms)
            .with_buffer_size(self.buffer_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_srt_url_parsing() {
        let receiver = SrtReceiver::new("srt://127.0.0.1:9000?mode=listener".to_string());
        let addr = receiver.parse_srt_url().unwrap();
        assert_eq!(addr.port(), 9000);
    }
    
    #[test]
    fn test_builder() {
        let receiver = SrtReceiverBuilder::new("srt://127.0.0.1:9000".to_string())
            .latency(200)
            .buffer_size(8192)
            .build();
        
        assert_eq!(receiver.latency_ms, 200);
        assert_eq!(receiver.buffer_size, 8192);
    }
}