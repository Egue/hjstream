use bytes::{Buf, BufMut, BytesMut};

/// Tamaño estándar de un paquete MPEG-TS
pub const TS_PACKET_SIZE: usize = 188;

/// Byte de sincronización MPEG-TS
pub const SYNC_BYTE: u8 = 0x47;

/// Paquete MPEG-TS
#[derive(Debug, Clone)]
pub struct TsPacket {
    pub header: TsHeader,
    pub adaptation_field: Option<AdaptationField>,
    pub payload: Vec<u8>,
}

/// Header de paquete MPEG-TS (4 bytes)
#[derive(Debug, Clone)]
pub struct TsHeader {
    pub sync_byte: u8,              // 0x47
    pub transport_error: bool,
    pub payload_unit_start: bool,
    pub transport_priority: bool,
    pub pid: u16,
    pub scrambling_control: u8,
    pub adaptation_field_control: u8,
    pub continuity_counter: u8,
}

/// Campo de adaptación (opcional)
#[derive(Debug, Clone)]
pub struct AdaptationField {
    pub length: u8,
    pub discontinuity: bool,
    pub random_access: bool,
    pub es_priority: bool,
    pub pcr_flag: bool,
    pub opcr_flag: bool,
    pub splicing_point_flag: bool,
    pub transport_private_data_flag: bool,
    pub adaptation_field_extension_flag: bool,
    pub pcr: Option<u64>,           // Program Clock Reference
    pub opcr: Option<u64>,          // Original PCR
}

impl TsPacket {
    /// Crear un nuevo paquete TS
    pub fn new(pid: u16, payload: Vec<u8>) -> Self {
        Self {
            header: TsHeader {
                sync_byte: SYNC_BYTE,
                transport_error: false,
                payload_unit_start: false,
                transport_priority: false,
                pid,
                scrambling_control: 0,
                adaptation_field_control: 1, // Solo payload
                continuity_counter: 0,
            },
            adaptation_field: None,
            payload,
        }
    }
    
    /// Parsear un paquete desde bytes
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        if data.len() < TS_PACKET_SIZE {
            return Err(format!("Tamaño inválido: {} bytes", data.len()));
        }
        
        // Verificar sync byte
        if data[0] != SYNC_BYTE {
            return Err(format!("Sync byte inválido: 0x{:02X}", data[0]));
        }
        
        // Parsear header (4 bytes)
        let header = Self::parse_header(&data[0..4])?;
        
        let mut offset = 4;
        let mut adaptation_field = None;
        
        // Parsear adaptation field si existe
        if header.adaptation_field_control & 0x02 != 0 {
            let af_length = data[offset] as usize;
            offset += 1;
            
            if af_length > 0 && offset + af_length <= TS_PACKET_SIZE {
                adaptation_field = Some(Self::parse_adaptation_field(&data[offset..offset + af_length])?);
                offset += af_length;
            }
        }
        
        // Extraer payload
        let payload = if header.adaptation_field_control & 0x01 != 0 {
            data[offset..TS_PACKET_SIZE].to_vec()
        } else {
            Vec::new()
        };
        
        Ok(Self {
            header,
            adaptation_field,
            payload,
        })
    }
    
    fn parse_header(data: &[u8]) -> Result<TsHeader, String> {
        if data.len() < 4 {
            return Err("Header demasiado corto".to_string());
        }
        
        let byte0 = data[0];
        let byte1 = data[1];
        let byte2 = data[2];
        let byte3 = data[3];
        
        Ok(TsHeader {
            sync_byte: byte0,
            transport_error: (byte1 & 0x80) != 0,
            payload_unit_start: (byte1 & 0x40) != 0,
            transport_priority: (byte1 & 0x20) != 0,
            pid: (((byte1 & 0x1F) as u16) << 8) | (byte2 as u16),
            scrambling_control: (byte3 & 0xC0) >> 6,
            adaptation_field_control: (byte3 & 0x30) >> 4,
            continuity_counter: byte3 & 0x0F,
        })
    }
    
    fn parse_adaptation_field(data: &[u8]) -> Result<AdaptationField, String> {
        if data.is_empty() {
            return Err("Adaptation field vacío".to_string());
        }
        
        let flags = data[0];
        let mut offset = 1;
        
        let mut pcr = None;
        
        // Parsear PCR si está presente
        if flags & 0x10 != 0 && data.len() >= offset + 6 {
            let pcr_base = ((data[offset] as u64) << 25)
                | ((data[offset + 1] as u64) << 17)
                | ((data[offset + 2] as u64) << 9)
                | ((data[offset + 3] as u64) << 1)
                | ((data[offset + 4] as u64) >> 7);
            
            let pcr_ext = (((data[offset + 4] & 0x01) as u64) << 8) | (data[offset + 5] as u64);
            
            pcr = Some(pcr_base * 300 + pcr_ext);
            offset += 6;
        }
        
        Ok(AdaptationField {
            length: data.len() as u8,
            discontinuity: (flags & 0x80) != 0,
            random_access: (flags & 0x40) != 0,
            es_priority: (flags & 0x20) != 0,
            pcr_flag: (flags & 0x10) != 0,
            opcr_flag: (flags & 0x08) != 0,
            splicing_point_flag: (flags & 0x04) != 0,
            transport_private_data_flag: (flags & 0x02) != 0,
            adaptation_field_extension_flag: (flags & 0x01) != 0,
            pcr,
            opcr: None, // Simplificado por ahora
        })
    }
    
    /// Serializar paquete a bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut packet = vec![0u8; TS_PACKET_SIZE];
        
        // Header
        packet[0] = self.header.sync_byte;
        
        packet[1] = ((self.header.transport_error as u8) << 7)
            | ((self.header.payload_unit_start as u8) << 6)
            | ((self.header.transport_priority as u8) << 5)
            | ((self.header.pid >> 8) as u8 & 0x1F);
        
        packet[2] = (self.header.pid & 0xFF) as u8;
        
        packet[3] = (self.header.scrambling_control << 6)
            | (self.header.adaptation_field_control << 4)
            | self.header.continuity_counter;
        
        let mut offset = 4;
        
        // Adaptation field
        if let Some(ref af) = self.adaptation_field {
            packet[offset] = af.length;
            offset += 1;
            
            // Flags
            let mut flags = 0u8;
            if af.discontinuity { flags |= 0x80; }
            if af.random_access { flags |= 0x40; }
            if af.es_priority { flags |= 0x20; }
            if af.pcr_flag { flags |= 0x10; }
            
            packet[offset] = flags;
            offset += 1;
            
            // PCR
            if let Some(pcr) = af.pcr {
                let pcr_base = pcr / 300;
                let pcr_ext = pcr % 300;
                
                packet[offset] = (pcr_base >> 25) as u8;
                packet[offset + 1] = (pcr_base >> 17) as u8;
                packet[offset + 2] = (pcr_base >> 9) as u8;
                packet[offset + 3] = (pcr_base >> 1) as u8;
                packet[offset + 4] = ((pcr_base & 0x01) << 7) as u8 | ((pcr_ext >> 8) as u8 & 0x01);
                packet[offset + 5] = (pcr_ext & 0xFF) as u8;
                offset += 6;
            }
        }
        
        // Payload
        let payload_len = self.payload.len().min(TS_PACKET_SIZE - offset);
        packet[offset..offset + payload_len].copy_from_slice(&self.payload[..payload_len]);
        
        // Padding con 0xFF
        if offset + payload_len < TS_PACKET_SIZE {
            for byte in &mut packet[offset + payload_len..] {
                *byte = 0xFF;
            }
        }
        
        packet
    }
    
    /// Verificar si el paquete es válido
    pub fn is_valid(&self) -> bool {
        self.header.sync_byte == SYNC_BYTE
    }
}

impl TsHeader {
    /// Crear header para PAT
    pub fn pat() -> Self {
        Self {
            sync_byte: SYNC_BYTE,
            transport_error: false,
            payload_unit_start: true,
            transport_priority: false,
            pid: 0x0000,
            scrambling_control: 0,
            adaptation_field_control: 1,
            continuity_counter: 0,
        }
    }
    
    /// Crear header para PMT
    pub fn pmt(pmt_pid: u16) -> Self {
        Self {
            sync_byte: SYNC_BYTE,
            transport_error: false,
            payload_unit_start: true,
            transport_priority: false,
            pid: pmt_pid,
            scrambling_control: 0,
            adaptation_field_control: 1,
            continuity_counter: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_packet_parse() {
        // Crear paquete de prueba con sync byte correcto
        let mut data = vec![0u8; TS_PACKET_SIZE];
        data[0] = SYNC_BYTE;
        data[1] = 0x40; // payload_unit_start
        data[2] = 0x00; // PID = 0 (PAT)
        data[3] = 0x10; // adaptation_field_control = 1, continuity_counter = 0
        
        let packet = TsPacket::parse(&data).unwrap();
        
        assert_eq!(packet.header.sync_byte, SYNC_BYTE);
        assert_eq!(packet.header.pid, 0);
        assert!(packet.header.payload_unit_start);
    }
    
    #[test]
    fn test_packet_serialize() {
        let packet = TsPacket::new(256, vec![0x01, 0x02, 0x03]);
        let bytes = packet.to_bytes();
        
        assert_eq!(bytes.len(), TS_PACKET_SIZE);
        assert_eq!(bytes[0], SYNC_BYTE);
    }
}