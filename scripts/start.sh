#!/bin/bash

# Script para iniciar el cliente de transcodificación
set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BIN_PATH="$PROJECT_DIR/target/release/catv-transcoder"

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== Iniciando CATV Transcoder Client ==="

# Verificar si el binario existe
if [ ! -f "$BIN_PATH" ]; then
    echo -e "${RED}Error: Binario no encontrado en $BIN_PATH${NC}"
    echo "Ejecuta: ./scripts/build.sh release"
    exit 1
fi

# Verificar si ya está corriendo
if [ -f "$PROJECT_DIR/transcoder.pid" ]; then
    PID=$(cat "$PROJECT_DIR/transcoder.pid")
    if ps -p $PID > /dev/null 2>&1; then
        echo -e "${YELLOW}El transcoder ya está corriendo (PID: $PID)${NC}"
        exit 0
    else
        echo "Limpiando PID file obsoleto..."
        rm "$PROJECT_DIR/transcoder.pid"
    fi
fi

# Crear directorios necesarios
mkdir -p "$PROJECT_DIR/logs"
mkdir -p "$PROJECT_DIR/config/channels"
mkdir -p "$PROJECT_DIR/config/backup"

# Verificar archivo de configuración
if [ ! -f "$PROJECT_DIR/.env" ]; then
    echo -e "${YELLOW}Advertencia: .env no encontrado, copiando desde .env.example${NC}"
    if [ -f "$PROJECT_DIR/.env.example" ]; then
        cp "$PROJECT_DIR/.env.example" "$PROJECT_DIR/.env"
        echo "Por favor edita .env con tu configuración"
    else
        echo -e "${RED}Error: .env.example no encontrado${NC}"
        exit 1
    fi
fi

# Cargar variables de entorno
if [ -f "$PROJECT_DIR/.env" ]; then
    export $(cat "$PROJECT_DIR/.env" | grep -v '^#' | xargs)
fi

# Verificar dependencias del sistema
echo "Verificando dependencias..."

if ! command -v ffmpeg &> /dev/null; then
    echo -e "${RED}Error: FFmpeg no está instalado${NC}"
    echo "Instala con: sudo apt-get install ffmpeg"
    exit 1
fi

echo -e "${GREEN}✓ FFmpeg disponible: $(ffmpeg -version | head -n1)${NC}"

# Modo de ejecución
MODE=${1:-foreground}

case $MODE in
    foreground|fg)
        echo "Iniciando en modo foreground..."
        cd "$PROJECT_DIR"
        exec "$BIN_PATH"
        ;;
    
    background|bg|daemon)
        echo "Iniciando en modo background..."
        cd "$PROJECT_DIR"
        
        # Redirigir output a log
        nohup "$BIN_PATH" > "$PROJECT_DIR/logs/transcoder.log" 2>&1 &
        PID=$!
        
        # Guardar PID
        echo $PID > "$PROJECT_DIR/transcoder.pid"
        
        # Esperar un momento para verificar que inició correctamente
        sleep 2
        
        if ps -p $PID > /dev/null; then
            echo -e "${GREEN}✓ Transcoder iniciado exitosamente (PID: $PID)${NC}"
            echo "Logs: tail -f $PROJECT_DIR/logs/transcoder.log"
        else
            echo -e "${RED}Error: El proceso terminó inesperadamente${NC}"
            echo "Revisa los logs: tail $PROJECT_DIR/logs/transcoder.log"
            rm "$PROJECT_DIR/transcoder.pid"
            exit 1
        fi
        ;;
    
    systemd)
        echo "Iniciando via systemd..."
        sudo systemctl start catv-transcoder
        sudo systemctl status catv-transcoder
        ;;
    
    *)
        echo "Uso: $0 [foreground|background|systemd]"
        echo "  foreground - Ejecutar en primer plano (default)"
        echo "  background - Ejecutar en segundo plano"
        echo "  systemd    - Iniciar via systemd"
        exit 1
        ;;
esac