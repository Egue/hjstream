//! Módulo de API - Comunicación con el backend y endpoints locales

pub mod client;
pub mod handlers;
pub mod websocket;
pub mod registration;
pub mod heartbeat;

pub use client::ApiClient;
pub use handlers::*;
pub use websocket::WebSocketManager;