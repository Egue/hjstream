//! Cliente de Transcodificación CATV
//! 
//! Sistema distribuido de transcodificación de video para CATV
//! que recibe streams SRT, transcodifica y envía por UDP a moduladores.

pub mod api;
pub mod codec;
pub mod config;
pub mod core;
pub mod models;
pub mod monitor;
pub mod mux;
pub mod network;
pub mod utils;

// Re-exports públicos
pub use config::loader::{ConfigLoader, ClientConfig, ChannelConfig};
pub use core::manager::TranscoderManager;
pub use models::channel::Channel;
pub use models::stats::ChannelStats;
pub use models::error::TranscoderError;

/// Versión del cliente
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Resultado estándar usado en toda la aplicación
pub type Result<T> = std::result::Result<T, TranscoderError>;