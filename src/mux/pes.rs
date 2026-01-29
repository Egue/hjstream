/// Paquete PES (Packetized Elementary Stream)
#[derive(Debug, Clone)]
pub struct PesPacket {
    pub stream_id: u8,
    pub packet_length: u16,
    pub pts: Option<u64>,
    pub dts: Option<u64>,
    pub data: Vec<u8>,
}

impl PesPacket {
    /// Parsear paquete PES
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < 6 {
            return Err("PES packet demasiado corto".to_string());
        }
        
        // Verificar start code
        if data[0] != 0x00 || data[1] != 0x00 || data[2] != 0x01 {
            return Err("Start code inválido".to_string());
        }
        
        let stream_id = data[3];
        let packet_length = ((data[4] as u16) << 8) | (data[5] as u16);
        
        // Parsear flags y PTS/DTS (simplificado)
        let mut pts = None;
        let mut dts = None;
        let mut data_offset = 9;
        
        if data.len() > 7 {
            let pts_dts_flags = (data[7] & 0xC0) >> 6;
            let pes_header_length = data[8] as usize;
            
            if pts_dts_flags >= 2 && data.len() >= data_offset + 5 {
                pts = Some(Self::parse_timestamp(&data[9..]));
            }
            
            data_offset = 9 + pes_header_length;
        }
        
        let payload = if data_offset < data.len() {
            data[data_offset..].to_vec()
        } else {
            Vec::new()
        };
        
        Ok(Self {
            stream_id,
            packet_length,
            pts,
            dts,
            data: payload,
        })
    }
    
    fn parse_timestamp(data: &[u8]) -> u64 {
        if data.len() < 5 {
            return 0;
        }
        
        let pts = (((data[0] as u64 & 0x0E) << 29)
            | ((data[1] as u64) << 22)
            | ((data[2] as u64 & 0xFE) << 14)
            | ((data[3] as u64) << 7)
            | ((data[4] as u64) >> 1)) as u64;
        
        pts
    }
}