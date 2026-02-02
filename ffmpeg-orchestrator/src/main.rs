mod handlers;
mod models;
mod process_manager;
mod storage;

use anyhow::Result;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use handlers::AppState;
use process_manager::ProcessManager;
use std::sync::Arc;
use storage::Storage;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Inicializar logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ffmpeg_orchestrator=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting FFmpeg Orchestrator");

    // Inicializar storage
    let storage = Arc::new(Storage::new(None).await?);
    tracing::info!("Storage initialized");

    // Inicializar process manager
    let process_manager = Arc::new(ProcessManager::new(Arc::clone(&storage)));
    tracing::info!("Process manager initialized");

    // Reiniciar canales que estaban corriendo
    restart_existing_channels(&storage, &process_manager).await?;

    // Crear estado compartido
    let state = Arc::new(AppState {
        storage,
        process_manager,
    });

    // Configurar rutas
    let app = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/channels", get(handlers::list_channels))
        .route("/api/channels", post(handlers::create_channel))
        .route("/api/channels/:id", get(handlers::get_channel))
        .route("/api/channels/:id", delete(handlers::delete_channel))
        .route("/api/channels/:id/start", put(handlers::start_channel))
        .route("/api/channels/:id/stop", put(handlers::stop_channel))
        .route("/api/channels/:id/restart", put(handlers::restart_channel))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Iniciar servidor
    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    tracing::info!("Server listening on {}", addr);
    tracing::info!("API Documentation:");
    tracing::info!("  GET    /health                    - Health check");
    tracing::info!("  GET    /api/channels               - List all channels");
    tracing::info!("  POST   /api/channels               - Create a new channel");
    tracing::info!("  GET    /api/channels/:id           - Get channel details");
    tracing::info!("  DELETE /api/channels/:id           - Delete a channel");
    tracing::info!("  PUT    /api/channels/:id/start     - Start a channel");
    tracing::info!("  PUT    /api/channels/:id/stop      - Stop a channel");
    tracing::info!("  PUT    /api/channels/:id/restart   - Restart a channel");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn restart_existing_channels(
    storage: &Arc<Storage>,
    process_manager: &Arc<ProcessManager>,
) -> Result<()> {
    let channels = storage.get_all_channels().await;
    
    let running_channels: Vec<_> = channels
        .into_iter()
        .filter(|c| {
            matches!(
                c.status,
                models::ChannelStatus::Running | models::ChannelStatus::Reconnecting
            )
        })
        .collect();

    if running_channels.is_empty() {
        tracing::info!("No channels to restart");
        return Ok(());
    }

    tracing::info!("Restarting {} channels", running_channels.len());

    for channel in running_channels {
        tracing::info!("Restarting channel: {} ({})", channel.name, channel.id);
        if let Err(e) = process_manager.start_channel(channel.id).await {
            tracing::error!("Failed to restart channel {}: {:?}", channel.name, e);
        }
    }

    Ok(())
}
