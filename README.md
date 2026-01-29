# CATV Transcoding System - Cliente Rust

Sistema distribuido de transcodificación de video para redes CATV (TV por Cable). Este cliente maneja la recepción de streams SRT, transcodificación en tiempo real y envío por UDP multicast a moduladores QAM.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

## 📋 Tabla de Contenidos

- [Características](#características)
- [Arquitectura](#arquitectura)
- [Requisitos](#requisitos)
- [Instalación](#instalación)
- [Configuración](#configuración)
- [Uso](#uso)
- [API REST](#api-rest)
- [Monitoreo](#monitoreo)
- [Deployment](#deployment)
- [Troubleshooting](#troubleshooting)
- [Desarrollo](#desarrollo)

## ✨ Características

### Core
- ✅ **Transcodificación en tiempo real** con FFmpeg
- ✅ **Soporte SRT** nativo para recepción de streams
- ✅ **Análisis inteligente** de streams para decisiones de transcodificación
- ✅ **Múltiples estrategias**: PassThrough, TranscodeVideo, TranscodeAudio, TranscodeBoth
- ✅ **Rate control CBR** optimizado para CATV
- ✅ **Salida UDP Multicast** con soporte MPEG-TS

### Gestión
- 🔄 **Auto-restart** de canales en caso de fallos
- 📊 **Estadísticas en tiempo real** (FPS, bitrate, frames descartados)
- 🔍 **Health checking** automático
- 💾 **Persistencia de configuración** en JSON
- 🔄 **Sincronización** con backend Quarkus
- 📝 **Logging estructurado** con niveles configurables

### Integración
- 🌐 **API REST** completa para control remoto
- 🔌 **WebSocket** para streaming de estadísticas
- 📈 **Métricas Prometheus** (opcional)
- 🔔 **Sistema de alertas** con deduplicación
- 🔐 **Autenticación** con API Key

### Hardware
- 🚀 **Aceleración por hardware** (NVENC, QSV, VAAPI)
- 💻 **Multi-threaded** para máximo rendimiento
- 🎯 **Optimizado para CATV** (CBR, PIDs correctos, MPEG-TS)

## 🏗️ Arquitectura
```
┌─────────────────────────────────────────────────────────────┐
│                    Backend Quarkus                          │
│         (Gestión Central, API REST, PostgreSQL)             │
└────────────────┬────────────────────────────────────────────┘
                 │ HTTP/REST + WebSocket
                 │
    ┌────────────┴────────┬────────────┬────────────┐
    │                     │            │            │
    ▼                     ▼            ▼            ▼
┌─────────┐         ┌─────────┐  ┌─────────┐  ┌─────────┐
│ Client  │         │ Client  │  │ Client  │  │ Client  │
│ Rust #1 │         │ Rust #2 │  │ Rust #3 │  │ Rust #4 │
└────┬────┘         └────┬────┘  └────┬────┘  └────┬────┘
     │                   │            │            │
   SRT in              SRT in       SRT in       SRT in
     │                   │            │            │
[Transcoder]        [Transcoder]  [Transcoder] [Transcoder]
     │                   │            │            │
  UDP out             UDP out      UDP out      UDP out
     │                   │            │            │
     ▼                   ▼            ▼            ▼
[Modulador]         [Modulador]  [Modulador]  [Modulador]
  CATV-2              CATV-2       CATV-2       CATV-2
     │                   │            │            │
     └───────────────────┴────────────┴────────────┘
                         │
                    Fibra Óptica
                         │
                    ┌────┴────┐
                    │   ONU   │
                    └────┬────┘
                         │
                    📺 TVs
```

## 📦 Requisitos

### Sistema Operativo
- **Linux**: Ubuntu 22.04+, Debian 11+, CentOS 8+, RHEL 8+
- **Kernel**: 5.4+

### Software
- **Rust**: 1.70 o superior
- **FFmpeg**: 4.4 o superior (con libx264, libfdk-aac)
- **FFprobe**: Incluido con FFmpeg
- **OpenSSL**: 1.1.1+

### Hardware Recomendado

#### Configuración Mínima (1-2 canales HD)
- **CPU**: 4 cores (Intel i5 / AMD Ryzen 5)
- **RAM**: 4 GB
- **Red**: 1 Gbps
- **Disco**: 20 GB

#### Configuración Recomendada (5-10 canales HD)
- **CPU**: 8 cores (Intel i7 / AMD Ryzen 7)
- **RAM**: 8 GB
- **Red**: 10 Gbps
- **Disco**: 50 GB SSD

#### Configuración Enterprise (20+ canales HD)
- **CPU**: 16+ cores (Intel Xeon / AMD EPYC)
- **RAM**: 16+ GB
- **GPU**: NVIDIA (para NVENC) o Intel (para QSV)
- **Red**: 10 Gbps bonded
- **Disco**: 100 GB NVMe SSD

### Aceleración por Hardware (Opcional)
- **NVIDIA**: Driver 470+ y CUDA Toolkit
- **Intel**: Driver iHD para VAAPI/QSV
- **AMD**: Mesa 22+ para VAAPI

## 🚀 Instalación

### Instalación Rápida
```bash
# 1. Instalar dependencias del sistema
sudo ./scripts/install-deps.sh

# 2. Instalar Rust (si no está instalado)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 3. Clonar repositorio
git clone https://github.com/tu-usuario/catv-transcoding-system.git
cd catv-transcoding-system/client-rust

# 4. Configurar
cp .env.example .env
nano .env  # Editar configuración

# 5. Crear directorios
mkdir -p config/channels config/backup logs

# 6. Compilar
./scripts/build.sh release

# 7. Iniciar
./scripts/start.sh background
```

### Instalación Detallada

#### 1. Dependencias del Sistema

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    ffmpeg \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev
```

**CentOS/RHEL:**
```bash
sudo yum install -y epel-release
sudo yum install -y \
    gcc \
    gcc-c++ \
    make \
    openssl-devel \
    ffmpeg \
    ffmpeg-devel
```

#### 2. Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version  # Verificar instalación
```

#### 3. Compilación
```bash
# Debug (rápido, para desarrollo)
cargo build

# Release (optimizado, para producción)
cargo build --release

# Con todas las características
cargo build --release --all-features
```

## ⚙️ Configuración

### Archivo Principal: `.env`
```bash
# Cliente
CLIENT_ID=transcoder-node-01
CLIENT_NAME="Transcodificador Principal"
CLIENT_LOCATION="Datacenter A - Rack 3"

# Backend
BACKEND_URL=http://backend.example.com:8080
API_KEY=tu-api-key-super-secreta

# Servidor Local
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
METRICS_PORT=9090

# Performance
MAX_CHANNELS=10
WORKER_THREADS=4
BUFFER_SIZE_MB=256

# Features
ENABLE_METRICS=true
ENABLE_AUTO_START=true
```

### Configuración de Cliente: `config/client.json`
```json
{
  "client_id": "transcoder-node-01",
  "name": "Transcodificador Principal",
  "location": "Datacenter A - Rack 3",
  "backend": {
    "url": "http://backend.example.com:8080",
    "api_key": "tu-api-key",
    "heartbeat_interval_seconds": 30,
    "reconnect_attempts": 5,
    "reconnect_delay_seconds": 10
  },
  "server": {
    "host": "0.0.0.0",
    "port": 8080,
    "enable_metrics": true,
    "metrics_port": 9090
  },
  "performance": {
    "max_channels": 10,
    "worker_threads": 4,
    "buffer_size_mb": 256
  },
  "auto_start_channels": true
}
```

### Configuración de Canal: `config/channels/channel_001.json`
```json
{
  "id": "channel_001",
  "name": "Canal 1 HD",
  "enabled": true,
  "mode": "srt_transcoder",
  "input": {
    "type": "srt",
    "url": "srt://10.0.1.100:9000?mode=listener",
    "latency_ms": 200
  },
  "output": {
    "type": "udp",
    "url": "udp://239.0.0.1:5000",
    "local_interface": "192.168.1.10",
    "ttl": 32,
    "packet_size": 1316
  },
  "video": {
    "codec": "h264",
    "profile": "main",
    "level": "4.0",
    "bitrate_kbps": 4000,
    "max_bitrate_kbps": 4500,
    "framerate": 30,
    "gop_size": 60,
    "preset": "medium",
    "rate_control": "cbr"
  },
  "audio": {
    "codec": "aac",
    "bitrate_kbps": 128,
    "sample_rate": 48000,
    "channels": 2
  },
  "mpegts": {
    "pmt_pid": 4096,
    "video_pid": 256,
    "audio_pid": 257,
    "pcr_pid": 256
  }
}
```

## 🎮 Uso

### Scripts de Gestión
```bash
# Iniciar (foreground)
./scripts/start.sh

# Iniciar (background)
./scripts/start.sh background

# Detener
./scripts/stop.sh

# Reiniciar
./scripts/restart.sh

# Ver estado
./scripts/status.sh

# Ver logs
./scripts/logs.sh tail

# Health check
./scripts/health-check.sh

# Backup de configuración
./scripts/backup-config.sh

# Restaurar configuración
./scripts/restore-config.sh
```

### Ejecución Manual
```bash
# Con configuración por defecto
./target/release/catv-transcoder

# Con variables de entorno
RUST_LOG=debug ./target/release/catv-transcoder

# Con archivo de log específico
LOG_FILE=/var/log/transcoder.log ./target/release/catv-transcoder
```

## 🌐 API REST

Base URL: `http://localhost:8080`

### Endpoints

#### Health Check
```bash
GET /health
```

**Respuesta:**
```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "version": "0.1.0",
    "uptime": "2h 15m"
  }
}
```

#### Listar Canales
```bash
GET /channels
```

#### Obtener Canal
```bash
GET /channels/{id}
```

#### Crear Canal
```bash
POST /channels
Content-Type: application/json

{
  "config": {
    "id": "channel_002",
    "name": "Canal 2 HD",
    "enabled": true,
    ...
  }
}
```

#### Reiniciar Canal
```bash
POST /channels/{id}/restart
```

#### Eliminar Canal
```bash
DELETE /channels/{id}
```

#### Estadísticas Agregadas
```bash
GET /stats
```

#### Resumen General
```bash
GET /monitor/summary
```

**Respuesta:**
```json
{
  "success": true,
  "data": {
    "total_channels": 5,
    "running": 5,
    "stopped": 0,
    "error": 0,
    "total_fps": 150.0,
    "total_bitrate_kbps": 20000.0,
    "channels": [...]
  }
}
```

### WebSocket

Conectar para estadísticas en tiempo real:
```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Stats:', data);
};
```

### Ejemplos con curl
```bash
# Health check
curl http://localhost:8080/health

# Listar canales
curl http://localhost:8080/channels

# Obtener estadísticas
curl http://localhost:8080/stats

# Reiniciar canal
curl -X POST http://localhost:8080/channels/channel_001/restart

# Crear canal
curl -X POST http://localhost:8080/channels \
  -H "Content-Type: application/json" \
  -d @config/channels/channel_002.json
```

## 📊 Monitoreo

### Logs
```bash
# Ver logs en tiempo real
./scripts/logs.sh tail

# Ver solo errores
./scripts/logs.sh errors

# Ver con less
./scripts/logs.sh less
```

### Métricas Prometheus

Si está habilitado (`ENABLE_METRICS=true`):
```bash
# Endpoint de métricas
curl http://localhost:9090/metrics
```

**Métricas disponibles:**
- `transcoder_frames_processed_total` - Total de frames procesados
- `transcoder_frames_dropped_total` - Frames descartados
- `transcoder_active_channels` - Canales activos
- `transcoder_fps` - FPS actual
- `transcoder_bitrate_kbps` - Bitrate en kbps
- `transcoder_restarts_total` - Total de reinicios
- `transcoder_errors_total` - Total de errores

### Configuración Prometheus

**prometheus.yml:**
```yaml
scrape_configs:
  - job_name: 'catv-transcoder'
    static_configs:
      - targets: ['localhost:9090']
        labels:
          instance: 'transcoder-01'
```

### Grafana

Importar dashboard desde `deploy/grafana-dashboard.json`

## 🚢 Deployment

### Systemd Service

Copiar archivo de servicio:
```bash
sudo cp deploy/catv-transcoder.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable catv-transcoder
sudo systemctl start catv-transcoder
```

**Comandos:**
```bash
# Iniciar
sudo systemctl start catv-transcoder

# Detener
sudo systemctl stop catv-transcoder

# Reiniciar
sudo systemctl restart catv-transcoder

# Estado
sudo systemctl status catv-transcoder

# Ver logs
sudo journalctl -u catv-transcoder -f
```

### Docker
```bash
# Build
docker build -t catv-transcoder:latest .

# Run
docker run -d \
  --name catv-transcoder \
  --network host \
  -v $(pwd)/config:/app/config \
  -v $(pwd)/logs:/app/logs \
  catv-transcoder:latest
```

### Docker Compose
```bash
docker-compose up -d
```

## 🔧 Troubleshooting

### El transcoder no inicia

**Verificar:**
1. FFmpeg está instalado: `ffmpeg -version`
2. Puertos disponibles: `netstat -tulpn | grep 8080`
3. Configuración válida: `cat .env`
4. Logs: `./scripts/logs.sh tail`

### Canales no procesan frames

**Posibles causas:**
- Stream SRT no disponible
- Codec no soportado
- Bitrate insuficiente de CPU

**Diagnóstico:**
```bash
# Ver estado
./scripts/status.sh

# Verificar stream SRT manualmente
ffprobe srt://source:9000?mode=listener

# Ver estadísticas
curl http://localhost:8080/channels/channel_001
```

### Alto uso de CPU

**Soluciones:**
1. Reducir número de canales
2. Usar preset más rápido (ultrafast, superfast)
3. Habilitar aceleración por hardware
4. Aumentar threads: `WORKER_THREADS=8`

### Frames descartados

**Causas:**
- CPU insuficiente
- Red congestionada
- Problemas de entrada

**Soluciones:**
```bash
# Verificar FPS y bitrate
curl http://localhost:8080/stats

# Ajustar buffer
# En .env: BUFFER_SIZE_MB=512

# Usar hardware encoding
# En channel config: "hardware_acceleration": {"enabled": true}
```

### Problemas de red
```bash
# Verificar conectividad SRT
nc -zv srt-source 9000

# Verificar multicast
iperf3 -c 239.0.0.1 -u -b 10M

# Ver interfaces
ip addr show
```

## 👨‍💻 Desarrollo

### Setup
```bash
# Instalar dependencias de desarrollo
cargo install cargo-watch cargo-edit cargo-outdated

# Compilar en modo debug
cargo build

# Ejecutar con auto-reload
cargo watch -x run
```

### Testing
```bash
# Ejecutar todos los tests
cargo test

# Tests específicos
cargo test --test integration_test

# Con output detallado
cargo test -- --nocapture

# Coverage
cargo tarpaulin --out Html
```

### Linting y Formateo
```bash
# Formatear código
cargo fmt

# Linter
cargo clippy -- -D warnings

# Verificar
cargo check
```

### Estructura del Proyecto
```
client-rust/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Librería
│   ├── api/                 # API REST y WebSocket
│   ├── codec/               # Encoders/decoders
│   ├── config/              # Gestión de configuración
│   ├── core/                # Lógica principal
│   ├── models/              # Modelos de datos
│   ├── monitor/             # Monitoreo y health checks
│   ├── mux/                 # MPEG-TS muxing/demuxing
│   ├── network/             # SRT, UDP, multicast
│   └── utils/               # Utilidades
├── config/                  # Configuraciones
├── scripts/                 # Scripts de operación
├── deploy/                  # Archivos de deployment
└── tests/                   # Tests de integración
```

## 📄 Licencia

MIT License - Ver [LICENSE](LICENSE) para detalles

## 🤝 Contribuir

1. Fork el proyecto
2. Crea tu rama: `git checkout -b feature/nueva-funcionalidad`
3. Commit: `git commit -am 'Agrega nueva funcionalidad'`
4. Push: `git push origin feature/nueva-funcionalidad`
5. Abre un Pull Request

## 📞 Soporte

- **Issues**: [GitHub Issues](https://github.com/tu-usuario/catv-transcoding-system/issues)
- **Documentación**: [Wiki](https://github.com/tu-usuario/catv-transcoding-system/wiki)
- **Email**: support@example.com

## 🙏 Agradecimientos

- FFmpeg team
- Rust community
- Axum framework
- Tokio async runtime

---

**Hecho con ❤️ para la industria CATV**