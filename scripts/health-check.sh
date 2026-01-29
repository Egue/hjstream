#!/bin/bash

# Health check script para uso en monitoreo externo
# Retorna 0 si está saludable, 1 si no

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
SERVER_PORT=${SERVER_PORT:-8080}

# Verificar proceso
if [ -f "$PROJECT_DIR/transcoder.pid" ]; then
    PID=$(cat "$PROJECT_DIR/transcoder.pid")
    
    if ! ps -p $PID > /dev/null 2>&1; then
        echo "ERROR: Proceso no está corriendo"
        exit 1
    fi
else
    echo "ERROR: PID file no encontrado"
    exit 1
fi

# Verificar API
if command -v curl &> /dev/null; then
    HEALTH_URL="http://localhost:${SERVER_PORT}/health"
    
    if ! curl -s -f --max-time 5 "$HEALTH_URL" > /dev/null 2>&1; then
        echo "ERROR: API no responde"
        exit 1
    fi
else
    echo "WARNING: curl no disponible, no se puede verificar API"
fi

# Verificar FFmpeg
if ! command -v ffmpeg &> /dev/null; then
    echo "ERROR: FFmpeg no está disponible"
    exit 1
fi

echo "OK: Sistema saludable"
exit 0