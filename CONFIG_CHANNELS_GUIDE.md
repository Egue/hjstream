# Configuración de Canales - Guía Actualizada

## Modos de Operación

### 1. Modo Pass-Through (Simple)
Solo requiere **input** y **output**. El stream pasa sin modificaciones.

```json
{
  "id": "mi-canal-simple",
  "name": "Mi Canal Simple",
  "enabled": true,
  "mode": "passthrough",
  "input": {
    "type": "srt",
    "url": "srt://192.168.1.100:20581?mode=listener",
    "latency_ms": 200
  },
  "output": {
    "type": "udp",
    "url": "udp://232.2.3.2:1002",
    "local_interface": "192.168.1.1",
    "ttl": 32,
    "packet_size": 1316
  }
}
```

**Ventajas:**
- Configuración mínima
- Menor uso de CPU
- Ideal para cuando solo necesitas reenviar streams

---

### 2. Modo Transcoding (Completo)
Incluye **transcoding** opcional con configuración detallada.

```json
{
  "id": "mi-canal-transcodificado",
  "name": "Mi Canal con Transcoding",
  "enabled": true,
  "mode": "srt_transcoder",
  "input": {
    "type": "srt",
    "url": "srt://181.79.86.130:20581?mode=listener",
    "latency_ms": 200,
    "max_bandwidth_mbps": 20,
    "passphrase": "opcional-clave-encriptacion"
  },
  "output": {
    "type": "udp",
    "url": "udp://232.2.3.2:1002",
    "local_interface": "192.168.2.140",
    "ttl": 32,
    "packet_size": 1316
  },
  "transcoding": {
    "enabled": true,
    "video": {
      "codec": "h264",
      "profile": "main",
      "level": "4.0",
      "bitrate_kbps": 4000,
      "max_bitrate_kbps": 4500,
      "buffer_size_kb": 8000,
      "framerate": 30,
      "gop_size": 60,
      "preset": "medium",
      "tune": "zerolatency",
      "rate_control": "cbr",
      "hardware_acceleration": {
        "enabled": false,
        "type": "nvenc"
      }
    },
    "audio": {
      "codec": "aac",
      "bitrate_kbps": 128,
      "sample_rate": 48000,
      "channels": 2,
      "profile": "aac_low"
    },
    "mpegts": {
      "pmt_pid": 4096,
      "video_pid": 256,
      "audio_pid": 257,
      "pcr_pid": 256,
      "service_id": 1,
      "service_name": "Mi Canal HD",
      "provider_name": "Mi Proveedor"
    },
    "analysis": {
      "auto_detect": true,
      "force_transcode": false,
      "passthrough_if_compatible": true
    }
  },
  "monitoring": {
    "report_interval_seconds": 10,
    "alert_on_error": true,
    "alert_on_bitrate_deviation_percent": 15
  },
  "failover": {
    "auto_restart": true,
    "max_restart_attempts": 3,
    "restart_delay_seconds": 5,
    "backup_input_url": null
  }
}
```

**Ventajas:**
- Control total sobre codificación
- Optimización de bitrate
- Detección automática de compatibilidad
- Recuperación ante fallos

---

## Campos Explicados

### Input (Requerido)
- `type`: Tipo de entrada (srt, udp, tcp, etc.)
- `url`: URL o dirección de conexión
- `latency_ms`: Latencia en milisegundos (SRT)
- `max_bandwidth_mbps`: Límite de ancho de banda (SRT)
- `passphrase`: Contraseña para encriptación (SRT)

### Output (Requerido)
- `type`: Tipo de salida (udp, srt, tcp, etc.)
- `url`: Dirección destino
- `local_interface`: Interfaz local para binding
- `ttl`: Time to live (UDP multicast)
- `packet_size`: Tamaño del paquete

### Transcoding (Opcional)
Solo si necesitas procesar/modificar el stream:

#### Video
- Codec, perfil, nivel
- Bitrate y buffer
- Frame rate y GOP
- Preset y tunning
- Aceleración por hardware

#### Audio
- Codec (aac, mp3, etc.)
- Bitrate y sample rate
- Canales y perfil

#### MPEGTS
- PIDs del stream
- Información del servicio

#### Analysis
- Detección automática
- Forzar transcodificación
- Pass-through si compatible

### Monitoring (Opcional)
- Intervalo de reporte
- Alertas de error
- Desviación de bitrate

### Failover (Opcional)
- Reinicio automático
- Número de intentos
- Delay entre intentos
- URL de backup

---

## Ejemplos de Uso

### Ejemplo 1: Canal Mínimo (Solo Pass-Through)
```json
{
  "id": "canal-minimo",
  "name": "Canal Mínimo",
  "enabled": true,
  "mode": "passthrough",
  "input": {
    "type": "udp",
    "url": "udp://0.0.0.0:5000"
  },
  "output": {
    "type": "udp",
    "url": "udp://232.1.1.1:5001"
  }
}
```

### Ejemplo 2: Canal con Monitoreo pero Sin Transcoding
```json
{
  "id": "canal-monitoreado",
  "name": "Canal Monitoreado",
  "enabled": true,
  "mode": "passthrough",
  "input": {
    "type": "srt",
    "url": "srt://10.0.0.1:20000?mode=listener"
  },
  "output": {
    "type": "udp",
    "url": "udp://232.1.1.1:5001",
    "ttl": 32
  },
  "monitoring": {
    "report_interval_seconds": 30,
    "alert_on_error": true,
    "alert_on_bitrate_deviation_percent": 20
  }
}
```

### Ejemplo 3: Canal Completo con Transcoding
Ver archivo `with_transcoding.json`

---

## Guía de Migración

Si tienes canales existentes, puedes actualizarlos:

**Antes (Transcoding Obligatorio):**
```json
{
  "id": "canal",
  "video": { ... },
  "audio": { ... },
  "mpegts": { ... }
}
```

**Después (Transcoding Opcional):**
```json
{
  "id": "canal",
  "transcoding": {
    "enabled": true,
    "video": { ... },
    "audio": { ... },
    "mpegts": { ... }
  }
}
```

O simplemente elimina la sección de `transcoding` si no la necesitas.
