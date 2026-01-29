#!/bin/bash

# Script para construir imagen Docker

set -e

VERSION=${1:-latest}
REGISTRY=${DOCKER_REGISTRY:-}

echo "=== Building Docker Image ==="
echo "Version: $VERSION"

# Build
docker build -t hjstream:$VERSION .

# Tag como latest también
if [ "$VERSION" != "latest" ]; then
    docker tag hjstream:$VERSION hjstream:latest
fi

# Tag para registry si está definido
if [ -n "$REGISTRY" ]; then
    docker tag hjstream:$VERSION $REGISTRY/hjstream:$VERSION
    docker tag hjstream:$VERSION $REGISTRY/hjstream:latest
    
    echo "Tagged for registry: $REGISTRY"
fi

# Mostrar tamaño
echo ""
echo "Image size:"
docker images hjstream:$VERSION

echo ""
echo "✓ Build completed"
echo ""
echo "To run:"
echo "  docker run -d --name transcoder hjstream:$VERSION"
echo ""
echo "With docker-compose:"
echo "  docker-compose up -d"