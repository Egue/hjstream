#!/bin/bash

# Script para instalar dependencias del sistema
# Ejecutar con: sudo ./scripts/install-deps.sh

set -e

echo "=== Instalando dependencias del sistema para CATV Transcoder ==="

# Detectar distribución
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
else
    echo "No se pudo detectar el sistema operativo"
    exit 1
fi

case $OS in
    ubuntu|debian)
        echo "Instalando dependencias para Ubuntu/Debian..."
        
        apt-get update
        
        # FFmpeg y herramientas multimedia
        apt-get install -y \
            ffmpeg \
            libavcodec-dev \
            libavformat-dev \
            libavutil-dev \
            libswscale-dev \
            libavfilter-dev
        
        # Herramientas de red
        apt-get install -y \
            iproute2 \
            net-tools \
            iperf3
        
        # Librerías de desarrollo
        apt-get install -y \
            build-essential \
            pkg-config \
            libssl-dev \
            cmake
        
        # SRT (opcional - para soporte nativo)
        # apt-get install -y libsrt-dev
        
        # Hardware encoding (opcional)
        # NVIDIA: apt-get install -y nvidia-cuda-toolkit
        # Intel: apt-get install -y intel-media-va-driver
        
        echo "Dependencias instaladas correctamente"
        ;;
        
    centos|rhel|fedora)
        echo "Instalando dependencias para CentOS/RHEL/Fedora..."
        
        # Habilitar EPEL y RPM Fusion
        if [ "$OS" = "centos" ] || [ "$OS" = "rhel" ]; then
            yum install -y epel-release
            yum install -y https://download1.rpmfusion.org/free/el/rpmfusion-free-release-8.noarch.rpm
        fi
        
        # FFmpeg
        yum install -y ffmpeg ffmpeg-devel
        
        # Herramientas
        yum install -y \
            iproute \
            net-tools \
            iperf3 \
            gcc \
            gcc-c++ \
            make \
            openssl-devel \
            cmake
        
        echo "Dependencias instaladas correctamente"
        ;;
        
    *)
        echo "Distribución no soportada: $OS"
        echo "Por favor instala manualmente:"
        echo "  - FFmpeg (ffmpeg, ffprobe)"
        echo "  - Rust (curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)"
        echo "  - build-essential / development tools"
        exit 1
        ;;
esac

# Verificar instalaciones
echo ""
echo "=== Verificando instalaciones ==="

if command -v ffmpeg &> /dev/null; then
    echo "✓ FFmpeg: $(ffmpeg -version | head -n1)"
else
    echo "✗ FFmpeg no encontrado"
fi

if command -v cargo &> /dev/null; then
    echo "✓ Rust: $(rustc --version)"
    echo "✓ Cargo: $(cargo --version)"
else
    echo "✗ Rust/Cargo no encontrado"
    echo "Instala Rust con: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi

echo ""
echo "=== Instalación completada ==="