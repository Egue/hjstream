#!/bin/bash

# Script para hacer backup de la configuración
set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_DIR="$PROJECT_DIR/config"
BACKUP_DIR="$PROJECT_DIR/backups"

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="config_backup_$TIMESTAMP.tar.gz"

GREEN='\033[0;32m'
NC='\033[0m'

echo "=== Backup de Configuración ==="

# Crear directorio de backups
mkdir -p "$BACKUP_DIR"

# Crear archivo tar
echo "Creando backup..."
cd "$PROJECT_DIR"

tar -czf "$BACKUP_DIR/$BACKUP_NAME" \
    config/ \
    .env 2>/dev/null || true

# Verificar
if [ -f "$BACKUP_DIR/$BACKUP_NAME" ]; then
    SIZE=$(du -h "$BACKUP_DIR/$BACKUP_NAME" | cut -f1)
    echo -e "${GREEN}✓ Backup creado: $BACKUP_NAME ($SIZE)${NC}"
    echo "  Ubicación: $BACKUP_DIR/$BACKUP_NAME"
    
    # Mantener solo los últimos 10 backups
    echo "Limpiando backups antiguos..."
    cd "$BACKUP_DIR"
    ls -t config_backup_*.tar.gz | tail -n +11 | xargs -r rm
    
    TOTAL=$(ls -1 config_backup_*.tar.gz | wc -l)
    echo "  Backups actuales: $TOTAL"
else
    echo "Error: No se pudo crear el backup"
    exit 1
fi