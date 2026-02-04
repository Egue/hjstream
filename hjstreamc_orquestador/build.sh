#!/bin/bash

# Script de compilación rápida
# Uso: ./build.sh [clean|install]

set -e

BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BOLD}=== Multicast Streamer - Build Script ===${NC}\n"

# Verificar dependencias
echo -e "${YELLOW}Verificando dependencias...${NC}"
if ! pkg-config --exists gstreamer-1.0; then
    echo -e "${RED}Error: GStreamer no encontrado${NC}"
    echo "Instala con: sudo apt install libgstreamer1.0-dev"
    exit 1
fi
echo -e "${GREEN}✓ GStreamer encontrado${NC}"

# Limpiar si se solicita
if [ "$1" == "clean" ]; then
    echo -e "\n${YELLOW}Limpiando build anterior...${NC}"
    rm -rf build/
    make clean 2>/dev/null || true
    echo -e "${GREEN}✓ Limpieza completada${NC}"
    exit 0
fi

# Compilar con CMake (preferido)
if command -v cmake &> /dev/null; then
    echo -e "\n${YELLOW}Compilando con CMake...${NC}"
    mkdir -p build
    cd build
    cmake ..
    make -j$(nproc)
    cd ..
    echo -e "${GREEN}✓ Compilación exitosa${NC}"
else
    # Fallback a Makefile
    echo -e "\n${YELLOW}CMake no encontrado, usando Makefile...${NC}"
    make -j$(nproc)
    echo -e "${GREEN}✓ Compilación exitosa${NC}"
fi

# Instalar si se solicita
if [ "$1" == "install" ]; then
    echo -e "\n${YELLOW}Instalando (requiere sudo)...${NC}"
    if [ -d build ]; then
        cd build
        sudo make install
        cd ..
    else
        sudo make install
    fi
    echo -e "${GREEN}✓ Instalación completada${NC}"
    echo -e "\nPróximo paso: sudo /usr/local/share/multicast-streamer/setup.sh"
fi

echo -e "\n${BOLD}Build completado${NC}"
if [ "$1" != "install" ]; then
    echo "Para instalar: ./build.sh install"
fi
