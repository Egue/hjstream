#!/bin/bash

# Script de configuración del sistema para Multicast Streamer
# Ejecutar con sudo

set -e

echo "=== Configuración del Sistema para Multicast Streamer ==="
echo ""

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Verificar que se ejecuta como root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Error: Este script debe ejecutarse como root${NC}"
    echo "Uso: sudo $0"
    exit 1
fi

# 1. Configurar parámetros de red
echo -e "${YELLOW}[1/5] Configurando parámetros de red...${NC}"

cat > /etc/sysctl.d/99-multicast-streamer.conf << 'EOF'
# Multicast Streamer - Optimización de red
# Aumentar buffers de red para 70+ canales multicast

# Buffers de red generales
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.core.rmem_default = 16777216
net.core.wmem_default = 16777216

# UDP específico
net.ipv4.udp_rmem_min = 8192
net.ipv4.udp_wmem_min = 8192

# Multicast
net.ipv4.igmp_max_memberships = 200

# Queue de red
net.core.netdev_max_backlog = 5000

# Evitar fragmentación IP
net.ipv4.ip_no_pmtu_disc = 0

# TCP tuning (para RTSP)
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216
EOF

sysctl -p /etc/sysctl.d/99-multicast-streamer.conf > /dev/null
echo -e "${GREEN}✓ Parámetros de kernel configurados${NC}"

# 2. Configurar interfaz de red
echo -e "${YELLOW}[2/5] Configurando interfaz eno1...${NC}"

if ip link show eno1 > /dev/null 2>&1; then
    # Intentar aumentar ring buffers (puede fallar si el driver no lo soporta)
    if ethtool -G eno1 rx 4096 tx 4096 2>/dev/null; then
        echo -e "${GREEN}✓ Ring buffers aumentados${NC}"
    else
        echo -e "${YELLOW}⚠ No se pudieron aumentar ring buffers (puede ser normal)${NC}"
    fi
    
    # Desactivar offloading problemático
    ethtool -K eno1 gso off gro off tso off 2>/dev/null || true
    echo -e "${GREEN}✓ Offloading configurado${NC}"
else
    echo -e "${YELLOW}⚠ Interfaz eno1 no encontrada, saltando configuración de NIC${NC}"
fi

# 3. Crear usuario del servicio
echo -e "${YELLOW}[3/5] Configurando usuario streamer...${NC}"

if ! id -u streamer > /dev/null 2>&1; then
    useradd -r -s /bin/false -M streamer
    echo -e "${GREEN}✓ Usuario 'streamer' creado${NC}"
else
    echo -e "${GREEN}✓ Usuario 'streamer' ya existe${NC}"
fi

# 4. Crear directorios
echo -e "${YELLOW}[4/5] Creando directorios...${NC}"

mkdir -p /etc/multicast-streamer
mkdir -p /var/log/multicast-streamer

# Copiar archivo de ejemplo si no existe canales.txt
if [ ! -f /etc/multicast-streamer/canales.txt ]; then
    if [ -f /usr/local/share/multicast-streamer/canales.txt.example ]; then
        cp /usr/local/share/multicast-streamer/canales.txt.example /etc/multicast-streamer/canales.txt
        echo -e "${GREEN}✓ Archivo de configuración de ejemplo copiado${NC}"
    fi
fi

chown -R streamer:streamer /var/log/multicast-streamer
echo -e "${GREEN}✓ Directorios creados${NC}"

# 5. Configurar systemd
echo -e "${YELLOW}[5/5] Configurando systemd...${NC}"

if [ -f /lib/systemd/system/multicast-streamer.service ]; then
    systemctl daemon-reload
    echo -e "${GREEN}✓ Servicio systemd registrado${NC}"
    
    echo ""
    echo -e "${GREEN}=== Configuración completada ===${NC}"
    echo ""
    echo "Próximos pasos:"
    echo "  1. Edita /etc/multicast-streamer/canales.txt con tus canales"
    echo "  2. Verifica que MediaMTX esté corriendo en 127.0.0.1:8554"
    echo "  3. Habilita el servicio: systemctl enable multicast-streamer"
    echo "  4. Inicia el servicio: systemctl start multicast-streamer"
    echo "  5. Revisa logs: journalctl -u multicast-streamer -f"
    echo ""
else
    echo -e "${YELLOW}⚠ Archivo de servicio no encontrado en /lib/systemd/system/${NC}"
    echo "  Ejecuta 'make install' primero para copiar los archivos"
fi

exit 0
