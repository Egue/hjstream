#!/bin/bash

# Script para establecer permisos correctos
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

echo "Configurando permisos de scripts..."

chmod +x "$SCRIPT_DIR"/*.sh

echo "✓ Permisos configurados"
ls -lh "$SCRIPT_DIR"/*.sh