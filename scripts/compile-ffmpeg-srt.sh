#!/bin/bash
# Script para compilar FFmpeg con soporte SRT
# Uso: bash compile-ffmpeg-srt.sh

set -e

echo "[*] Compilando FFmpeg con soporte SRT..."
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Versión de FFmpeg (usar 5.x por mejor soporte SRT)
FFMPEG_VERSION="5.1.2"
BUILD_DIR="/tmp/ffmpeg_build"
INSTALL_PREFIX="/usr/local"

echo -e "${YELLOW}[1/5] Instalando dependencias...${NC}"
sudo apt-get update
sudo apt-get install -y \
  build-essential \
  git \
  wget \
  cmake \
  pkg-config \
  libfdk-aac-dev \
  libopus-dev \
  libvpx-dev \
  libx264-dev \
  libx265-dev \
  libssl-dev \
  yasm \
  nasm \
  autoconf \
  automake \
  libtool \
  tclsh

echo -e "${GREEN}[✓] Dependencias instaladas${NC}"
echo ""

# Compilar libsrt desde fuente
echo -e "${YELLOW}[1.5/5] Compilando libsrt desde fuente...${NC}"
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

if [ ! -d "srt" ]; then
  git clone https://github.com/Haivision/srt.git
fi

cd srt
git pull origin master 2>/dev/null || true
mkdir -p build
cd build

cmake .. -DCMAKE_INSTALL_PREFIX=/usr/local -DENABLE_SHARED=ON -DENABLE_STATIC=ON
make -j$(nproc)
sudo make install
sudo ldconfig

echo -e "${GREEN}[✓] libsrt compilado${NC}"
echo ""

echo -e "${YELLOW}[2/5] Descargando FFmpeg ${FFMPEG_VERSION}...${NC}"
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

if [ ! -f "ffmpeg-${FFMPEG_VERSION}.tar.bz2" ]; then
  wget -q "https://ffmpeg.org/releases/ffmpeg-${FFMPEG_VERSION}.tar.bz2"
fi

if [ ! -d "ffmpeg-${FFMPEG_VERSION}" ]; then
  tar -xjf "ffmpeg-${FFMPEG_VERSION}.tar.bz2"
fi

cd "ffmpeg-${FFMPEG_VERSION}"

echo -e "${GREEN}[✓] FFmpeg descargado${NC}"
echo ""

echo -e "${YELLOW}[3/5] Configurando compilación con SRT...${NC}"
export PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:$PKG_CONFIG_PATH"
export LD_LIBRARY_PATH="/usr/local/lib:$LD_LIBRARY_PATH"

./configure \
  --prefix="$INSTALL_PREFIX" \
  --enable-gpl \
  --enable-libx264 \
  --enable-libx265 \
  --enable-libopus \
  --enable-libvpx \
  --enable-libfdk-aac \
  --enable-libsrt \
  --enable-nonfree \
  --enable-shared \
  --enable-network \
  --enable-protocol=file \
  --enable-protocol=http \
  --enable-protocol=https \
  --enable-protocol=rtmp \
  --enable-protocol=rtmps \
  --enable-protocol=srt \
  --enable-protocol=udp \
  2>&1 | tail -20

echo -e "${GREEN}[✓] Configuración completada${NC}"
echo ""

echo -e "${YELLOW}[4/5] Compilando FFmpeg (esto puede tomar 15-20 minutos)...${NC}"
NUM_CORES=$(nproc)
echo "[*] Usando $NUM_CORES cores"
make -j"$NUM_CORES" 2>&1 | tail -10

echo -e "${GREEN}[✓] Compilación completada${NC}"
echo ""

echo -e "${YELLOW}[5/5] Instalando FFmpeg...${NC}"
sudo make install
sudo ldconfig

echo -e "${GREEN}[✓] Instalación completada${NC}"
echo ""

echo -e "${YELLOW}Verificando instalación...${NC}"
ffmpeg_path=$(which ffmpeg)
echo "FFmpeg ubicado en: $ffmpeg_path"
ffmpeg_version=$(ffmpeg -version | head -1)
echo "Versión: $ffmpeg_version"

echo ""
echo -e "${YELLOW}Protocolos disponibles:${NC}"
ffmpeg -protocols 2>&1 | grep -E "(srt|udp|rtmp|hls)"

echo ""
echo -e "${GREEN}[OK] FFmpeg compilado con soporte SRT!${NC}"
echo ""

# Probar con un canal
read -p "¿Deseas probar con el canal CARACOL? (s/n): " -n 1 -r
echo
if [[ $REPLY =~ ^[Ss]$ ]]; then
  echo -e "${YELLOW}Probando conexión SRT...${NC}"
  ffprobe -v quiet srt://181.79.86.130:20582 2>&1 | head -5 || echo "Error de conexión (normal si la fuente está offline)"
fi

echo ""
echo -e "${GREEN}[✓] Script completado!${NC}"
