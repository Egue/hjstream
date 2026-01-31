#!/bin/bash

# Script para crear canales fácilmente
# Uso: ./scripts/create-channel.sh

set -e

CHANNELS_DIR="config/channels"

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   HJStream - Channel Creator            ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════╝${NC}"
echo ""

# Función para solicitar entrada
read_input() {
    local prompt=$1
    local default=$2
    local input
    
    if [ -z "$default" ]; then
        read -p "$(echo -e ${YELLOW}${prompt}${NC}): " input
    else
        read -p "$(echo -e ${YELLOW}${prompt}${NC}) [${default}]: " input
        input=${input:-$default}
    fi
    echo "$input"
}

# Preguntar tipo de canal
echo -e "${BLUE}¿Qué tipo de canal deseas crear?${NC}"
echo "1) Pass-Through (solo input/output, sin transcoding)"
echo "2) Con Transcoding (input/output + configuración de video/audio)"
echo ""
CHANNEL_TYPE=$(read_input "Selecciona tipo (1 o 2)" "1")

# Información básica
CHANNEL_ID=$(read_input "ID del canal (ej: RCN_HD)" "CANAL_001")
CHANNEL_NAME=$(read_input "Nombre del canal" "Mi Canal")

# Input
echo ""
echo -e "${BLUE}Configuración de ENTRADA${NC}"
INPUT_TYPE=$(read_input "Tipo input (srt/udp/tcp)" "srt")
INPUT_URL=$(read_input "URL input" "srt://192.168.1.100:20581?mode=listener")
LATENCY=$(read_input "Latencia en ms (opcional)" "200")

# Output
echo ""
echo -e "${BLUE}Configuración de SALIDA${NC}"
OUTPUT_TYPE=$(read_input "Tipo output (srt/udp/tcp)" "udp")
OUTPUT_URL=$(read_input "URL output" "udp://232.1.1.1:5001")
LOCAL_IF=$(read_input "Interfaz local (opcional)" "")
TTL=$(read_input "TTL para multicast (opcional)" "32")

# Crear archivo JSON
CHANNEL_FILE="${CHANNELS_DIR}/${CHANNEL_ID}.json"

if [ "$CHANNEL_TYPE" == "1" ]; then
    # Pass-Through simple
    cat > "$CHANNEL_FILE" << EOF
{
  "id": "${CHANNEL_ID}",
  "name": "${CHANNEL_NAME}",
  "enabled": true,
  "mode": "passthrough",
  "input": {
    "type": "${INPUT_TYPE}",
    "url": "${INPUT_URL}"$([ -n "$LATENCY" ] && echo "," || echo "")
$([ -n "$LATENCY" ] && echo "    \"latency_ms\": ${LATENCY}" || echo "")
  },
  "output": {
    "type": "${OUTPUT_TYPE}",
    "url": "${OUTPUT_URL}"$([ -n "$LOCAL_IF" ] || [ -n "$TTL" ] && echo "," || echo "")
$([ -n "$LOCAL_IF" ] && echo "    \"local_interface\": \"${LOCAL_IF}\"," || echo "")
$([ -n "$TTL" ] && echo "    \"ttl\": ${TTL}" || echo "")
  }
}
EOF

    echo ""
    echo -e "${GREEN}✓ Canal Pass-Through creado${NC}"

else
    # Con Transcoding
    echo ""
    echo -e "${BLUE}Configuración de VIDEO (transcoding)${NC}"
    VIDEO_CODEC=$(read_input "Codec video (h264/h265)" "h264")
    VIDEO_BITRATE=$(read_input "Bitrate video (kbps)" "4000")
    VIDEO_FPS=$(read_input "Framerate" "30")
    
    echo ""
    echo -e "${BLUE}Configuración de AUDIO (transcoding)${NC}"
    AUDIO_CODEC=$(read_input "Codec audio (aac/mp3)" "aac")
    AUDIO_BITRATE=$(read_input "Bitrate audio (kbps)" "128")
    AUDIO_SAMPLE=$(read_input "Sample rate (48000/44100)" "48000")
    
    cat > "$CHANNEL_FILE" << EOF
{
  "id": "${CHANNEL_ID}",
  "name": "${CHANNEL_NAME}",
  "enabled": true,
  "mode": "srt_transcoder",
  "input": {
    "type": "${INPUT_TYPE}",
    "url": "${INPUT_URL}"$([ -n "$LATENCY" ] && echo "," || echo "")
$([ -n "$LATENCY" ] && echo "    \"latency_ms\": ${LATENCY}" || echo "")
  },
  "output": {
    "type": "${OUTPUT_TYPE}",
    "url": "${OUTPUT_URL}"$([ -n "$LOCAL_IF" ] || [ -n "$TTL" ] && echo "," || echo "")
$([ -n "$LOCAL_IF" ] && echo "    \"local_interface\": \"${LOCAL_IF}\"," || echo "")
$([ -n "$TTL" ] && echo "    \"ttl\": ${TTL}" || echo "")
  },
  "transcoding": {
    "enabled": true,
    "video": {
      "codec": "${VIDEO_CODEC}",
      "profile": "main",
      "level": "4.0",
      "bitrate_kbps": ${VIDEO_BITRATE},
      "max_bitrate_kbps": $((VIDEO_BITRATE + 500)),
      "buffer_size_kb": 8000,
      "framerate": ${VIDEO_FPS},
      "gop_size": $((VIDEO_FPS * 2)),
      "preset": "medium",
      "tune": "zerolatency",
      "rate_control": "cbr",
      "hardware_acceleration": {
        "enabled": false,
        "type": "nvenc"
      }
    },
    "audio": {
      "codec": "${AUDIO_CODEC}",
      "bitrate_kbps": ${AUDIO_BITRATE},
      "sample_rate": ${AUDIO_SAMPLE},
      "channels": 2,
      "profile": "aac_low"
    },
    "mpegts": {
      "pmt_pid": 4096,
      "video_pid": 256,
      "audio_pid": 257,
      "pcr_pid": 256,
      "service_id": 1,
      "service_name": "${CHANNEL_NAME}",
      "provider_name": "CATV Provider"
    },
    "analysis": {
      "auto_detect": true,
      "force_transcode": false,
      "passthrough_if_compatible": true
    }
  }
}
EOF

    echo ""
    echo -e "${GREEN}✓ Canal con Transcoding creado${NC}"
fi

echo -e "${GREEN}✓ Archivo guardado: ${CHANNEL_FILE}${NC}"
echo ""
echo -e "${YELLOW}Próximos pasos:${NC}"
echo "1. Verifica la configuración: cat ${CHANNEL_FILE}"
echo "2. Reinicia el servicio: ./scripts/restart.sh"
echo "3. Verifica el estado: ./scripts/status.sh"
echo ""
