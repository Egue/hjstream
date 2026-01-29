use std::time::{Duration, Instant};

/// Calculadora de bitrate en tiempo real
pub struct BitrateCalculator {
    window_duration: Duration,
    samples: Vec<BitrateSample>,
    max_samples: usize,
}

#[derive(Debug, Clone)]
struct BitrateSample {
    timestamp: Instant,
    bytes: u64,
}

impl BitrateCalculator {
    /// Crear calculadora con ventana de tiempo específica
    pub fn new(window_duration: Duration) -> Self {
        Self {
            window_duration,
            samples: Vec::new(),
            max_samples: 1000, // Límite de muestras en memoria
        }
    }
    
    /// Crear con ventana de 1 segundo
    pub fn new_1sec() -> Self {
        Self::new(Duration::from_secs(1))
    }
    
    /// Crear con ventana de 5 segundos
    pub fn new_5sec() -> Self {
        Self::new(Duration::from_secs(5))
    }
    
    /// Registrar bytes transferidos
    pub fn add_sample(&mut self, bytes: u64) {
        let sample = BitrateSample {
            timestamp: Instant::now(),
            bytes,
        };
        
        self.samples.push(sample);
        
        // Limpiar muestras antiguas
        self.cleanup_old_samples();
        
        // Limitar número de muestras
        if self.samples.len() > self.max_samples {
            self.samples.remove(0);
        }
    }
    
    /// Calcular bitrate actual en bits por segundo
    pub fn calculate_bps(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        
        let now = Instant::now();
        let cutoff = now - self.window_duration;
        
        let total_bytes: u64 = self.samples.iter()
            .filter(|s| s.timestamp >= cutoff)
            .map(|s| s.bytes)
            .sum();
        
        let window_secs = self.window_duration.as_secs_f64();
        
        if window_secs > 0.0 {
            (total_bytes as f64 * 8.0) / window_secs
        } else {
            0.0
        }
    }
    
    /// Calcular bitrate en kilobits por segundo
    pub fn calculate_kbps(&self) -> f64 {
        self.calculate_bps() / 1000.0
    }
    
    /// Calcular bitrate en megabits por segundo
    pub fn calculate_mbps(&self) -> f64 {
        self.calculate_bps() / 1_000_000.0
    }
    
    /// Calcular bitrate promedio de todas las muestras
    pub fn average_bps(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        
        let total_bytes: u64 = self.samples.iter().map(|s| s.bytes).sum();
        
        if let (Some(first), Some(last)) = (self.samples.first(), self.samples.last()) {
            let duration = last.timestamp.duration_since(first.timestamp).as_secs_f64();
            if duration > 0.0 {
                return (total_bytes as f64 * 8.0) / duration;
            }
        }
        
        0.0
    }
    
    /// Calcular estadísticas
    pub fn stats(&self) -> BitrateStats {
        let current_bps = self.calculate_bps();
        let average_bps = self.average_bps();
        
        // Calcular min y max
        let bitrates: Vec<f64> = self.calculate_historical_bitrates();
        
        let min_bps = bitrates.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_bps = bitrates.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        BitrateStats {
            current_bps,
            current_kbps: current_bps / 1000.0,
            current_mbps: current_bps / 1_000_000.0,
            average_bps,
            average_kbps: average_bps / 1000.0,
            average_mbps: average_bps / 1_000_000.0,
            min_bps: if min_bps.is_finite() { min_bps } else { 0.0 },
            max_bps: if max_bps.is_finite() { max_bps } else { 0.0 },
            sample_count: self.samples.len(),
        }
    }
    
    /// Resetear estadísticas
    pub fn reset(&mut self) {
        self.samples.clear();
    }
    
    // Métodos privados
    
    fn cleanup_old_samples(&mut self) {
        let now = Instant::now();
        let cutoff = now - self.window_duration;
        
        self.samples.retain(|sample| sample.timestamp >= cutoff);
    }
    
    fn calculate_historical_bitrates(&self) -> Vec<f64> {
        let mut bitrates = Vec::new();
        
        if self.samples.len() < 2 {
            return bitrates;
        }
        
        for i in 1..self.samples.len() {
            let duration = self.samples[i].timestamp
                .duration_since(self.samples[i - 1].timestamp)
                .as_secs_f64();
            
            if duration > 0.0 {
                let bps = (self.samples[i].bytes as f64 * 8.0) / duration;
                bitrates.push(bps);
            }
        }
        
        bitrates
    }
}

/// Estadísticas de bitrate
#[derive(Debug, Clone)]
pub struct BitrateStats {
    pub current_bps: f64,
    pub current_kbps: f64,
    pub current_mbps: f64,
    pub average_bps: f64,
    pub average_kbps: f64,
    pub average_mbps: f64,
    pub min_bps: f64,
    pub max_bps: f64,
    pub sample_count: usize,
}

impl BitrateStats {
    /// Calcular varianza del bitrate (para detectar CBR vs VBR)
    pub fn variance(&self) -> f64 {
        if self.max_bps == 0.0 {
            return 0.0;
        }
        ((self.max_bps - self.min_bps) / self.average_bps) * 100.0
    }
    
    /// Verificar si el bitrate es constante (CBR)
    pub fn is_cbr(&self, tolerance_percent: f64) -> bool {
        self.variance() <= tolerance_percent
    }
}

/// Helper para calcular bitrate objetivo para CATV
pub struct CatvBitrateHelper;

impl CatvBitrateHelper {
    /// Calcular bitrate recomendado para resolución
    pub fn recommended_video_bitrate(width: u32, height: u32, fps: u32) -> u32 {
        match (width, height) {
            // SD
            (720, 480) | (720, 576) => 2_500,  // 2.5 Mbps
            
            // HD 720p
            (1280, 720) => {
                if fps >= 50 {
                    4_500  // 4.5 Mbps para 50/60fps
                } else {
                    3_500  // 3.5 Mbps para 25/30fps
                }
            }
            
            // Full HD 1080p
            (1920, 1080) => {
                if fps >= 50 {
                    8_000  // 8 Mbps para 50/60fps
                } else {
                    6_000  // 6 Mbps para 25/30fps
                }
            }
            
            // 4K (no típico en CATV tradicional)
            (3840, 2160) => 15_000,
            
            // Fallback
            _ => {
                // Calcular basado en píxeles
                let pixels = width * height * fps;
                (pixels / 50_000) as u32  // ~0.02 bits por pixel
            }
        }
    }
    
    /// Calcular bitrate total del canal (video + audio + overhead)
    pub fn total_channel_bitrate(video_kbps: u32, audio_kbps: u32) -> u32 {
        let overhead = 0.05; // 5% de overhead para MPEG-TS
        let total = video_kbps + audio_kbps;
        (total as f64 * (1.0 + overhead)) as u32
    }
    
    /// Calcular cuántos canales HD caben en un transponder
    pub fn channels_per_transponder(
        transponder_mbps: u32,
        channel_bitrate_kbps: u32,
    ) -> u32 {
        (transponder_mbps * 1000) / channel_bitrate_kbps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_bitrate_calculation() {
        let mut calc = BitrateCalculator::new_1sec();
        
        // Simular 1MB en 1 segundo = 8 Mbps
        calc.add_sample(1_000_000);
        thread::sleep(Duration::from_millis(100));
        
        let bps = calc.calculate_bps();
        assert!(bps > 0.0);
    }
    
    #[test]
    fn test_recommended_bitrates() {
        // HD 720p @ 30fps
        let bitrate = CatvBitrateHelper::recommended_video_bitrate(1280, 720, 30);
        assert_eq!(bitrate, 3_500);
        
        // Full HD @ 60fps
        let bitrate = CatvBitrateHelper::recommended_video_bitrate(1920, 1080, 60);
        assert_eq!(bitrate, 8_000);
    }
    
    #[test]
    fn test_total_bitrate() {
        let total = CatvBitrateHelper::total_channel_bitrate(4_000, 128);
        // 4000 + 128 = 4128, + 5% overhead = ~4334
        assert!(total >= 4_300 && total <= 4_400);
    }
}