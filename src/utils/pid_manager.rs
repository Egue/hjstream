use std::collections::HashMap;
use tracing::{info, warn};

/// Gestor de PIDs para MPEG-TS
/// Asigna y rastrea PIDs para streams de video, audio y datos
pub struct PidManager {
    allocated_pids: HashMap<String, u16>,
    next_available: u16,
    reserved_pids: Vec<u16>,
}

impl PidManager {
    /// PIDs reservados según el estándar MPEG-TS
    const PAT_PID: u16 = 0x0000;      // Program Association Table
    const CAT_PID: u16 = 0x0001;      // Conditional Access Table
    const TSDT_PID: u16 = 0x0002;     // Transport Stream Description Table
    const NULL_PID: u16 = 0x1FFF;     // Null packets
    
    pub fn new() -> Self {
        let mut reserved_pids = vec![
            Self::PAT_PID,
            Self::CAT_PID,
            Self::TSDT_PID,
            Self::NULL_PID,
        ];
        
        // Reservar rango 0x0003-0x000F (reservado por estándar)
        for pid in 0x0003..=0x000F {
            reserved_pids.push(pid);
        }
        
        Self {
            allocated_pids: HashMap::new(),
            next_available: 0x0100, // Comenzar desde 256
            reserved_pids,
        }
    }
    
    /// Crear con PIDs base específicos
    pub fn with_base_pids(pmt_pid: u16, video_pid: u16, audio_pid: u16) -> Self {
        let mut manager = Self::new();
        
        manager.allocate_specific("pmt", pmt_pid).ok();
        manager.allocate_specific("video", video_pid).ok();
        manager.allocate_specific("audio", audio_pid).ok();
        
        manager
    }
    
    /// Asignar un PID automáticamente
    pub fn allocate(&mut self, stream_type: &str) -> Result<u16, String> {
        // Buscar siguiente PID disponible
        while self.is_pid_used(self.next_available) {
            self.next_available += 1;
            
            if self.next_available >= 0x1FFF {
                return Err("No hay PIDs disponibles".to_string());
            }
        }
        
        let pid = self.next_available;
        self.allocated_pids.insert(stream_type.to_string(), pid);
        self.next_available += 1;
        
        info!("PID {} asignado a {}", pid, stream_type);
        Ok(pid)
    }
    
    /// Asignar un PID específico
    pub fn allocate_specific(&mut self, stream_type: &str, pid: u16) -> Result<u16, String> {
        if self.is_pid_reserved(pid) {
            return Err(format!("PID {} está reservado", pid));
        }
        
        if self.is_pid_allocated(pid) {
            return Err(format!("PID {} ya está asignado", pid));
        }
        
        self.allocated_pids.insert(stream_type.to_string(), pid);
        info!("PID {} asignado específicamente a {}", pid, stream_type);
        Ok(pid)
    }
    
    /// Liberar un PID
    pub fn free(&mut self, stream_type: &str) -> Option<u16> {
        if let Some(pid) = self.allocated_pids.remove(stream_type) {
            info!("PID {} liberado (era {})", pid, stream_type);
            Some(pid)
        } else {
            None
        }
    }
    
    /// Obtener PID asignado a un tipo de stream
    pub fn get_pid(&self, stream_type: &str) -> Option<u16> {
        self.allocated_pids.get(stream_type).copied()
    }
    
    /// Verificar si un PID está reservado
    pub fn is_pid_reserved(&self, pid: u16) -> bool {
        self.reserved_pids.contains(&pid)
    }
    
    /// Verificar si un PID está asignado
    pub fn is_pid_allocated(&self, pid: u16) -> bool {
        self.allocated_pids.values().any(|&p| p == pid)
    }
    
    /// Verificar si un PID está usado (reservado o asignado)
    pub fn is_pid_used(&self, pid: u16) -> bool {
        self.is_pid_reserved(pid) || self.is_pid_allocated(pid)
    }
    
    /// Obtener todos los PIDs asignados
    pub fn get_all_allocated(&self) -> HashMap<String, u16> {
        self.allocated_pids.clone()
    }
    
    /// Validar una configuración de PIDs
    pub fn validate_pids(&self, pmt: u16, video: u16, audio: u16) -> Result<(), String> {
        let pids = vec![pmt, video, audio];
        
        // Verificar que no haya duplicados
        for i in 0..pids.len() {
            for j in (i + 1)..pids.len() {
                if pids[i] == pids[j] {
                    return Err(format!("PIDs duplicados: {}", pids[i]));
                }
            }
        }
        
        // Verificar que no sean reservados
        for pid in &pids {
            if self.is_pid_reserved(*pid) {
                return Err(format!("PID {} está reservado", pid));
            }
        }
        
        Ok(())
    }
    
    /// Generar configuración de PIDs automáticamente
    pub fn auto_assign() -> PidAssignment {
        PidAssignment {
            pmt_pid: 0x1000,     // 4096
            pcr_pid: 0x0100,     // 256 (mismo que video típicamente)
            video_pid: 0x0100,   // 256
            audio_pid: 0x0101,   // 257
            subtitle_pid: None,
            data_pid: None,
        }
    }
}

impl Default for PidManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuración de asignación de PIDs
#[derive(Debug, Clone)]
pub struct PidAssignment {
    pub pmt_pid: u16,
    pub pcr_pid: u16,
    pub video_pid: u16,
    pub audio_pid: u16,
    pub subtitle_pid: Option<u16>,
    pub data_pid: Option<u16>,
}

impl PidAssignment {
    /// Crear asignación estándar para CATV
    pub fn catv_standard(service_id: u16) -> Self {
        // PIDs estándar basados en el ID de servicio
        let base = 0x0100 + (service_id * 0x10);
        
        Self {
            pmt_pid: 0x1000 + service_id,
            pcr_pid: base,
            video_pid: base,
            audio_pid: base + 1,
            subtitle_pid: Some(base + 2),
            data_pid: Some(base + 3),
        }
    }
    
    /// Validar la asignación
    pub fn validate(&self) -> Result<(), String> {
        let mut pids = vec![
            self.pmt_pid,
            self.pcr_pid,
            self.video_pid,
            self.audio_pid,
        ];
        
        if let Some(pid) = self.subtitle_pid {
            pids.push(pid);
        }
        
        if let Some(pid) = self.data_pid {
            pids.push(pid);
        }
        
        // Verificar duplicados (excepto PCR que puede ser igual a video)
        for i in 0..pids.len() {
            for j in (i + 1)..pids.len() {
                if pids[i] == pids[j] {
                    // PCR puede ser igual a video PID
                    if !(pids[i] == self.pcr_pid && pids[j] == self.video_pid) &&
                       !(pids[j] == self.pcr_pid && pids[i] == self.video_pid) {
                        return Err(format!("PIDs duplicados: {}", pids[i]));
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pid_allocation() {
        let mut manager = PidManager::new();
        
        let video_pid = manager.allocate("video").unwrap();
        let audio_pid = manager.allocate("audio").unwrap();
        
        assert_ne!(video_pid, audio_pid);
        assert!(manager.is_pid_allocated(video_pid));
        assert!(manager.is_pid_allocated(audio_pid));
    }
    
    #[test]
    fn test_specific_pid_allocation() {
        let mut manager = PidManager::new();
        
        let result = manager.allocate_specific("video", 256);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 256);
        
        // Intentar asignar el mismo PID
        let result2 = manager.allocate_specific("audio", 256);
        assert!(result2.is_err());
    }
    
    #[test]
    fn test_reserved_pids() {
        let manager = PidManager::new();
        
        assert!(manager.is_pid_reserved(PidManager::PAT_PID));
        assert!(manager.is_pid_reserved(PidManager::CAT_PID));
        assert!(manager.is_pid_reserved(PidManager::NULL_PID));
    }
    
    #[test]
    fn test_pid_validation() {
        let manager = PidManager::new();
        
        // Configuración válida
        assert!(manager.validate_pids(4096, 256, 257).is_ok());
        
        // PIDs duplicados
        assert!(manager.validate_pids(256, 256, 257).is_err());
        
        // PID reservado
        assert!(manager.validate_pids(0, 256, 257).is_err());
    }
    
    #[test]
    fn test_catv_standard_pids() {
        let assignment = PidAssignment::catv_standard(1);
        
        assert_eq!(assignment.pmt_pid, 0x1001);
        assert_eq!(assignment.video_pid, 0x0110);
        assert_eq!(assignment.audio_pid, 0x0111);
        
        assert!(assignment.validate().is_ok());
    }
}