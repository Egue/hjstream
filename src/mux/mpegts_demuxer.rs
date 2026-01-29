use super::packet::{TsPacket, TS_PACKET_SIZE, SYNC_BYTE};
use bytes::BytesMut;
use std::collections::HashMap;
use tracing::{debug, warn};

/// Demuxer de MPEG-TS
pub struct MpegTsDemuxer {
    buffer: BytesMut,
    stream_buffers: HashMap<u16, Vec<u8>>,
    packet_count: u64,
}

impl MpegTsDemuxer {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::with_capacity(TS_PACKET_SIZE * 7),
            stream_buffers: HashMap::new(),
            packet_count: 0,
        }
    }
    
    /// Procesar datos de entrada
    pub fn process(&mut self, data: &[u8]) -> Result<Vec<TsPacket>, String> {
        self.buffer.extend_from_slice(data);
        let mut packets = Vec::new();
        
        // Buscar sync byte si no estamos alineados
        while !self.buffer.is_empty() && self.buffer[0] != SYNC_BYTE {
            warn!("Sync byte perdido, resincronizando...");
            self.buffer.advance(1);
        }
        
        // Procesar paquetes completos
        while self.buffer.len() >= TS_PACKET_SIZE {
            // Verificar sync byte
            if self.buffer[0] != SYNC_BYTE {
                // Buscar siguiente sync byte
                if let Some(pos) = self.buffer[1..].iter().position(|&b| b == SYNC_BYTE) {
                    self.buffer.advance(pos + 1);
                } else {
                    self.buffer.clear();
                    break;
                }
                continue;
            }
            
            // Extraer paquete
            let packet_data = self.buffer.split_to(TS_PACKET_SIZE);
            
            match TsPacket::parse(&packet_data) {
                Ok(packet) => {
                    self.packet_count += 1;
                    
                    // Acumular payload por PID
                    if !packet.payload.is_empty() {
                        let stream = self.stream_buffers.entry(packet.header.pid).or_insert_with(Vec::new);
                        stream.extend_from_slice(&packet.payload);
                    }
                    
                    packets.push(packet);
                }
                Err(e) => {
                    warn!("Error parseando paquete: {}", e);
                }
            }
        }
        
        Ok(packets)
    }
    
    /// Extraer stream por PID
    pub fn extract_stream(&mut self, pid: u16) -> Option<Vec<u8>> {
        self.stream_buffers.remove(&pid)
    }
    
    /// Obtener datos acumulados de un stream
    pub fn get_stream(&self, pid: u16) -> Option<&Vec<u8>> {
        self.stream_buffers.get(&pid)
    }
    
    /// Limpiar buffer de un stream
    pub fn clear_stream(&mut self, pid: u16) {
        self.stream_buffers.remove(&pid);
    }
    
    /// Obtener número de paquetes procesados
    pub fn packet_count(&self) -> u64 {
        self.packet_count
    }
    
    /// Obtener PIDs activos
    pub fn active_pids(&self) -> Vec<u16> {
        self.stream_buffers.keys().copied().collect()
    }
}

impl Default for MpegTsDemuxer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_demuxer() {
        let mut demuxer = MpegTsDemuxer::new();
        
        // Crear paquete de prueba
        let mut packet_data = vec![0u8; TS_PACKET_SIZE];
        packet_data[0] = SYNC_BYTE;
        packet_data[1] = 0x41; // PID = 0x100
        packet_data[2] = 0x00;
        packet_data[3] = 0x10;
        packet_data[4] = 0xAA; // Payload
        
        let packets = demuxer.process(&packet_data).unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(demuxer.packet_count(), 1);
    }
}