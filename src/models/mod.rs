//! Modelos de datos del sistema de transcodificación

pub mod channel;
pub mod config;
pub mod error;
pub mod stats;

pub use channel::Channel;
pub use error::TranscoderError;
pub use stats::{ChannelStats, ChannelStatus};