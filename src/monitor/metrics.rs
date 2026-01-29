#[cfg(feature = "metrics")]
use prometheus::{
    Counter, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry,
};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Colector de métricas Prometheus
#[cfg(feature = "metrics")]
pub struct MetricsCollector {
    registry: Registry,
    
    // Contadores
    frames_processed: IntCounter,
    frames_dropped: IntCounter,
    restarts_total: IntCounter,
    errors_total: IntCounter,
    
    // Gauges
    active_channels: IntGauge,
    current_fps: Gauge,
    current_bitrate: Gauge,
    
    // Histogramas
    processing_duration: Histogram,
}

#[cfg(feature = "metrics")]
impl MetricsCollector {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        // Contadores
        let frames_processed = IntCounter::with_opts(
            Opts::new("transcoder_frames_processed_total", "Total frames procesados")
        )?;
        registry.register(Box::new(frames_processed.clone()))?;
        
        let frames_dropped = IntCounter::with_opts(
            Opts::new("transcoder_frames_dropped_total", "Total frames descartados")
        )?;
        registry.register(Box::new(frames_dropped.clone()))?;
        
        let restarts_total = IntCounter::with_opts(
            Opts::new("transcoder_restarts_total", "Total de reinicios de canales")
        )?;
        registry.register(Box::new(restarts_total.clone()))?;
        
        let errors_total = IntCounter::with_opts(
            Opts::new("transcoder_errors_total", "Total de errores")
        )?;
        registry.register(Box::new(errors_total.clone()))?;
        
        // Gauges
        let active_channels = IntGauge::with_opts(
            Opts::new("transcoder_active_channels", "Número de canales activos")
        )?;
        registry.register(Box::new(active_channels.clone()))?;
        
        let current_fps = Gauge::with_opts(
            Opts::new("transcoder_fps", "FPS actual")
        )?;
        registry.register(Box::new(current_fps.clone()))?;
        
        let current_bitrate = Gauge::with_opts(
            Opts::new("transcoder_bitrate_kbps", "Bitrate actual en kbps")
        )?;
        registry.register(Box::new(current_bitrate.clone()))?;
        
        // Histogramas
        let processing_duration = Histogram::with_opts(
            HistogramOpts::new(
                "transcoder_processing_duration_seconds",
                "Duración de procesamiento de frames"
            )
        )?;
        registry.register(Box::new(processing_duration.clone()))?;
        
        Ok(Self {
            registry,
            frames_processed,
            frames_dropped,
            restarts_total,
            errors_total,
            active_channels,
            current_fps,
            current_bitrate,
            processing_duration,
        })
    }
    
    /// Incrementar frames procesados
    pub fn inc_frames_processed(&self, count: u64) {
        self.frames_processed.inc_by(count);
    }
    
    /// Incrementar frames descartados
    pub fn inc_frames_dropped(&self, count: u64) {
        self.frames_dropped.inc_by(count);
    }
    
    /// Incrementar reinicios
    pub fn inc_restarts(&self) {
        self.restarts_total.inc();
    }
    
    /// Incrementar errores
    pub fn inc_errors(&self) {
        self.errors_total.inc();
    }
    
    /// Actualizar canales activos
    pub fn set_active_channels(&self, count: i64) {
        self.active_channels.set(count);
    }
    
    /// Actualizar FPS
    pub fn set_fps(&self, fps: f64) {
        self.current_fps.set(fps);
    }
    
    /// Actualizar bitrate
    pub fn set_bitrate(&self, bitrate_kbps: f64) {
        self.current_bitrate.set(bitrate_kbps);
    }
    
    /// Registrar duración de procesamiento
    pub fn observe_processing_duration(&self, duration_seconds: f64) {
        self.processing_duration.observe(duration_seconds);
    }
    
    /// Obtener texto de métricas para Prometheus
    pub fn gather(&self) -> String {
        use prometheus::Encoder;
        
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        
        String::from_utf8(buffer).unwrap()
    }
}

#[cfg(not(feature = "metrics"))]
pub struct MetricsCollector;

#[cfg(not(feature = "metrics"))]
impl MetricsCollector {
    pub fn new() -> Result<Self, String> {
        Ok(Self)
    }
    
    pub fn inc_frames_processed(&self, _count: u64) {}
    pub fn inc_frames_dropped(&self, _count: u64) {}
    pub fn inc_restarts(&self) {}
    pub fn inc_errors(&self) {}
    pub fn set_active_channels(&self, _count: i64) {}
    pub fn set_fps(&self, _fps: f64) {}
    pub fn set_bitrate(&self, _bitrate_kbps: f64) {}
    pub fn observe_processing_duration(&self, _duration_seconds: f64) {}
    
    pub fn gather(&self) -> String {
        "# Metrics feature not enabled\n".to_string()
    }
}

/// Handler para endpoint /metrics
pub async fn metrics_handler() -> Response {
    #[cfg(feature = "metrics")]
    {
        // En producción, esto debería venir de un Arc<MetricsCollector> global
        match MetricsCollector::new() {
            Ok(collector) => {
                let metrics = collector.gather();
                (StatusCode::OK, metrics).into_response()
            }
            Err(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Error collecting metrics").into_response()
            }
        }
    }
    
    #[cfg(not(feature = "metrics"))]
    {
        (StatusCode::NOT_IMPLEMENTED, "Metrics feature not enabled").into_response()
    }
}

/// Metrics manager global
pub struct MetricsManager {
    collector: Arc<RwLock<MetricsCollector>>,
}

impl MetricsManager {
    pub fn new() -> Result<Self, String> {
        #[cfg(feature = "metrics")]
        let collector = MetricsCollector::new()
            .map_err(|e| format!("Error creando metrics collector: {}", e))?;
        
        #[cfg(not(feature = "metrics"))]
        let collector = MetricsCollector::new()?;
        
        Ok(Self {
            collector: Arc::new(RwLock::new(collector)),
        })
    }
    
    pub async fn update_from_stats(&self, stats: &crate::models::stats::ChannelStats) {
        let collector = self.collector.read().await;
        
        collector.inc_frames_processed(stats.frames_processed);
        collector.inc_frames_dropped(stats.dropped_frames);
        collector.set_fps(stats.current_fps);
        collector.set_bitrate(stats.output_bitrate_kbps);
    }
    
    pub async fn gather(&self) -> String {
        let collector = self.collector.read().await;
        collector.gather()
    }
}