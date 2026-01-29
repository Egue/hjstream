/// Program Association Table (PAT)
#[derive(Debug, Clone)]
pub struct Pat {
    pub service_id: u16,
    pub pmt_pid: u16,
}

impl Pat {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();
        
        // Pointer field
        data.push(0x00);
        
        // Table ID
        data.push(0x00);
        
        // Section syntax indicator + section length
        let section_length = 13;
        data.push(0xB0);
        data.push(section_length);
        
        // Transport stream ID
        data.push(0x00);
        data.push(0x01);
        
        // Version + current/next
        data.push(0xC1);
        
        // Section number
        data.push(0x00);
        
        // Last section number
        data.push(0x00);
        
        // Program number
        data.push((self.service_id >> 8) as u8);
        data.push((self.service_id & 0xFF) as u8);
        
        // PMT PID
        data.push(0xE0 | ((self.pmt_pid >> 8) as u8 & 0x1F));
        data.push((self.pmt_pid & 0xFF) as u8);
        
        // CRC32 (simplificado - calcular correctamente en producción)
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        
        data
    }
}

/// Program Map Table (PMT)
#[derive(Debug, Clone)]
pub struct Pmt {
    pub program_number: u16,
    pub pcr_pid: u16,
    pub streams: Vec<ProgramInfo>,
}

impl Pmt {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();
        
        // Pointer field
        data.push(0x00);
        
        // Table ID
        data.push(0x02);
        
        // Section length (calculado después)
        let section_length = 13 + (self.streams.len() * 5);
        data.push(0xB0);
        data.push(section_length as u8);
        
        // Program number
        data.push((self.program_number >> 8) as u8);
        data.push((self.program_number & 0xFF) as u8);
        
        // Version + current/next
        data.push(0xC1);
        
        // Section number
        data.push(0x00);
        
        // Last section number
        data.push(0x00);
        
        // PCR PID
        data.push(0xE0 | ((self.pcr_pid >> 8) as u8 & 0x1F));
        data.push((self.pcr_pid & 0xFF) as u8);
        
        // Program info length
        data.push(0xF0);
        data.push(0x00);
        
        // Streams
        for stream in &self.streams {
            data.push(stream.stream_type);
            data.push(0xE0 | ((stream.pid >> 8) as u8 & 0x1F));
            data.push((stream.pid & 0xFF) as u8);
            data.push(0xF0);
            data.push(0x00);
        }
        
        // CRC32 (simplificado)
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        
        data
    }
}

#[derive(Debug, Clone)]
pub struct ProgramInfo {
    pub stream_type: u8,
    pub pid: u16,
}