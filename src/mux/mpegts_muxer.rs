use super::packet::{TsPacket, TsHeader, AdaptationField, TS_PACKET_SIZE};
use super::psi::{Pat, Pmt, ProgramInfo};
use std::collections::HashMap;

/// Muxer de MPEG-TS
pub struct MpegTsMuxer {
    pmt_pid: u16,
    video_pid: u16,
    audio_pid: u16,
    pcr_pid: u16,
    service_id: u16,
    continuity_counters: HashMap<u16, u8>,
    pcr_counter: u64,
}

impl MpegTsMuxer {
    pub fn new(pmt_pid: u16, video_pid: u16, audio_pid: u16) -> Self {
        Self {
            pmt_pid,
            video_pid,
            audio_pid,
            pcr_pid: video_pid, // PCR típicamente en el stream de video
            service_id: 1,
            continuity_counters: HashMap::new(),
            pcr_counter: 0,
        }
    }
    
    /// Generar PAT (Program Association Table)
    pub fn generate_pat(&mut self) -> TsPacket {
        let pat = Pat {
            service_id: self.service_id,
            pmt_pid: self.pmt_pid,
        };
        
        let payload = pat.to_bytes();
        
        let mut header = TsHeader::pat();
        header.continuity_counter = self.get_continuity_counter(0x0000);
        
        TsPacket {
            header,
            adaptation_field: None,
            payload,
        }
    }
    
    /// Generar PMT (Program Map Table)
    pub fn generate_pmt(&mut self) -> TsPacket {
        let pmt = Pmt {
            program_number: self.service_id,
            pcr_pid: self.pcr_pid,
            streams: vec![
                ProgramInfo {
                    stream_type: 0x1B, // H.264
                    pid: self.video_pid,
                },
                ProgramInfo {
                    stream_type: 0x0F, // AAC
                    pid: self.audio_pid,
                },
            ],
        };
        
        let payload = pmt.to_bytes();
        
        let mut header = TsHeader::pmt(self.pmt_pid);
        header.continuity_counter = self.get_continuity_counter(self.pmt_pid);
        
        TsPacket {
            header,
            adaptation_field: None,
            payload,
        }
    }
    
    /// Crear paquete para datos de video
    pub fn create_video_packet(&mut self, data: &[u8], pts: Option<u64>) -> Vec<TsPacket> {
        self.create_pes_packets(self.video_pid, data, 0xE0, pts)
    }
    
    /// Crear paquete para datos de audio
    pub fn create_audio_packet(&mut self, data: &[u8], pts: Option<u64>) -> Vec<TsPacket> {
        self.create_pes_packets(self.audio_pid, data, 0xC0, pts)
    }
    
    /// Crear paquetes PES (puede generar múltiples paquetes TS)
    fn create_pes_packets(&mut self, pid: u16, data: &[u8], stream_id: u8, pts: Option<u64>) -> Vec<TsPacket> {
        let mut packets = Vec::new();
        
        // Crear header PES
        let pes_header = self.create_pes_header(stream_id, data.len(), pts);
        
        // Combinar header PES + datos
        let mut full_data = pes_header;
        full_data.extend_from_slice(data);
        
        // Fragmentar en paquetes TS
        let max_payload = TS_PACKET_SIZE - 4; // 184 bytes
        let mut offset = 0;
        let mut first = true;
        
        while offset < full_data.len() {
            let chunk_size = (full_data.len() - offset).min(max_payload);
            let chunk = &full_data[offset..offset + chunk_size];
            
            let mut header = TsHeader {
                sync_byte: 0x47,
                transport_error: false,
                payload_unit_start: first,
                transport_priority: false,
                pid,
                scrambling_control: 0,
                adaptation_field_control: 1,
                continuity_counter: self.get_continuity_counter(pid),
            };
            
            // Agregar PCR cada ~100ms si es video
            let adaptation_field = if pid == self.pcr_pid && first {
                header.adaptation_field_control = 3; // Ambos
                Some(AdaptationField {
                    length: 7,
                    discontinuity: false,
                    random_access: first,
                    es_priority: false,
                    pcr_flag: true,
                    opcr_flag: false,
                    splicing_point_flag: false,
                    transport_private_data_flag: false,
                    adaptation_field_extension_flag: false,
                    pcr: Some(self.get_pcr()),
                    opcr: None,
                })
            } else {
                None
            };
            
            let packet = TsPacket {
                header,
                adaptation_field,
                payload: chunk.to_vec(),
            };
            
            packets.push(packet);
            
            offset += chunk_size;
            first = false;
        }
        
        packets
    }
    
    /// Crear header PES
    fn create_pes_header(&self, stream_id: u8, data_len: usize, pts: Option<u64>) -> Vec<u8> {
        let mut header = Vec::new();
        
        // Packet start code prefix
        header.extend_from_slice(&[0x00, 0x00, 0x01]);
        
        // Stream ID
        header.push(stream_id);
        
        // PES packet length (0 = no especificado para video)
        let pes_length = if stream_id == 0xE0 { 0 } else { (data_len + 8) as u16 };
        header.push((pes_length >> 8) as u8);
        header.push((pes_length & 0xFF) as u8);
        
        // Flags
        header.push(0x80); // '10'
        
        // PTS/DTS flags
        let pts_dts_flags = if pts.is_some() { 0x80 } else { 0x00 };
        header.push(pts_dts_flags);
        
        // PES header length
        let pes_header_len = if pts.is_some() { 5 } else { 0 };
        header.push(pes_header_len);
        
        // PTS
        if let Some(pts_value) = pts {
            header.push(0x21 | ((pts_value >> 29) as u8 & 0x0E));
            header.push((pts_value >> 22) as u8);
            header.push(0x01 | ((pts_value >> 14) as u8 & 0xFE));
            header.push((pts_value >> 7) as u8);
            header.push(0x01 | ((pts_value << 1) as u8 & 0xFE));
        }
        
        header
    }
    
    /// Obtener y actualizar continuity counter
    fn get_continuity_counter(&mut self, pid: u16) -> u8 {
        let counter = self.continuity_counters.entry(pid).or_insert(0);
        let current = *counter;
        *counter = (*counter + 1) & 0x0F;
        current
    }
    
    /// Obtener PCR actual
    fn get_pcr(&mut self) -> u64 {
        let pcr = self.pcr_counter;
        self.pcr_counter += 3600; // ~40ms @ 90kHz
        pcr
    }
    
    /// Muxear streams completos
    pub fn mux(&mut self, video_data: &[u8], audio_data: &[u8]) -> Vec<u8> {
        let mut output = Vec::new();
        
        // Insertar PAT
        output.extend_from_slice(&self.generate_pat().to_bytes());
        
        // Insertar PMT
        output.extend_from_slice(&self.generate_pmt().to_bytes());
        
        // Insertar paquetes de video
        for packet in self.create_video_packet(video_data, None) {
            output.extend_from_slice(&packet.to_bytes());
        }
        
        // Insertar paquetes de audio
        for packet in self.create_audio_packet(audio_data, None) {
            output.extend_from_slice(&packet.to_bytes());
        }
        
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_muxer_creation() {
        let muxer = MpegTsMuxer::new(0x1000, 0x0100, 0x0101);
        assert_eq!(muxer.pmt_pid, 0x1000);
        assert_eq!(muxer.video_pid, 0x0100);
        assert_eq!(muxer.audio_pid, 0x0101);
    }
    
    #[test]
    fn test_pat_generation() {
        let mut muxer = MpegTsMuxer::new(0x1000, 0x0100, 0x0101);
        let pat = muxer.generate_pat();
        
        assert_eq!(pat.header.pid, 0x0000);
        assert!(pat.header.payload_unit_start);
    }
}