#!/bin/bash

# Pre-start check para systemd

set -e

INSTALL_DIR="/opt/hjstream"

# Verificar que el binario existe
if [ ! -f "$INSTALL_DIR/bin/hjstream" ]; then
    echo "Error: Binario no encontrado"
    exit 1
fi

# Verificar configuración
if [ ! -f "$INSTALL_DIR/.env" ]; then
    echo "Error: Archivo .env no encontrado"
    exit 1
fi

# Verificar FFmpeg
if ! command -v ffmpeg &> /dev/null; then
    echo "Error: FFmpeg no está instalado"
    exit 1
fi

# Crear directorios si no existen
mkdir -p "$INSTALL_DIR/logs"
mkdir -p "$INSTALL_DIR/config/channels"
mkdir -p "$INSTALL_DIR/config/backup"

# Verificar permisos
if [ ! -w "$INSTALL_DIR/logs" ]; then
    echo "Error: No se puede escribir en $INSTALL_DIR/logs"
    exit 1
fi

exit 0