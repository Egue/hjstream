#!/bin/bash

# Script para actualizar el transcoder
set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== Actualizando CATV Transcoder ==="

# Verificar si hay cambios en git
if [ -d "$PROJECT_DIR/.git" ]; then
    echo "Verificando actualizaciones en git..."
    cd "$PROJECT_DIR"
    
    git fetch
    
    LOCAL=$(git rev-parse @)
    REMOTE=$(git rev-parse @{u})
    
    if [ "$LOCAL" = "$REMOTE" ]; then
        echo "Ya estás en la última versión"
    else
        echo -e "${YELLOW}Hay actualizaciones disponibles${NC}"
        
        # Hacer backup de config
        echo "Haciendo backup de configuración..."
        "$SCRIPT_DIR/backup-config.sh"
        
        # Pull cambios
        echo "Descargando actualizaciones..."
        git pull
        
        # Recompilar
        echo "Recompilando..."
        cargo build --release
        
        echo -e "${GREEN}✓ Actualización completada${NC}"
        echo "Ejecuta './scripts/restart.sh' para aplicar cambios"
    fi
else
    echo "No es un repositorio git, actualizando solo el binario..."
    cargo build --release
    echo -e "${GREEN}✓ Binario actualizado${NC}"
fi