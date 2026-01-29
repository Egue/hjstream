//! Módulo de configuración - Carga, sincronización y gestión de configuraciones

pub mod loader;
pub mod sync;

pub use loader::{ConfigLoader, ClientConfig, ChannelConfig};
pub use sync::ConfigSyncService;
