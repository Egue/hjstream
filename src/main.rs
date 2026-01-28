use anyhow::Result;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;
use tokio::signal;

mod api;
mod codec;
mod config;
mod core;
mod models;
mod monitor;
mod mux;
mod network;
mod utils;

use config::loader::ConfigLoader;
use config::sync::ConfigSyncService;
use api::client::ApiClient;
use core::manager::TranscoderManager;
use monitor::health_checker::HealthChecker;

#[tokio::main]
async fn main() -> Result<()> {
    // Inicializar sistema de logging
    init_logging()?;
    
    info!("=== Iniciando Cliente de Transcodificación CATV ===");
    
    // Cargar configuración del cliente
    let config_loader = ConfigLoader::new("config");
    let client_config = config_loader.load_client_config()?;
    
    info!("Cliente ID: {}", client_config.client_id);
    info!("Nombre: {}", client_config.name);
    info!("Backend: {}", client_config.backend.url);
    
    // Crear API client para comunicación con backend
    let api_client = ApiClient::new(
        client_config.backend.url.clone(),
        client_config.backend.api_key.clone(),
    );
    
    // Registrar cliente en el backend
    match api_client.register_client(&client_config).await {
        Ok(_) => info!("Cliente registrado exitosamente en el backend"),
        Err(e) => error!("Error registrando cliente: {}. Continuando en modo offline...", e),
    }
    
    // Crear manager de transcodificadores
    let manager = Arc::new(TranscoderManager::new(client_config.clone()));
    
    // Cargar y arrancar canales configurados
    if client_config.auto_start_channels {
        info!("Cargando canales configurados...");
        let channels = config_loader.load_all_channels()?;
        info!("Encontrados {} canales", channels.len());
        
        for channel_config in channels {
            info!("Iniciando canal: {} ({})", channel_config.id, channel_config.name);
            match manager.add_channel(channel_config).await {
                Ok(_) => info!("Canal iniciado exitosamente"),
                Err(e) => error!("Error iniciando canal: {}", e),
            }
        }
    }
    
    // Iniciar servicio de sincronización de configuración
    let sync_service = ConfigSyncService::new(
        config_loader.clone(),
        api_client.clone(),
        client_config.config_sync_interval_seconds,
    );
    
    let sync_handle = tokio::spawn(async move {
        sync_service.start().await;
    });
    
    // Iniciar health checker
    let health_checker = HealthChecker::new(
        manager.clone(),
        api_client.clone(),
    );
    
    let health_handle = tokio::spawn(async move {
        health_checker.start().await;
    });
    
    // Iniciar heartbeat con backend
    let heartbeat_api = api_client.clone();
    let heartbeat_interval = client_config.backend.heartbeat_interval_seconds;
    let client_id = client_config.client_id.clone();
    
    let heartbeat_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(heartbeat_interval)
        );
        
        loop {
            interval.tick().await;
            if let Err(e) = heartbeat_api.send_heartbeat(&client_id).await {
                error!("Error enviando heartbeat: {}", e);
            }
        }
    });
    
    // Iniciar servidor API local (REST + WebSocket)
    let server_manager = manager.clone();
    let server_config = client_config.server.clone();
    let server_handle = tokio::spawn(async move {
        if let Err(e) = start_local_server(server_manager, server_config).await {
            error!("Error en servidor local: {}", e);
        }
    });
    
    // Iniciar servidor de métricas Prometheus (si está habilitado)
    if client_config.server.enable_metrics {
        let metrics_port = client_config.server.metrics_port;
        tokio::spawn(async move {
            if let Err(e) = start_metrics_server(metrics_port).await {
                error!("Error en servidor de métricas: {}", e);
            }
        });
    }
    
    info!("=== Sistema iniciado correctamente ===");
    info!("Presiona Ctrl+C para detener");
    
    // Esperar señal de terminación
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Señal de terminación recibida");
        }
        Err(e) => {
            error!("Error esperando señal: {}", e);
        }
    }
    
    // Cleanup: detener todos los canales
    info!("Deteniendo canales...");
    manager.stop_all_channels().await?;
    
    // Cancelar tareas
    sync_handle.abort();
    health_handle.abort();
    heartbeat_handle.abort();
    server_handle.abort();
    
    info!("=== Sistema detenido correctamente ===");
    Ok(())
}

fn init_logging() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    Ok(())
}

async fn start_local_server(
    manager: Arc<TranscoderManager>,
    config: config::loader::ServerConfig,
) -> Result<()> {
    use axum::{
        extract::State,
        routing::{get, post, delete},
        Router,
    };
    use tower_http::cors::CorsLayer;
    use std::net::SocketAddr;
    
    let app = Router::new()
        .route("/health", get(api::handlers::health_check))
        .route("/channels", get(api::handlers::list_channels))
        .route("/channels/:id", get(api::handlers::get_channel))
        .route("/channels/:id/restart", post(api::handlers::restart_channel))
        .route("/stats", get(api::handlers::get_all_stats))
        .route("/ws", get(api::handlers::websocket_handler))
        .layer(CorsLayer::permissive())
        .with_state(manager);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Servidor API local escuchando en http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn start_metrics_server(port: u16) -> Result<()> {
    use axum::{routing::get, Router};
    use std::net::SocketAddr;
    
    let app = Router::new()
        .route("/metrics", get(monitor::metrics::metrics_handler));
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Servidor de métricas escuchando en http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}