//! Módulo core - Lógica principal del transcodificador

pub mod manager;
pub mod channel;
pub mod transcoder;
pub mod analyzer;
pub mod strategy;
pub mod pipeline;

pub use manager::TranscoderManager;
pub use channel::Channel;
pub use transcoder::Transcoder;
pub use analyzer::StreamAnalyzer;
pub use strategy::{TranscodeStrategy, decide_strategy};