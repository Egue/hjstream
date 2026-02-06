#!/bin/bash
# Script de desinstalación para MediaMTX Orchestrator

set -e

APP_NAME="orchestrator"
INSTALL_DIR="/opt/orchestrator"
SERVICE_FILE="/etc/systemd/system/orchestrator.service"
USER="orchestrator"

echo "======================================"
echo "MediaMTX Orchestrator - Desinstalador"
echo "======================================"
echo ""

# Verificar si se ejecuta como root
if [ "$EUID" -ne 0 ]; then 
    echo "Error: Este script debe ejecutarse como root (usa sudo)"
    exit 1
fi

# Confirmar desinstalación
read -p "¿Estás seguro de que deseas desinstalar el Orchestrator? (s/n): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Ss]$ ]]; then
    echo "Desinstalación cancelada"
    exit 0
fi

# Detener servicio si está corriendo
if systemctl is-active --quiet orchestrator; then
    echo "Deteniendo servicio..."
    systemctl stop orchestrator
fi

# Deshabilitar servicio si está habilitado
if systemctl is-enabled --quiet orchestrator; then
    echo "Deshabilitando servicio..."
    systemctl disable orchestrator
fi

# Eliminar archivo de servicio
if [ -f "$SERVICE_FILE" ]; then
    echo "Eliminando servicio systemd..."
    rm -f $SERVICE_FILE
    systemctl daemon-reload
fi

# Eliminar directorio de instalación
if [ -d "$INSTALL_DIR" ]; then
    echo "Eliminando archivos de instalación..."
    rm -rf $INSTALL_DIR
fi

# Preguntar si eliminar usuario
read -p "¿Deseas eliminar el usuario $USER? (s/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Ss]$ ]]; then
    if id "$USER" &>/dev/null; then
        echo "Eliminando usuario $USER..."
        userdel $USER
    fi
fi

echo ""
echo "======================================"
echo "✓ Desinstalación completada"
echo "======================================"
echo ""
