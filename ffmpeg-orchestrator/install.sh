#!/bin/bash

set -e

echo "========================================="
echo "hjstream - Installation Script"
echo "========================================="
echo ""

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Función para imprimir mensajes
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Verificar que se ejecuta como root
if [ "$EUID" -ne 0 ]; then 
    print_error "Este script debe ejecutarse como root (sudo)"
    exit 1
fi

# 1. Instalar dependencias
print_info "Instalando dependencias del sistema..."
apt update
apt install -y ffmpeg curl build-essential pkg-config libssl-dev

# 2. Instalar Rust si no está instalado
if ! command -v cargo &> /dev/null; then
    print_info "Instalando Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
else
    print_info "Rust ya está instalado"
fi

# 3. Compilar el proyecto
print_info "Compilando hjstream..."
cargo build --release

# 4. Copiar el binario
print_info "Instalando binario en /usr/local/bin..."
cp target/release/hjstream /usr/local/bin/
chmod +x /usr/local/bin/hjstream

# 5. Crear directorios necesarios
print_info "Creando directorios de configuración..."
mkdir -p /etc/hjstream
mkdir -p /var/log/hjstream
mkdir -p /opt/hjstream

# 6. Establecer permisos
print_info "Configurando permisos..."
chmod 755 /etc/hjstream
chmod 755 /var/log/hjstream
chmod 755 /opt/hjstream

# 7. Copiar archivo de servicio systemd
print_info "Instalando servicio systemd..."
cp hjstream.service /etc/systemd/system/
systemctl daemon-reload

# 8. Preguntar si se debe iniciar el servicio
read -p "¿Desea habilitar e iniciar el servicio ahora? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    print_info "Habilitando e iniciando el servicio..."
    systemctl enable hjstream
    systemctl start hjstream
    sleep 2
    systemctl status hjstream
fi

echo ""
echo "========================================="
echo "Instalación completada exitosamente!"
echo "========================================="
echo ""
print_info "Comandos útiles:"
echo "  - Iniciar servicio:    sudo systemctl start hjstream"
echo "  - Detener servicio:    sudo systemctl stop hjstream"
echo "  - Estado del servicio: sudo systemctl status hjstream"
echo "  - Ver logs:            sudo journalctl -u hjstream -f"
echo ""
print_info "API disponible en: http://localhost:31337"
print_info "Health check:      curl http://localhost:31337/health"
echo ""
