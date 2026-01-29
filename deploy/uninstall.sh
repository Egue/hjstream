#!/bin/bash

# Script de desinstalación

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Error: Este script debe ejecutarse como root${NC}"
    exit 1
fi

echo -e "${YELLOW}=== Desinstalación de CATV Transcoder ===${NC}"
echo ""
echo -e "${RED}ADVERTENCIA: Esto eliminará el servicio y todos sus datos${NC}"
read -p "¿Continuar? (escriba 'yes' para confirmar): " CONFIRM

if [ "$CONFIRM" != "yes" ]; then
    echo "Cancelado"
    exit 0
fi

INSTALL_DIR="/opt/hjstream"
SERVICE_USER="transcoder"

# Detener y deshabilitar servicio
echo "Deteniendo servicio..."
systemctl stop hjstream 2>/dev/null || true
systemctl disable hjstream 2>/dev/null || true

# Remover archivos de systemd
echo "Removiendo archivos de systemd..."
rm -f /etc/systemd/system/hjstream.service
rm -f /etc/systemd/system/hjstream@.service
rm -f /etc/systemd/system/hjstream.target
systemctl daemon-reload

# Backup de configuración (opcional)
read -p "¿Hacer backup de configuración antes de eliminar? (y/N): " BACKUP
if [ "$BACKUP" = "y" ] || [ "$BACKUP" = "Y" ]; then
    BACKUP_DIR="/tmp/hjstream-backup-$(date +%Y%m%d_%H%M%S)"
    mkdir -p $BACKUP_DIR
    cp -r $INSTALL_DIR/config $BACKUP_DIR/
    cp $INSTALL_DIR/.env $BACKUP_DIR/ 2>/dev/null || true
    echo -e "${GREEN}✓ Backup guardado en: $BACKUP_DIR${NC}"
fi

# Eliminar directorio de instalación
echo "Eliminando archivos..."
rm -rf $INSTALL_DIR

# Eliminar usuario (opcional)
read -p "¿Eliminar usuario $SERVICE_USER? (y/N): " DELETE_USER
if [ "$DELETE_USER" = "y" ] || [ "$DELETE_USER" = "Y" ]; then
    userdel -r $SERVICE_USER 2>/dev/null || true
    echo -e "${GREEN}✓ Usuario eliminado${NC}"
fi

echo ""
echo -e "${GREEN}✓ Desinstalación completada${NC}"