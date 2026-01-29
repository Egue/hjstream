use crate::core::manager::TranscoderManager;
use axum::extract::ws::{Message, WebSocket};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};

/// Manager de conexiones WebSocket para estadísticas en tiempo real
pub struct WebSocketManager;

impl WebSocketManager {
    /// Maneja una conexión WebSocket individual
    pub async fn handle_connection(socket: WebSocket, manager: Arc<TranscoderManager>) {
        let (mut sender, mut receiver) = socket.split();
        
        // Canal para comunicación entre tareas
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);
        
        // Tarea para enviar estadísticas periódicamente
        let manager_clone = manager.clone();
        let stats_task = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(2));
            
            loop {
                interval.tick().await;
                
                // Obtener estadísticas de todos los canales
                let stats = manager_clone.get_all_stats().await;
                
                let message = StatsMessage {
                    message_type: "stats_update".to_string(),
                    timestamp: chrono::Utc::now(),
                    data: StatsData::AllChannels(stats),
                };
                
                if let Ok(json) = serde_json::to_string(&message) {
                    if tx.send(json).await.is_err() {
                        break; // Cliente desconectado
                    }
                } else {
                    error!("Error serializando estadísticas");
                }
            }
        });
        
        // Tarea para enviar mensajes al cliente
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if sender.send(Message::Text(msg)).await.is_err() {
                    break; // Error enviando, cliente desconectado
                }
            }
        });
        
        // Tarea para recibir mensajes del cliente
        let receive_task = tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        info!("Mensaje recibido del cliente WS: {}", text);
                        
                        // Procesar comandos del cliente
                        if let Ok(command) = serde_json::from_str::<ClientCommand>(&text) {
                            handle_client_command(command, &manager).await;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("Cliente cerró conexión WebSocket");
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        // Responder al ping automáticamente
                        info!("Ping recibido");
                    }
                    Err(e) => {
                        error!("Error en WebSocket: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });
        
        // Esperar a que termine alguna tarea
        tokio::select! {
            _ = stats_task => {
                info!("Tarea de estadísticas terminada");
            }
            _ = send_task => {
                info!("Tarea de envío terminada");
            }
            _ = receive_task => {
                info!("Tarea de recepción terminada");
            }
        }
        
        info!("Conexión WebSocket cerrada");
    }
}

/// Punto de entrada para el handler de WebSocket
pub async fn handle_socket(socket: WebSocket, manager: Arc<TranscoderManager>) {
    info!("Nueva conexión WebSocket establecida");
    WebSocketManager::handle_connection(socket, manager).await;
}

// ============================================================================
// MENSAJES Y COMANDOS
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct StatsMessage {
    message_type: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    data: StatsData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum StatsData {
    AllChannels(std::collections::HashMap<String, crate::models::stats::ChannelStats>),
    SingleChannel(crate::models::stats::ChannelStats),
}

#[derive(Debug, Deserialize)]
struct ClientCommand {
    command: String,
    channel_id: Option<String>,
    parameters: Option<serde_json::Value>,
}

async fn handle_client_command(command: ClientCommand, manager: &Arc<TranscoderManager>) {
    info!("Procesando comando: {:?}", command.command);
    
    match command.command.as_str() {
        "subscribe_channel" => {
            if let Some(channel_id) = command.channel_id {
                info!("Cliente suscrito al canal: {}", channel_id);
                // Implementar lógica de suscripción específica
            }
        }
        "unsubscribe_channel" => {
            if let Some(channel_id) = command.channel_id {
                info!("Cliente desuscrito del canal: {}", channel_id);
            }
        }
        "restart_channel" => {
            if let Some(channel_id) = command.channel_id {
                info!("Reiniciando canal vía WebSocket: {}", channel_id);
                if let Err(e) = manager.restart_channel(&channel_id).await {
                    error!("Error reiniciando canal: {}", e);
                }
            }
        }
        _ => {
            warn!("Comando desconocido: {}", command.command);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stats_message_serialization() {
        use std::collections::HashMap;
        
        let message = StatsMessage {
            message_type: "stats_update".to_string(),
            timestamp: chrono::Utc::now(),
            data: StatsData::AllChannels(HashMap::new()),
        };
        
        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("stats_update"));
    }
}