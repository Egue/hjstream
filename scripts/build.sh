#!/bin/bash

# Script de compilación
set -e

echo "=== Compilando CATV Transcoder Client ==="

# Limpiar builds anteriores
echo "Limpiando builds anteriores..."
cargo clean

# Build de desarrollo
if [ "$1" = "dev" ]; then
    echo "Compilando en modo desarrollo..."
    cargo build
    echo "✓ Build de desarrollo completado"
    echo "Ejecutable: target/debug/hjstream"
    
# Build de producción
elif [ "$1" = "release" ] || [ "$1" = "prod" ]; then
    echo "Compilando en modo producción (optimizado)..."
    cargo build --release
    
    # Opcional: strip para reducir tamaño
    if command -v strip &> /dev/null; then
        echo "Reduciendo tamaño del binario..."
        strip target/release/hjstream
    fi
    
    echo "✓ Build de producción completado"
    echo "Ejecutable: target/release/hjstream"
    
    # Mostrar tamaño
    ls -lh target/release/hjstream
    
# Build con características específicas
elif [ "$1" = "full" ]; then
    echo "Compilando con todas las características..."
    cargo build --release --all-features
    echo "✓ Build completo"
    
else
    echo "Uso: ./scripts/build.sh [dev|release|full]"
    echo "  dev     - Build de desarrollo (rápido)"
    echo "  release - Build optimizado para producción"
    echo "  full    - Build con todas las características"
    exit 1
fi

# Tests
if [ "$2" = "test" ]; then
    echo ""
    echo "=== Ejecutando tests ==="
    cargo test --all-features
fi

echo ""
echo "=== Build completado exitosamente ==="