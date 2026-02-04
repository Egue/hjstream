#!/bin/bash

# Script de instalación de MediaMTX v1.16.0 para ARM64
# Ejecutar con sudo o como root

set -e  # Detener el script si hay algún error

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # Sin color

echo -e "${GREEN}=== Instalador de MediaMTX v1.16.0 ===${NC}"

# Verificar si se ejecuta como root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Por favor ejecuta este script como root o con sudo${NC}"
    exit 1
fi

# Variables
VERSION="v1.16.0"
DOWNLOAD_URL="https://github.com/bluenviron/mediamtx/releases/download/${VERSION}/mediamtx_${VERSION}_linux_arm64.tar.gz"
TEMP_DIR="/tmp/mediamtx_install"
INSTALL_DIR="/usr/local/bin"
SERVICE_FILE="/etc/systemd/system/mediamtx.service"

# Crear directorio temporal
echo -e "${YELLOW}Creando directorio temporal...${NC}"
mkdir -p "$TEMP_DIR"
cd "$TEMP_DIR"

# Descargar MediaMTX
echo -e "${YELLOW}Descargando MediaMTX ${VERSION}...${NC}"
wget -O mediamtx.tar.gz "$DOWNLOAD_URL"

# Descomprimir
echo -e "${YELLOW}Descomprimiendo archivo...${NC}"
tar -xzf mediamtx.tar.gz

# Copiar binario a /usr/local/bin
echo -e "${YELLOW}Instalando binario en ${INSTALL_DIR}...${NC}"
cp mediamtx "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/mediamtx"

# Crear directorio de configuración
echo -e "${YELLOW}Creando directorio de configuración...${NC}"
mkdir -p /etc/mediamtx

# Copiar archivo de configuración si existe
if [ -f "mediamtx.yml" ]; then
    cp mediamtx.yml /etc/mediamtx/
fi

# Crear servicio systemd
echo -e "${YELLOW}Creando servicio systemd...${NC}"
cat > "$SERVICE_FILE" << 'EOF'
[Unit]
Description=MediaMTX RTSP Server
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/mediamtx /etc/mediamtx/mediamtx.yml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

# Recargar systemd
echo -e "${YELLOW}Recargando systemd...${NC}"
systemctl daemon-reload

# Habilitar el servicio
echo -e "${YELLOW}Habilitando servicio para inicio automático...${NC}"
systemctl enable mediamtx.service

# Limpiar archivos temporales
echo -e "${YELLOW}Limpiando archivos temporales...${NC}"
cd /
rm -rf "$TEMP_DIR"

echo -e "${GREEN}=== Instalación completada ===${NC}"
echo -e "${GREEN}MediaMTX ha sido instalado correctamente${NC}"
echo ""
echo -e "Comandos útiles:"
echo -e "  ${YELLOW}Iniciar servicio:${NC}    sudo systemctl start mediamtx"
echo -e "  ${YELLOW}Detener servicio:${NC}    sudo systemctl stop mediamtx"
echo -e "  ${YELLOW}Ver estado:${NC}          sudo systemctl status mediamtx"
echo -e "  ${YELLOW}Ver logs:${NC}            sudo journalctl -u mediamtx -f"
echo -e "  ${YELLOW}Configuración:${NC}       /etc/mediamtx/mediamtx.yml"
echo ""
echo -e "${YELLOW}¿Deseas iniciar el servicio ahora? (s/n)${NC}"
read -r respuesta
if [[ "$respuesta" =~ ^[Ss]$ ]]; then
    systemctl start mediamtx
    echo -e "${GREEN}Servicio iniciado. Verificando estado...${NC}"
    systemctl status mediamtx --no-pager
fi