#!/bin/bash

# Script para ver logs
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
LOG_FILE="$PROJECT_DIR/logs/transcoder.log"

MODE=${1:-tail}

case $MODE in
    tail|follow|f)
        if [ -f "$LOG_FILE" ]; then
            tail -f "$LOG_FILE"
        else
            echo "Log file no encontrado: $LOG_FILE"
            exit 1
        fi
        ;;
    
    head)
        if [ -f "$LOG_FILE" ]; then
            head -n ${2:-50} "$LOG_FILE"
        else
            echo "Log file no encontrado: $LOG_FILE"
            exit 1
        fi
        ;;
    
    less)
        if [ -f "$LOG_FILE" ]; then
            less "$LOG_FILE"
        else
            echo "Log file no encontrado: $LOG_FILE"
            exit 1
        fi
        ;;
    
    errors)
        if [ -f "$LOG_FILE" ]; then
            grep -i "error\|fatal\|critical" "$LOG_FILE" | tail -n ${2:-50}
        else
            echo "Log file no encontrado: $LOG_FILE"
            exit 1
        fi
        ;;
    
    clear)
        if [ -f "$LOG_FILE" ]; then
            > "$LOG_FILE"
            echo "Log limpiado"
        else
            echo "Log file no encontrado"
        fi
        ;;
    
    *)
        echo "Uso: $0 [tail|head|less|errors|clear]"
        echo "  tail   - Ver últimas líneas en tiempo real (default)"
        echo "  head   - Ver primeras líneas"
        echo "  less   - Abrir con less"
        echo "  errors - Mostrar solo errores"
        echo "  clear  - Limpiar log"
        exit 1
        ;;
esac