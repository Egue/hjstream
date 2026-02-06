# MediaMTX Orchestrator

Sistema de gestión y orquestación de streams de video que lee canales RTSP desde MediaMTX y los inyecta a Ethernet vía UDP.

## Características

- 📡 **Gestión de Canales**: Administra múltiples canales de video desde una interfaz web
- 🔄 **Conversión RTSP → UDP**: Lee streams RTSP y los inyecta a Ethernet vía UDP
- 🎯 **Multi-protocolo**: Soporta fuentes SRT, RTMP, HLS y RTSP
- 💻 **Interfaz Web**: UI moderna y responsive con Alpine.js
- ⚡ **Alto Rendimiento**: Escrito en Go, bajo consumo de recursos
- 🔒 **Concurrencia Segura**: Manejo thread-safe de múltiples streams

## Requisitos

- Go 1.21 o superior
- MediaMTX instalado y configurado

## Instalación

### 1. Descomprimir el archivo

```bash
tar -xzf mediamtx-orchestrator.tar.gz
cd mediamtx-orchestrator
```

### 2. Descargar dependencias

```bash
go mod download
```

### 3. Compilar

```bash
# Para Linux
go build -o orchestrator cmd/server/main.go

# Para Linux (optimizado, tamaño reducido)
go build -ldflags="-s -w" -o orchestrator cmd/server/main.go

# Para Windows
GOOS=windows GOARCH=amd64 go build -o orchestrator.exe cmd/server/main.go

# Para macOS
GOOS=darwin GOARCH=amd64 go build -o orchestrator-mac cmd/server/main.go

# Para Raspberry Pi (ARM64)
GOOS=linux GOARCH=arm64 go build -o orchestrator-arm64 cmd/server/main.go
```

## Uso

### 1. Iniciar el servidor

```bash
./orchestrator
```

El servidor iniciará en `http://localhost:8080`

### 2. Acceder a la interfaz web

Abre tu navegador y accede a:
```
http://localhost:8080
```

### 3. Agregar un canal

1. Completa el formulario con la información del canal:
   - **Nombre**: Identificador del canal
   - **Tipo de Fuente**: SRT, RTMP, HLS o RTSP
   - **URL RTSP**: URL del stream RTSP de MediaMTX
   - **Dirección UDP**: IP multicast o unicast de destino
   - **Puerto UDP**: Puerto de destino

2. Haz clic en "Agregar Canal"

### 4. Controlar streams

- **Iniciar**: Comienza la captura RTSP y la inyección UDP
- **Detener**: Detiene el stream
- **Eliminar**: Elimina el canal (debe estar detenido)

## Integración con MediaMTX

### Configuración típica de MediaMTX

```yaml
# mediamtx.yml

# Configurar paths para tus streams
paths:
  canal1:
    source: srt://0.0.0.0:8890?streamid=publish:canal1
    
  canal2:
    source: rtmp://0.0.0.0:1935/live/canal2
    
  canal3:
    source: http://stream.example.com/playlist.m3u8
```

Luego en el Orchestrator, usa las URLs RTSP generadas por MediaMTX:
```
rtsp://localhost:8554/canal1
rtsp://localhost:8554/canal2
rtsp://localhost:8554/canal3
```

## API REST

### Endpoints disponibles

#### Listar canales
```bash
GET /api/v1/channels
```

#### Crear canal
```bash
POST /api/v1/channels
Content-Type: application/json

{
  "id": "unique-id",
  "name": "Canal 1",
  "source_type": "srt",
  "rtsp_url": "rtsp://localhost:8554/stream1",
  "udp_address": "239.1.1.1",
  "udp_port": 5000
}
```

#### Obtener estado de un canal
```bash
GET /api/v1/channels/:id/status
```

#### Iniciar stream
```bash
POST /api/v1/channels/:id/start
```

#### Detener stream
```bash
POST /api/v1/channels/:id/stop
```

#### Eliminar canal
```bash
DELETE /api/v1/channels/:id
```

## Estructura del Proyecto

```
mediamtx-orchestrator/
├── cmd/
│   └── server/
│       └── main.go              # Entry point
├── internal/
│   ├── api/
│   │   └── handlers.go          # HTTP handlers
│   └── orchestrator/
│       ├── orchestrator.go      # Lógica principal
│       ├── channel_manager.go   # Gestión de canales
│       ├── rtsp_client.go       # Cliente RTSP
│       └── udp_injector.go      # Inyección UDP
├── web/
│   └── templates/
│       └── index.html           # Interfaz web
├── go.mod
└── README.md
```

## Despliegue en Producción

### Como servicio systemd (Linux)

1. Crear archivo de servicio:

```bash
sudo nano /etc/systemd/system/orchestrator.service
```

2. Contenido:

```ini
[Unit]
Description=MediaMTX Orchestrator
After=network.target mediamtx.service

[Service]
Type=simple
User=orchestrator
WorkingDirectory=/opt/orchestrator
ExecStart=/opt/orchestrator/orchestrator
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

3. Activar e iniciar:

```bash
sudo systemctl daemon-reload
sudo systemctl enable orchestrator
sudo systemctl start orchestrator
```

### Docker (opcional)

```dockerfile
FROM golang:1.21-alpine AS builder
WORKDIR /app
COPY . .
RUN go mod download
RUN go build -ldflags="-s -w" -o orchestrator cmd/server/main.go

FROM alpine:latest
RUN apk --no-cache add ca-certificates
WORKDIR /root/
COPY --from=builder /app/orchestrator .
EXPOSE 8080
CMD ["./orchestrator"]
```

Compilar y ejecutar:
```bash
docker build -t orchestrator .
docker run -p 8080:8080 orchestrator
```

## Troubleshooting

### El stream no inicia

1. Verifica que MediaMTX esté corriendo
2. Comprueba la URL RTSP: `ffplay rtsp://localhost:8554/tu-stream`
3. Revisa los logs del servidor

### Error de conexión UDP

1. Verifica que la dirección IP sea válida
2. Para multicast, asegúrate que tu red lo soporta
3. Comprueba permisos de firewall

### Alto consumo de CPU

- Reduce el número de streams simultáneos
- Verifica que no haya fugas de memoria con `top` o `htop`

## Rendimiento

### Consumo de recursos típico

- **Memoria**: ~30-50 MB base + ~10-20 MB por stream activo
- **CPU**: <5% por stream (depende del bitrate)
- **Red**: Depende del bitrate del stream original

### Optimizaciones

- Binario compilado con `-ldflags="-s -w"` reduce ~40% el tamaño
- Usar `upx --best` puede comprimir aún más
- En producción, limitar número de streams simultáneos

## Licencia

MIT License

## Soporte

Para reportar bugs o solicitar features, crea un issue en el repositorio.

## Changelog

### v1.0.0 (2024)
- Versión inicial
- Gestión básica de canales
- Cliente RTSP con inyección UDP
- Interfaz web con Alpine.js
- API REST completa
