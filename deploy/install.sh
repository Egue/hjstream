#!/bin/bash

# Script de instalación para deployment nativo

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Verificar que se ejecuta como root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Error: Este script debe ejecutarse como root${NC}"
    echo "Uso: sudo ./deploy/install.sh"
    exit 1
fi

echo -e "${BLUE}=== Instalación de CATV Transcoder ===${NC}"
echo ""

# Variables
INSTALL_DIR="/opt/hjstream"
BIN_DIR="$INSTALL_DIR/bin"
CONFIG_DIR="$INSTALL_DIR/config"
LOGS_DIR="$INSTALL_DIR/logs"
SERVICE_USER="transcoder"
SERVICE_GROUP="transcoder"

# Paso 1: Crear usuario del sistema
echo -e "${BLUE}[1/8]${NC} Creando usuario del sistema..."
if ! id -u $SERVICE_USER > /dev/null 2>&1; then
    useradd -r -s /bin/bash -d $INSTALL_DIR -m $SERVICE_USER
    echo -e "${GREEN}✓ Usuario $SERVICE_USER creado${NC}"
else
    echo -e "${YELLOW}⚠ Usuario $SERVICE_USER ya existe${NC}"
fi

# Paso 2: Crear directorios
echo -e "${BLUE}[2/8]${NC} Creando directorios..."
mkdir -p $BIN_DIR
mkdir -p $CONFIG_DIR/channels
mkdir -p $CONFIG_DIR/backup
mkdir -p $LOGS_DIR
mkdir -p $INSTALL_DIR/scripts
echo -e "${GREEN}✓ Directorios creados${NC}"

# Paso 3: Copiar binario
echo -e "${BLUE}[3/8]${NC} Instalando binario..."
if [ ! -f "target/release/hjstream" ]; then
    echo -e "${RED}Error: Binario no encontrado. Ejecuta primero: cargo build --release${NC}"
    exit 1
fi

cp target/release/hjstream $BIN_DIR/
chmod +x $BIN_DIR/hjstream
echo -e "${GREEN}✓ Binario instalado en $BIN_DIR${NC}"

# Paso 4: Copiar configuración
echo -e "${BLUE}[4/8]${NC} Copiando configuración..."

# .env
if [ ! -f "$INSTALL_DIR/.env" ]; then
    if [ -f ".env" ]; then
        cp .env $INSTALL_DIR/.env
    else
        cp .env.example $INSTALL_DIR/.env
        echo -e "${YELLOW}⚠ Creado .env desde example, edita la configuración${NC}"
    fi
else
    echo -e "${YELLOW}⚠ .env ya existe, no se sobrescribirá${NC}"
fi

# client.json
if [ -f "config/client.json" ]; then
    cp config/client.json $CONFIG_DIR/
fi

# Templates
cp -r config/templates $CONFIG_DIR/ 2>/dev/null || true

echo -e "${GREEN}✓ Configuración copiada${NC}"

# Paso 5: Copiar scripts
echo -e "${BLUE}[5/8]${NC} Copiando scripts..."
cp scripts/*.sh $INSTALL_DIR/scripts/
chmod +x $INSTALL_DIR/scripts/*.sh
echo -e "${GREEN}✓ Scripts copiados${NC}"

# Paso 6: Establecer permisos
echo -e "${BLUE}[6/8]${NC} Configurando permisos..."
chown -R $SERVICE_USER:$SERVICE_GROUP $INSTALL_DIR
chmod 755 $BIN_DIR
chmod 755 $CONFIG_DIR
chmod 755 $LOGS_DIR
chmod 600 $INSTALL_DIR/.env
echo -e "${GREEN}✓ Permisos configurados${NC}"

# Paso 7: Instalar servicio systemd
echo -e "${BLUE}[7/8]${NC} Instalando servicio systemd..."
cp deploy/hjstream.service /etc/systemd/system/
cp deploy/hjstream@.service /etc/systemd/system/ 2>/dev/null || true
cp deploy/hjstream.target /etc/systemd/system/ 2>/dev/null || true

# Copiar scripts de lifecycle
cp deploy/pre-start-check.sh $INSTALL_DIR/scripts/
cp deploy/post-stop-cleanup.sh $INSTALL_DIR/scripts/
chmod +x $INSTALL_DIR/scripts/pre-start-check.sh
chmod +x $INSTALL_DIR/scripts/post-stop-cleanup.sh

systemctl daemon-reload
echo -e "${GREEN}✓ Servicio systemd instalado${NC}"

# Paso 8: Verificar dependencias
echo -e "${BLUE}[8/8]${NC} Verificando dependencias..."
DEPS_OK=true

if ! command -v ffmpeg &> /dev/null; then
    echo -e "${RED}✗ FFmpeg no está instalado${NC}"
    DEPS_OK=false
else
    echo -e "${GREEN}✓ FFmpeg disponible: $(ffmpeg -version | head -n1)${NC}"
fi

if ! command -v ffprobe &> /dev/null; then
    echo -e "${RED}✗ FFprobe no está instalado${NC}"
    DEPS_OK=false
else
    echo -e "${GREEN}✓ FFprobe disponible${NC}"
fi

if [ "$DEPS_OK" = false ]; then
    echo ""
    echo -e "${YELLOW}Instala dependencias faltantes:${NC}"
    echo "  Ubuntu/Debian: sudo apt-get install ffmpeg"
    echo "  CentOS/RHEL:   sudo yum install ffmpeg"
fi

# Resumen
echo ""
echo -e "${GREEN}=== Instalación Completada ===${NC}"
echo ""
echo "Instalado en: $INSTALL_DIR"
echo "Usuario:      $SERVICE_USER"
echo ""
echo -e "${BLUE}Próximos pasos:${NC}"
echo ""
echo "1. Editar configuración:"
echo "   sudo nano $INSTALL_DIR/.env"
echo ""
echo "2. Habilitar servicio:"
echo "   sudo systemctl enable hjstream"
echo ""
echo "3. Iniciar servicio:"
echo "   sudo systemctl start hjstream"
echo ""
echo "4. Ver estado:"
echo "   sudo systemctl status hjstream"
echo ""
echo "5. Ver logs:"
echo "   sudo journalctl -u hjstream -f"
echo ""
echo -e "${YELLOW}Nota: Recuerda configurar los canales en:${NC}"
echo "      $CONFIG_DIR/channels/"
echo ""