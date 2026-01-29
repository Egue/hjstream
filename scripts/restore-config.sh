#!/bin/bash

# Script para restaurar configuración desde backup
set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BACKUP_DIR="$PROJECT_DIR/backups"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== Restaurar Configuración ==="

# Listar backups disponibles
if [ ! -d "$BACKUP_DIR" ] || [ -z "$(ls -A $BACKUP_DIR 2>/dev/null)" ]; then
    echo -e "${RED}No hay backups disponibles${NC}"
    exit 1
fi

echo "Backups disponibles:"
ls -lht "$BACKUP_DIR"/*.tar.gz 2>/dev/null | nl

echo ""
read -p "Selecciona el número del backup a restaurar (o 'latest' para el más reciente): " SELECTION

if [ "$SELECTION" = "latest" ]; then
    BACKUP_FILE=$(ls -t "$BACKUP_DIR"/*.tar.gz | head -n1)
else
    BACKUP_FILE=$(ls -t "$BACKUP_DIR"/*.tar.gz | sed -n "${SELECTION}p")
fi

if [ ! -f "$BACKUP_FILE" ]; then
    echo -e "${RED}Backup no encontrado${NC}"
    exit 1
fi

echo "Seleccionado: $(basename $BACKUP_FILE)"
echo ""
echo -e "${YELLOW}ADVERTENCIA: Esto sobrescribirá la configuración actual${NC}"
read -p "¿Continuar? (y/N): " CONFIRM

if [ "$CONFIRM" != "y" ] && [ "$CONFIRM" != "Y" ]; then
    echo "Cancelado"
    exit 0
fi

# Hacer backup de la config actual antes de restaurar
echo "Haciendo backup de seguridad de la configuración actual..."
"$SCRIPT_DIR/backup-config.sh"

# Restaurar
echo "Restaurando configuración..."
cd "$PROJECT_DIR"
tar -xzf "$BACKUP_FILE"

echo -e "${GREEN}✓ Configuración restaurada${NC}"
echo "Reinicia el transcoder para aplicar los cambios"