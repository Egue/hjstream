#!/bin/bash
# Script de instalación para MediaMTX Orchestrator en Linux

set -e

APP_NAME="orchestrator"
INSTALL_DIR="/opt/orchestrator"
SERVICE_FILE="/etc/systemd/system/orchestrator.service"
USER="orchestrator"

echo "======================================"
echo "MediaMTX Orchestrator - Instalador"
echo "======================================"
echo ""

# Verificar si se ejecuta como root
if [ "$EUID" -ne 0 ]; then 
    echo "Error: Este script debe ejecutarse como root (usa sudo)"
    exit 1
fi

# Verificar si el binario existe
if [ ! -f "./$APP_NAME" ]; then
    echo "Error: No se encuentra el binario './$APP_NAME'"
    echo "Ejecuta primero: make build"
    exit 1
fi

# Crear usuario si no existe
if ! id "$USER" &>/dev/null; then
    echo "Creando usuario $USER..."
    useradd -r -s /bin/false $USER
fi

# Crear directorio de instalación
echo "Creando directorio $INSTALL_DIR..."
mkdir -p $INSTALL_DIR

# Copiar binario
echo "Copiando binario..."
cp ./$APP_NAME $INSTALL_DIR/
chmod +x $INSTALL_DIR/$APP_NAME

# Copiar archivos de configuración si existen
if [ -d "./config" ]; then
    echo "Copiando configuración..."
    cp -r ./config $INSTALL_DIR/
fi

# Establecer permisos
chown -R $USER:$USER $INSTALL_DIR

# Crear archivo de servicio systemd
echo "Creando servicio systemd..."
cat > $SERVICE_FILE << EOF
[Unit]
Description=MediaMTX Orchestrator
After=network.target
Wants=mediamtx.service

[Service]
Type=simple
User=$USER
Group=$USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/$APP_NAME
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Límites de recursos
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
EOF

# Recargar systemd
echo "Recargando systemd..."
systemctl daemon-reload

echo ""
echo "======================================"
echo "✓ Instalación completada"
echo "======================================"
echo ""
echo "Comandos útiles:"
echo "  Iniciar:         sudo systemctl start orchestrator"
echo "  Detener:         sudo systemctl stop orchestrator"
echo "  Estado:          sudo systemctl status orchestrator"
echo "  Habilitar:       sudo systemctl enable orchestrator"
echo "  Logs:            sudo journalctl -u orchestrator -f"
echo ""
echo "Accede a la interfaz web en: http://localhost:8080"
echo ""

# Preguntar si iniciar el servicio
read -p "¿Deseas iniciar el servicio ahora? (s/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Ss]$ ]]; then
    echo "Iniciando servicio..."
    systemctl start orchestrator
    systemctl status orchestrator --no-pager
    echo ""
    echo "✓ Servicio iniciado"
fi

# Preguntar si habilitar al inicio
read -p "¿Deseas habilitar el servicio al inicio del sistema? (s/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Ss]$ ]]; then
    systemctl enable orchestrator
    echo "✓ Servicio habilitado al inicio"
fi

echo ""
echo "Instalación finalizada con éxito!"
