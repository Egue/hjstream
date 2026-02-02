# hjstream

Sistema de orquestación para gestionar canales FFmpeg a través de una API REST, con soporte para múltiples formatos de entrada (SRT, RTMP, HLS, etc.) y salida UDP multicast.

## Características

- ✅ **API REST completa** para gestión de canales
- ✅ **Múltiples formatos de entrada**: SRT, RTMP, HLS, HTTP, RTSP
- ✅ **Salida UDP multicast** configurable
- ✅ **Auto-reconexión** automática en caso de fallos
- ✅ **Persistencia en JSON** de configuraciones
- ✅ **Restauración automática** de canales al reiniciar el servidor
- ✅ **Logging individual** por canal
- ✅ **Gestión de procesos** con control de estado

## Requisitos

- Rust 1.70 o superior
- FFmpeg instalado en el sistema
- Linux (Ubuntu 24 recomendado)

## Instalación

### 1. Instalar FFmpeg

```bash
sudo apt update
sudo apt install ffmpeg -y
```

### 2. Compilar el proyecto

```bash
cd hjstream
cargo build --release
```

### 3. Crear directorios necesarios

```bash
sudo mkdir -p /etc/hjstream
sudo mkdir -p /var/log/hjstream
sudo chown -R $USER:$USER /var/log/hjstream
```

### 4. Ejecutar el orquestador

```bash
cargo run --release
```

O copiar el binario:

```bash
sudo cp target/release/hjstream /usr/local/bin/
hjstream
```

## API REST

### Endpoints disponibles

#### Health Check
```bash
GET /health
```

#### Listar todos los canales
```bash
GET /api/channels
```

**Respuesta:**
```json
{
  "channels": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "10chv",
      "status": "running",
      "input": {...},
      "output": {...}
    }
  ],
  "total": 1
}
```

#### Crear un nuevo canal
```bash
POST /api/channels
Content-Type: application/json

{
  "name": "10chv",
  "description": "Canal 10 CHV",
  "input": {
    "format": "srt",
    "url": "192.x.x.x:111",
    "mode": "caller",
    "latency": 200000
  },
  "output": {
    "multicast_ip": "239.10.10.10",
    "port": 1010,
    "local_addr": "192.168.2.140",
    "pkt_size": 1316,
    "ttl": 64
  }
}
```

**Respuesta (201 Created):**
```json
{
  "channel": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "10chv",
    "status": "stopped",
    "created_at": "2026-02-01T10:00:00Z",
    ...
  }
}
```

#### Obtener detalles de un canal
```bash
GET /api/channels/{id}
```

#### Iniciar un canal
```bash
PUT /api/channels/{id}/start
```

#### Detener un canal
```bash
PUT /api/channels/{id}/stop
```

#### Reiniciar un canal
```bash
PUT /api/channels/{id}/restart
```

#### Eliminar un canal
```bash
DELETE /api/channels/{id}
```

## Ejemplos de uso

### Crear canal SRT a UDP
```bash
curl -X POST http://localhost:31337/api/channels \
  -H "Content-Type: application/json" \
  -d '{
    "name": "canal-srt-1",
    "input": {
      "format": "srt",
      "url": "192.x.x.x:111",
      "mode": "caller",
      "latency": 200000
    },
    "output": {
      "multicast_ip": "239.10.10.10",
      "port": 1010,
      "local_addr": "192.168.2.140"
    }
  }'
```

### Crear canal RTMP a UDP
```bash
curl -X POST http://localhost:31337/api/channels \
  -H "Content-Type: application/json" \
  -d '{
    "name": "canal-rtmp-1",
    "input": {
      "format": "rtmp",
      "url": "rtmp://live.example.com/stream/key"
    },
    "output": {
      "multicast_ip": "239.10.10.20",
      "port": 1020,
      "local_addr": "192.168.2.140"
    }
  }'
```

### Crear canal HLS a UDP
```bash
curl -X POST http://localhost:31337/api/channels \
  -H "Content-Type: application/json" \
  -d '{
    "name": "canal-hls-1",
    "input": {
      "format": "hls",
      "url": "https://example.com/stream/playlist.m3u8"
    },
    "output": {
      "multicast_ip": "239.10.10.30",
      "port": 1030,
      "local_addr": "192.168.2.140"
    }
  }'
```

### Iniciar un canal
```bash
# Obtener el ID del canal
CHANNEL_ID=$(curl -s http://localhost:31337/api/channels | jq -r '.channels[0].id')

# Iniciar el canal
curl -X PUT http://localhost:31337/api/channels/$CHANNEL_ID/start
```

### Listar todos los canales
```bash
curl http://localhost:31337/api/channels | jq
```

### Detener un canal
```bash
curl -X PUT http://localhost:31337/api/channels/$CHANNEL_ID/stop
```

## Configuración del sistema

### Crear servicio systemd

Crear el archivo `/etc/systemd/system/hjstream.service`:

```ini
[Unit]
Description=hjstream Service
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/hjstream
Restart=always
RestartSec=5
LimitNOFILE=1048576

[Install]
WantedBy=multi-user.target
```

Habilitar e iniciar el servicio:

```bash
sudo systemctl daemon-reload
sudo systemctl enable hjstream
sudo systemctl start hjstream
sudo systemctl status hjstream
```

## Estructura de archivos

```
/etc/hjstream/
  └── channels.json          # Configuración persistente de canales

/var/log/hjstream/
  ├── canal-1.log           # Logs del canal 1
  ├── canal-2.log           # Logs del canal 2
  └── ...
```

## Estados de los canales

- **stopped**: Canal detenido
- **starting**: Canal iniciando
- **running**: Canal corriendo normalmente
- **reconnecting**: Canal reconectando después de un fallo
- **error**: Canal en error

## Auto-reconexión

El orquestador implementa auto-reconexión automática:

1. Si FFmpeg se detiene inesperadamente, espera 5 segundos
2. Intenta reconectar automáticamente
3. Continúa intentando hasta que el canal sea detenido manualmente
4. Registra todos los intentos en el archivo de log del canal

## Logs

Cada canal tiene su propio archivo de log en `/var/log/hjstream/{nombre-canal}.log`.

Los logs incluyen:
- Inicio y detención del proceso FFmpeg
- Intentos de reconexión
- Errores y estado del proceso

## Variables de entorno

```bash
# Nivel de logging (trace, debug, info, warn, error)
RUST_LOG=debug

# Puerto del servidor (default: 31337)
PORT=31337
```

## Desarrollo

### Ejecutar en modo desarrollo
```bash
cargo run
```

### Ejecutar tests
```bash
cargo test
```

### Compilar para producción
```bash
cargo build --release
```

## Troubleshooting

### El canal no se inicia
1. Verificar que FFmpeg esté instalado: `ffmpeg -version`
2. Verificar los logs: `cat /var/log/hjstream/{nombre-canal}.log`
3. Verificar que la URL de entrada sea accesible

### Error de permisos
```bash
sudo chown -R $USER:$USER /var/log/hjstream
sudo chown -R $USER:$USER /etc/hjstream
```

### El servidor no guarda la configuración
Verificar permisos en `/etc/hjstream`:
```bash
ls -la /etc/hjstream/
```

## Licencia

MIT

## Autor

hjstream
