#!/bin/bash
# Script de instalación para hjstreamc CATV-2

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

INSTALL_DIR="/opt/hjstreamc"
SERVICE_NAME="hjstreamc"
USER="streaming"

echo -e "${BLUE}"
echo "=========================================="
echo "  Instalador hjstreamc CATV-2"
echo "=========================================="
echo -e "${NC}"

# Verificar si se ejecuta como root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Por favor ejecuta como root (sudo)${NC}"
    exit 1
fi

# Detectar distribución
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
    VERSION=$VERSION_ID
else
    echo -e "${RED}No se puede detectar la distribución${NC}"
    exit 1
fi

echo -e "${GREEN}Sistema detectado: $OS $VERSION${NC}"
echo ""

# Instalar dependencias según la distribución
install_dependencies() {
    echo -e "${YELLOW}Instalando dependencias...${NC}"
    
    case $OS in
        ubuntu|debian)
            apt-get update
            apt-get install -y build-essential git pkg-config \
                libavformat-dev libavcodec-dev libavutil-dev \
                libswscale-dev libswresample-dev bc
            ;;
        centos|rhel|rocky|almalinux)
            yum install -y gcc gcc-c++ make git pkgconfig \
                ffmpeg-devel bc
            ;;
        fedora)
            dnf install -y gcc gcc-c++ make git pkgconfig \
                ffmpeg-devel bc
            ;;
        *)
            echo -e "${RED}Distribución no soportada: $OS${NC}"
            echo "Instala manualmente: gcc, make, libavformat-dev, libavcodec-dev, libavutil-dev"
            exit 1
            ;;
    esac
    
    echo -e "${GREEN}✓ Dependencias instaladas${NC}"
}

# Crear usuario del sistema
create_user() {
    if id "$USER" &>/dev/null; then
        echo -e "${YELLOW}Usuario $USER ya existe${NC}"
    else
        echo -e "${YELLOW}Creando usuario $USER...${NC}"
        useradd -r -s /bin/false -d $INSTALL_DIR $USER
        echo -e "${GREEN}✓ Usuario creado${NC}"
    fi
}

# Crear directorio de instalación
create_directory() {
    echo -e "${YELLOW}Creando directorio de instalación...${NC}"
    
    mkdir -p $INSTALL_DIR
    mkdir -p $INSTALL_DIR/logs
    
    echo -e "${GREEN}✓ Directorio creado: $INSTALL_DIR${NC}"
}

# Copiar archivos
copy_files() {
    echo -e "${YELLOW}Copiando archivos...${NC}"
    
    # Compilar el programa
    echo "Compilando stream_relay_ts..."
    gcc -O3 -Wall -Wextra -pthread \
-march=native -mtune=native \
-o stream_relay_ts stream_relay_ts.c \
-lavformat -lavcodec -lavutil -lm
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}Error en compilación${NC}"
        exit 1
    fi
    
    # Copiar binario
    cp stream_relay_ts $INSTALL_DIR/
    chmod +x $INSTALL_DIR/stream_relay_ts
    
    # Copiar configuración de ejemplo si no existe
    if [ ! -f "$INSTALL_DIR/config.txt" ]; then
        cp config.txt $INSTALL_DIR/config.txt.example
        cat > $INSTALL_DIR/config.txt << 'EOF'
# Configuración de hjstreamc
# Formato: INPUT_URL OUTPUT_IP OUTPUT_PORT
#
# Ejemplo:
# srt://source:9000 239.1.1.1 5000
# rtmp://server/app/stream 239.1.1.2 5001

EOF
        echo -e "${YELLOW}Edita $INSTALL_DIR/config.txt con tus streams${NC}"
    fi
    
    # Copiar README
    if [ -f "README.md" ]; then
        cp README.md $INSTALL_DIR/
    fi
    
    # Ajustar permisos
    chown -R $USER:$USER $INSTALL_DIR
    
    echo -e "${GREEN}✓ Archivos copiados${NC}"
}

# Instalar servicio systemd
install_service() {
    echo -e "${YELLOW}Instalando servicio systemd...${NC}"
    
    cp hjstreamc.service /etc/systemd/system/$SERVICE_NAME.service
    systemctl daemon-reload
    
    echo -e "${GREEN}✓ Servicio instalado${NC}"
}

# Configurar sistema
configure_system() {
    echo -e "${YELLOW}Configurando sistema...${NC}"
    
    # Aumentar límites
    cat > /etc/security/limits.d/hjstreamc.conf << EOF
$USER soft nofile 65536
$USER hard nofile 65536
$USER soft nproc 4096
$USER hard nproc 4096
EOF
    
    # Optimizar red
    if ! grep -q "net.core.rmem_max" /etc/sysctl.conf; then
        cat >> /etc/sysctl.conf << EOF

# Optimizaciones para hjstreamc
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.core.rmem_default = 16777216
net.core.wmem_default = 16777216
net.ipv4.udp_mem = 8388608 12582912 16777216
EOF
        sysctl -p > /dev/null
    fi
    
    echo -e "${GREEN}✓ Sistema configurado${NC}"
}

# Configurar firewall (opcional)
configure_firewall() {
    echo ""
    read -p "¿Configurar firewall para permitir puertos UDP 5000-5099? (s/n): " config_fw
    
    if [[ $config_fw == "s" || $config_fw == "S" ]]; then
        if command -v ufw &> /dev/null; then
            ufw allow 5000:5099/udp
            echo -e "${GREEN}✓ Firewall configurado (UFW)${NC}"
        elif command -v firewall-cmd &> /dev/null; then
            firewall-cmd --permanent --add-port=5000-5099/udp
            firewall-cmd --reload
            echo -e "${GREEN}✓ Firewall configurado (firewalld)${NC}"
        else
            echo -e "${YELLOW}No se detectó UFW ni firewalld${NC}"
        fi
    fi
}

# Habilitar multicast routing
enable_multicast() {
    echo ""
    read -p "¿Habilitar routing multicast? (s/n): " enable_mc
    
    if [[ $enable_mc == "s" || $enable_mc == "S" ]]; then
        echo "Interfaces de red disponibles:"
        ip link show | grep "^[0-9]" | cut -d: -f2 | tr -d ' '
        echo ""
        read -p "Ingresa la interfaz (ej: eth0): " iface
        
        ip route add 239.0.0.0/8 dev $iface 2>/dev/null || true
        
        # Hacerlo permanente
        if [ ! -f "/etc/network/if-up.d/multicast-route" ]; then
            cat > /etc/network/if-up.d/multicast-route << EOF
#!/bin/sh
ip route add 239.0.0.0/8 dev $iface 2>/dev/null || true
EOF
            chmod +x /etc/network/if-up.d/multicast-route
        fi
        
        echo -e "${GREEN}✓ Multicast configurado en $iface${NC}"
    fi
}

# Resumen final
show_summary() {
    echo ""
    echo -e "${BLUE}=========================================="
    echo "  Instalación completada"
    echo -e "==========================================${NC}"
    echo ""
    echo "Ubicación: $INSTALL_DIR"
    echo "Usuario: $USER"
    echo "Servicio: $SERVICE_NAME"
    echo ""
    echo -e "${YELLOW}Próximos pasos:${NC}"
    echo ""
    echo "1. Editar configuración:"
    echo "   sudo nano $INSTALL_DIR/config.txt"
    echo ""
    echo "2. Habilitar servicio:"
    echo "   sudo systemctl enable $SERVICE_NAME"
    echo ""
    echo "3. Iniciar servicio:"
    echo "   sudo systemctl start $SERVICE_NAME"
    echo ""
    echo "4. Ver estado:"
    echo "   sudo systemctl status $SERVICE_NAME"
    echo ""
    echo "5. Ver logs:"
    echo "   sudo journalctl -u $SERVICE_NAME -f"
    echo ""
    echo "6. Test manual:"
    echo "   cd $INSTALL_DIR"
    echo "   sudo -u $USER ./stream_relay_ts config.txt"
    echo ""
    echo -e "${GREEN}¡Instalación exitosa!${NC}"
    echo ""
}

# Ejecutar instalación
main() {
    install_dependencies
    create_user
    create_directory
    copy_files
    install_service
    configure_system
    configure_firewall
    enable_multicast
    show_summary
}

# Mostrar menú
echo "Este script instalará hjstreamc en: $INSTALL_DIR"
echo ""
read -p "¿Continuar? (s/n): " confirm

if [[ $confirm == "s" || $confirm == "S" ]]; then
    main
else
    echo "Instalación cancelada"
    exit 0
fi
