#!/bin/bash

# Script para reiniciar el cliente de transcodificación
set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

GREEN='\033[0;32m'
NC='\033[0m'

echo "=== Reiniciando CATV Transcoder Client ==="

MODE=${1:-background}

# Detener
"$SCRIPT_DIR/stop.sh" ${2:-pid}

# Esperar un momento
sleep 2

# Iniciar
"$SCRIPT_DIR/start.sh" $MODE

echo -e "${GREEN}✓ Reinicio completado${NC}"