#!/bin/bash

# Script de actualización del servicio

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Error: Este script debe ejecutarse como root${NC}"
    exit 1
fi

INSTALL_DIR="/opt/hjstream"

echo -e "${BLUE}=== Actualización de CATV Transcoder ===${NC}"
echo ""

# Verificar que está instalado
if [ ! -d "$INSTALL_DIR" ]; then
    echo -e "${RED}Error: CATV Transcoder no está instalado${NC}"
    echo "Ejecuta primero: sudo ./deploy/install.sh"
    exit 1
fi

# Verificar que el nuevo binario existe
if [ ! -f "target/release/hjstream" ]; then
    echo -e "${RED}Error: Binario no encontrado${NC}"
    echo "Ejecuta primero: cargo build --release"
    exit 1
fi

# Backup del binario actual
echo "Haciendo backup del binario actual..."
cp "$INSTALL_DIR/bin/hjstream" "$INSTALL_DIR/bin/hjstream.backup"

# Detener servicio
echo "Deteniendo servicio..."
systemctl stop hjstream

# Actualizar binario
echo "Actualizando binario..."
cp target/release/hjstream "$INSTALL_DIR/bin/"
chmod +x "$INSTALL_DIR/bin/hjstream"
chown transcoder:transcoder "$INSTALL_DIR/bin/hjstream"

# Actualizar scripts si hay cambios
if [ -d "scripts" ]; then
    echo "Actualizando scripts..."
    cp scripts/*.sh "$INSTALL_DIR/scripts/"
    chmod +x "$INSTALL_DIR/scripts/"*.sh
    chown -R transcoder:transcoder "$INSTALL_DIR/scripts"
fi

# Actualizar servicio systemd si cambió
if [ -f "deploy/hjstream.service" ]; then
    if ! diff -q deploy/hjstream.service /etc/systemd/system/hjstream.service > /dev/null 2>&1; then
        echo "Actualizando servicio systemd..."
        cp deploy/hjstream.service /etc/systemd/system/
        systemctl daemon-reload
    fi
fi

# Iniciar servicio
echo "Iniciando servicio..."
systemctl start hjstream

# Esperar un momento
sleep 3

# Verificar estado
if systemctl is-active --quiet hjstream; then
    echo -e "${GREEN}✓ Actualización completada exitosamente${NC}"
    echo ""
    echo "Ver estado:"
    echo "  sudo systemctl status hjstream"
    echo ""
    echo "Ver logs:"
    echo "  sudo journalctl -u hjstream -f"
else
    echo -e "${RED}✗ Error: El servicio no se inició correctamente${NC}"
    echo ""
    echo "Restaurando backup..."
    systemctl stop hjstream
    cp "$INSTALL_DIR/bin/hjstream.backup" "$INSTALL_DIR/bin/hjstream"
    systemctl start hjstream
    echo ""
    echo "Ver logs para diagnóstico:"
    echo "  sudo journalctl -u hjstream -n 50"
    exit 1
fi
```

## 82. `client-rust/deploy/logrotate.conf`
```
# Logrotate configuration for CATV Transcoder

/opt/hjstream/logs/*.log {
    daily
    rotate 7
    compress
    delaycompress
    notifempty
    missingok
    create 0644 transcoder transcoder
    sharedscripts
    
    postrotate
        # Reabrir archivo de log (enviar señal al proceso)
        systemctl reload hjstream > /dev/null 2>&1 || true
    endscript
}