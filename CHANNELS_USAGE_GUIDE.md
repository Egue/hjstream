# Guía de Uso - Canales Migrados desde Flussonic

## Resumen Rápido

Se han migrado **50 canales** de Flussonic a hjStream con configuración de **passthrough** (sin transcoding). Todos los canales están listos para usar.

---

## 1. Estructura de Carpetas

```
config/
├── channels/
│   ├── rcn.json                      # Canal RCN (SRT)
│   ├── caracol.json                  # Canal Caracol (SRT)
│   ├── espn.json                     # Canal ESPN (RTMP)
│   ├── canaltrece.json               # Canal Trece (RTMP)
│   ├── ewtn.json                     # EWTN (HLS)
│   └── ... (45 más)
├── client.json                       # Configuración global
└── channels.json                     # Índice de canales
```

---

## 2. Configuración de un Canal

Cada canal tiene esta estructura:

```json
{
  "id": "RCN",
  "name": "RCN",
  "enabled": true,
  "mode": "passthrough",
  "input": {
    "type": "srt",
    "url": "srt://181.79.86.130:20581"
  },
  "output": {
    "type": "udp",
    "url": "udp://232.2.3.2:1002",
    "local_interface": "192.168.2.140",
    "ttl": 32
  },
  "metadata": {
    "source": "Migrado desde Flussonic",
    "notes": "Canal original: RCN"
  }
}
```

### Campos explicados

| Campo | Descripción |
|-------|------------|
| `id` | Identificador único del canal |
| `name` | Nombre legible del canal |
| `enabled` | Si el canal está activo (true/false) |
| `mode` | `passthrough` = sin transcoding; `transcode` = con transcoding |
| `input.type` | Protocolo de entrada: srt, rtmp, hls, http |
| `input.url` | URL completa de la fuente |
| `output.type` | Siempre `udp` (multicast) |
| `output.url` | IP multicast y puerto de salida |
| `output.local_interface` | IP de la interfaz de red local (192.168.2.140) |
| `output.ttl` | Time-To-Live para multicast (32 es estándar) |

---

## 3. Habilitar/Deshabilitar Canales

Para **deshabilitar** un canal, cambie `enabled` a `false`:

```json
{
  "enabled": false,
  ...
}
```

Para **habilitar**, cambie a `true`.

---

## 4. Modificar una Fuente

Si la URL de entrada cambia, actualice `input.url`:

**Antes:**
```json
"input": {
  "type": "srt",
  "url": "srt://181.79.86.130:20581"
}
```

**Después:**
```json
"input": {
  "type": "srt",
  "url": "srt://nueva.ip.address:20581"
}
```

---

## 5. Cambiar Puerto de Salida

Para cambiar el puerto multicast de salida:

**Antes:**
```json
"output": {
  "url": "udp://232.2.3.2:1002",
  ...
}
```

**Después:**
```json
"output": {
  "url": "udp://232.2.3.2:2000",  # Puerto cambiado
  ...
}
```

---

## 6. Listar Todos los Canales

### En terminal (PowerShell)
```powershell
# Listar nombres de canales
Get-ChildItem config/channels/*.json | Where-Object {$_.Name -ne 'flussonic.json'} | ForEach-Object {
  $json = Get-Content $_.FullName | ConvertFrom-Json
  Write-Host "$($json.name) -> $($json.input.type)"
}
```

### Con Python
```bash
python scripts/validate-channels.py
```

---

## 7. Protocolos Disponibles

### Canales SRT (36 total)
Protocolo de baja latencia, muy confiable:
- RCN, CARACOL, ZOOMOO, CITYTV, CNC, TELECAFE, TELECARIBE, TELEPACIFICO
- Y 28 más...

### Canales RTMP (12 total)
Protocolo estándar, requiere más recursos:
- ESPN, CANALTRECE, GOLDENPLUS, PANICO, TNTSERIES
- Y 7 más...

### Canales HLS (2 total)
Protocolo basado en HTTP:
- EWTN, CANALDELASESTRELLAS

---

## 8. Verificar Estado

### Validar configuración
```bash
python scripts/validate-channels.py
```

### Compilar proyecto
```bash
cargo check
```

### Ver resumen de migración
```bash
cat MIGRATION_SUMMARY.md
```

---

## 9. Agregar Transcoding (Opcional)

Si necesita transcoding en algún canal, agregue la sección `transcoding`:

```json
{
  "id": "RCN",
  "enabled": true,
  "mode": "transcode",  # Cambiar a 'transcode'
  "input": { ... },
  "output": { ... },
  "transcoding": {
    "video": {
      "codec": "h264",
      "bitrate": "5000k",
      "width": 1920,
      "height": 1080,
      "fps": 30
    },
    "audio": {
      "codec": "aac",
      "bitrate": "192k",
      "sample_rate": 48000
    }
  },
  "metadata": { ... }
}
```

---

## 10. Tabla de Referencia Rápida

### Canales Principales
| Canal | Protocolo | IP:Puerto | Local Interface |
|-------|-----------|----------|-----------------|
| RCN | SRT | 232.2.3.2:1002 | 192.168.2.140 |
| CARACOL | SRT | 232.2.3.1:1001 | 192.168.2.140 |
| ESPN | RTMP | 232.2.3.18:1018 | 192.168.2.140 |
| CANALTRECE | RTMP | 232.2.3.3:1003 | 192.168.2.140 |
| EWTN | HLS | 232.2.3.42:1042 | 192.168.2.140 |

---

## 11. Troubleshooting

### Canal no conecta
1. Verificar que `input.url` sea correcta
2. Verificar conectividad de red a la fuente
3. Verificar firewall permite protocolo (SRT, RTMP, HLS)

### Sin audio/video en salida
1. Verificar `enabled: true`
2. Verificar `mode: passthrough` o configuración de transcoding
3. Verificar puertos multicast no están bloqueados

### Latencia alta
1. Cambiar a SRT si es posible (más bajo que RTMP)
2. Verificar `ttl: 32` en output
3. Verificar ancho de banda disponible

---

## 12. Actualizar desde Flussonic

Si Flussonic recibe cambios de configuración:

```bash
# Actualizar config de Flussonic
cp /ruta/a/flussonic.json config/channels/flussonic.json

# Regenerar todos los canales
python scripts/convert-flussonic.py

# Validar cambios
python scripts/validate-channels.py

# Compilar si es necesario
cargo check
```

---

## 13. Documentación Adicional

- [MIGRATION_SUMMARY.md](MIGRATION_SUMMARY.md) - Resumen ejecutivo
- [FLUSSONIC_MIGRATION_REPORT.md](FLUSSONIC_MIGRATION_REPORT.md) - Reporte detallado
- [README.md](README.md) - Documentación del proyecto

---

## Contacto

Para soporte técnico:
1. Revisar los 50 canales en `config/channels/`
2. Ejecutar validador: `python scripts/validate-channels.py`
3. Consultar logs del sistema
4. Revisar documentación de hjStream

---

**Todas las 50 canales están listas para operar en modo passthrough.**
