#!/bin/bash

# Script para detener el cliente de transcodificación
set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== Deteniendo CATV Transcoder Client ==="

MODE=${1:-pid}

stop_by_pid() {
    if [ ! -f "$PROJECT_DIR/transcoder.pid" ]; then
        echo -e "${YELLOW}No se encontró archivo PID${NC}"
        return 1
    fi
    
    PID=$(cat "$PROJECT_DIR/transcoder.pid")
    
    if ! ps -p $PID > /dev/null 2>&1; then
        echo -e "${YELLOW}El proceso (PID: $PID) no está corriendo${NC}"
        rm "$PROJECT_DIR/transcoder.pid"
        return 0
    fi
    
    echo "Enviando señal SIGTERM al proceso $PID..."
    kill -TERM $PID
    
    # Esperar hasta 10 segundos para que termine gracefully
    for i in {1..10}; do
        if ! ps -p $PID > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Proceso detenido correctamente${NC}"
            rm "$PROJECT_DIR/transcoder.pid"
            return 0
        fi
        sleep 1
    done
    
    # Si no terminó, forzar
    echo -e "${YELLOW}Proceso no responde, forzando terminación...${NC}"
    kill -KILL $PID
    sleep 1
    
    if ! ps -p $PID > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Proceso forzado a terminar${NC}"
        rm "$PROJECT_DIR/transcoder.pid"
        return 0
    else
        echo -e "${RED}Error: No se pudo detener el proceso${NC}"
        return 1
    fi
}

stop_by_name() {
    echo "Buscando procesos por nombre..."
    PIDS=$(pgrep -f "hjstream" || true)
    
    if [ -z "$PIDS" ]; then
        echo -e "${YELLOW}No se encontraron procesos corriendo${NC}"
        return 0
    fi
    
    echo "Procesos encontrados: $PIDS"
    
    for PID in $PIDS; do
        echo "Deteniendo proceso $PID..."
        kill -TERM $PID
    done
    
    sleep 2
    
    # Verificar si terminaron
    REMAINING=$(pgrep -f "hjstream" || true)
    if [ -z "$REMAINING" ]; then
        echo -e "${GREEN}✓ Todos los procesos detenidos${NC}"
        return 0
    else
        echo -e "${YELLOW}Forzando terminación de procesos restantes...${NC}"
        killall -9 hjstream 2>/dev/null || true
        echo -e "${GREEN}✓ Procesos terminados${NC}"
    fi
}

case $MODE in
    pid)
        stop_by_pid || stop_by_name
        ;;
    
    name)
        stop_by_name
        ;;
    
    systemd)
        echo "Deteniendo via systemd..."
        sudo systemctl stop hjstream
        echo -e "${GREEN}✓ Servicio detenido${NC}"
        ;;
    
    all)
        echo "Deteniendo todos los métodos..."
        stop_by_pid || true
        stop_by_name || true
        ;;
    
    *)
        echo "Uso: $0 [pid|name|systemd|all]"
        echo "  pid     - Detener usando archivo PID (default)"
        echo "  name    - Detener buscando por nombre de proceso"
        echo "  systemd - Detener via systemd"
        echo "  all     - Intentar todos los métodos"
        exit 1
        ;;
esac