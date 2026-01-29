#!/bin/bash

# Script para verificar el estado del transcoder
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo "=== Estado del CATV Transcoder Client ==="
echo ""

# Verificar proceso
echo -e "${BLUE}[Proceso]${NC}"
if [ -f "$PROJECT_DIR/transcoder.pid" ]; then
    PID=$(cat "$PROJECT_DIR/transcoder.pid")
    
    if ps -p $PID > /dev/null 2>&1; then
        echo -e "  Estado: ${GREEN}CORRIENDO${NC}"
        echo "  PID: $PID"
        
        # CPU y memoria
        if command -v ps &> /dev/null; then
            CPU=$(ps -p $PID -o %cpu --no-headers)
            MEM=$(ps -p $PID -o %mem --no-headers)
            RSS=$(ps -p $PID -o rss --no-headers)
            
            echo "  CPU: ${CPU}%"
            echo "  Memoria: ${MEM}% ($(($RSS / 1024)) MB)"
        fi
        
        # Tiempo de ejecución
        ELAPSED=$(ps -p $PID -o etime --no-headers | xargs)
        echo "  Uptime: $ELAPSED"
    else
        echo -e "  Estado: ${RED}DETENIDO${NC} (PID file obsoleto)"
    fi
else
    echo -e "  Estado: ${RED}DETENIDO${NC}"
fi

echo ""

# Verificar API local
echo -e "${BLUE}[API Local]${NC}"
SERVER_PORT=${SERVER_PORT:-8080}

if command -v curl &> /dev/null; then
    HEALTH_URL="http://localhost:${SERVER_PORT}/health"
    
    if curl -s -f "$HEALTH_URL" > /dev/null 2>&1; then
        echo -e "  Estado: ${GREEN}DISPONIBLE${NC}"
        echo "  URL: $HEALTH_URL"
        
        # Obtener información de salud
        HEALTH_DATA=$(curl -s "$HEALTH_URL")
        echo "  Info: $HEALTH_DATA" | head -n 5
    else
        echo -e "  Estado: ${RED}NO DISPONIBLE${NC}"
    fi
else
    echo "  (curl no disponible para verificar)"
fi

echo ""

# Verificar canales
echo -e "${BLUE}[Canales]${NC}"
if command -v curl &> /dev/null; then
    CHANNELS_URL="http://localhost:${SERVER_PORT}/channels"
    
    if CHANNELS=$(curl -s -f "$CHANNELS_URL" 2>/dev/null); then
        CHANNEL_COUNT=$(echo "$CHANNELS" | grep -o '"channel_id"' | wc -l)
        echo "  Total: $CHANNEL_COUNT canales"
    else
        echo "  No se pudo obtener información de canales"
    fi
fi

echo ""

# Logs recientes
echo -e "${BLUE}[Logs Recientes]${NC}"
if [ -f "$PROJECT_DIR/logs/transcoder.log" ]; then
    echo "  Últimas 5 líneas:"
    tail -n 5 "$PROJECT_DIR/logs/transcoder.log" | sed 's/^/    /'
    echo ""
    echo "  Ver logs completos: tail -f $PROJECT_DIR/logs/transcoder.log"
else
    echo "  No se encontró archivo de log"
fi

echo ""

# Recursos del sistema
echo -e "${BLUE}[Recursos del Sistema]${NC}"

# CPU
if [ -f /proc/loadavg ]; then
    LOAD=$(cat /proc/loadavg | cut -d' ' -f1-3)
    echo "  Load Average: $LOAD"
fi

# Memoria
if command -v free &> /dev/null; then
    MEM_FREE=$(free -h | grep Mem | awk '{print $4}')
    MEM_TOTAL=$(free -h | grep Mem | awk '{print $2}')
    echo "  Memoria disponible: $MEM_FREE / $MEM_TOTAL"
fi

# Disco
if command -v df &> /dev/null; then
    DISK_USAGE=$(df -h / | tail -1 | awk '{print $5 " usado de " $2}')
    echo "  Disco: $DISK_USAGE"
fi

echo ""