# Cambios de Código Rust - Transcoding Opcional

## ✅ Cambios Completados

### 1. [src/config/loader.rs](src/config/loader.rs)
**Refactorización de `ChannelConfig`**
- `video`, `audio`, `mpegts`, `analysis` ahora están dentro de `TranscodingConfig` (opcional)
- Agregada nueva struct `TranscodingConfig` con campo `enabled: bool`
- `monitoring` y `failover` también son opcionales
- Uso de `#[serde(default)]` para deserialización segura

### 2. [src/core/channel.rs](src/core/channel.rs)
**Detección inteligente de modo**
- Agregado método `is_transcoding_enabled()` que verifica si el transcoding está configurado
- Actualizado `start()` para detectar automáticamente modo pass-through o transcoding
- Lógica condicional: si no hay `transcoding` o está deshabilitado → pass-through
- Si hay `transcoding.enabled = true` → usa estrategia de análisis

### 3. [src/core/strategy.rs](src/core/strategy.rs)
**Manejo robusto de configuración opcional**
- Agregada función helper `is_transcoding_configured()` 
- Actualizado `default_for_config()` para verificar configuración opcional
- Refactorizado `decide_strategy()` para manejar `transcoding.is_some()`
- `is_video_compatible()` retorna `true` si no hay config de video (compatible para pass-through)
- `is_audio_compatible()` retorna `true` si no hay config de audio (compatible para pass-through)
- Actualizado `decide_strategy_with_reasons()` para razonamiento detallado

### 4. [src/core/transcoder.rs](src/core/transcoder.rs)
**Codificación condicionada**
- Actualizado `add_video_encoding_args()`:
  - Si `PassThrough` o `TranscodeAudio` → `-c:v copy`
  - Si no hay config de video → `-c:v copy`
  - Si hay config → aplica encoding normal
- Actualizado `add_audio_encoding_args()`:
  - Si `PassThrough` o `TranscodeVideo` → `-c:a copy`
  - Si no hay config de audio → `-c:a copy`
  - Si hay config → aplica encoding normal

### 5. [src/models/config.rs](src/models/config.rs)
**Validaciones flexibles**
- Actualizado `ConfigValidator::validate_channel_config()` 
- Valida URL input/output siempre (requeridas)
- Solo valida video/audio/mpegts si `transcoding.enabled = true`
- Validaciones anidadas dentro de `if let Some(ref transcoding) = config.transcoding`

---

## 🧪 Guía de Testing

### Test 1: Canal Pass-Through Mínimo
**Archivo:** [config/channels/simple_passthrough.json](config/channels/simple_passthrough.json)

```bash
# Cargar canal
curl -X POST http://localhost:3000/api/channels \
  -H "Content-Type: application/json" \
  -d @config/channels/simple_passthrough.json

# Verificar que está corriendo sin procesar (solo copiando)
curl http://localhost:3000/api/channels/SIMPLE_PASSTHROUGH
```

**Resultado esperado:**
- ✅ Canal inicia sin errores
- ✅ Stats muestran `0% CPU` (no hay transcoding)
- ✅ Bitrate entrada ≈ Bitrate salida

---

### Test 2: Canal Con Transcoding Completo
**Archivo:** [config/channels/with_transcoding.json](config/channels/with_transcoding.json)

```bash
# Cargar canal
curl -X POST http://localhost:3000/api/channels \
  -H "Content-Type: application/json" \
  -d @config/channels/with_transcoding.json

# Verificar que está transcodificando
curl http://localhost:3000/api/channels/WITH_TRANSCODING
```

**Resultado esperado:**
- ✅ Canal inicia con transcoding
- ✅ Stats muestran uso de CPU
- ✅ Bitrate coincide con config (4000 kbps video)

---

### Test 3: Validaciones
```bash
# Intentar crear canal sin input (debe fallar)
curl -X POST http://localhost:3000/api/channels \
  -H "Content-Type: application/json" \
  -d '{"id": "TEST", "name": "Test", "enabled": true, "mode": "passthrough", "output": {"type": "udp", "url": "udp://232.1.1.1:5001"}}'

# Resultado esperado: Error 400 - "La URL de entrada no puede estar vacía"

# Intentar crear canal con transcoding inválido (debe fallar)
curl -X POST http://localhost:3000/api/channels \
  -H "Content-Type: application/json" \
  -d '{"id": "TEST2", "name": "Test2", "enabled": true, "mode": "srt_transcoder", "input": {"type": "srt", "url": "srt://..."}, "output": {"type": "udp", "url": "udp://..."}, "transcoding": {"enabled": true, "video": {"bitrate_kbps": 0}}}'

# Resultado esperado: Error 400 - "El bitrate de video debe ser mayor a 0"
```

---

### Test 4: Backwards Compatibility
Si tienes canales antiguos con estructura antigua (video/audio directamente):

```bash
# Migrar archivo antiguo:
# Antes:
{
  "id": "OLD_CHANNEL",
  "video": { ... },
  "audio": { ... }
}

# Después:
{
  "id": "OLD_CHANNEL",
  "transcoding": {
    "enabled": true,
    "video": { ... },
    "audio": { ... }
  }
}
```

---

## 📊 Matriz de Escenarios

| Escenario | Resultado |
|-----------|-----------|
| Solo input/output | ✅ Pass-through |
| input/output + transcoding.enabled=false | ✅ Pass-through |
| input/output + transcoding.enabled=true + video config | ✅ Transcodifica video |
| input/output + transcoding.enabled=true + audio config | ✅ Transcodifica audio |
| input/output + transcoding.enabled=true + ambos | ✅ Transcodifica ambos |
| Sin input | ❌ Error validación |
| Sin output | ❌ Error validación |
| Transcoding.enabled=true pero sin video ni audio | ✅ Pass-through (compatible) |

---

## 🔧 Cómo Crear Canales Ahora

### Opción 1: Script Interactivo (Recomendado)
```bash
./scripts/create-channel.sh
```

### Opción 2: Copiar Ejemplo
```bash
# Pass-through
cp config/channels/simple_passthrough.json config/channels/mi_canal.json
# Editar según necesidades

# Con transcoding
cp config/channels/with_transcoding.json config/channels/mi_canal.json
# Editar según necesidades
```

### Opción 3: API REST
```bash
curl -X POST http://localhost:3000/api/channels \
  -H "Content-Type: application/json" \
  -d '{
    "id": "mi_canal_simple",
    "name": "Mi Canal Simple",
    "enabled": true,
    "mode": "passthrough",
    "input": {"type": "srt", "url": "srt://..."},
    "output": {"type": "udp", "url": "udp://..."}
  }'
```

---

## 📝 Notas Importantes

1. **Serialización**: Los archivos JSON con `transcoding: null` se deserializan correctamente como `None`
2. **Análisis de stream**: Solo se ejecuta si `transcoding.enabled=true` y `analysis.auto_detect=true`
3. **Pass-through es más eficiente**: Usa mínimo CPU
4. **Compatibilidad**: Código antiguo necesita migración manual a nueva estructura
5. **Validaciones**: Solo validan lo necesario para el modo elegido

---

## 🚀 Compilación

```bash
# Compilar (sin errores, solo warnings)
cargo build --release

# Ejecutar
./target/release/hjstream
```

**Estado:** ✅ Compila exitosamente sin errores
