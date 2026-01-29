#!/bin/bash

# Script para subir imagen a registry

set -e

VERSION=${1:-latest}
REGISTRY=${DOCKER_REGISTRY}

if [ -z "$REGISTRY" ]; then
    echo "Error: DOCKER_REGISTRY no está definido"
    echo "Uso: DOCKER_REGISTRY=registry.example.com ./scripts/docker-push.sh [version]"
    exit 1
fi

echo "=== Pushing to Registry ==="
echo "Registry: $REGISTRY"
echo "Version: $VERSION"

# Login si es necesario
if [ -n "$DOCKER_USERNAME" ] && [ -n "$DOCKER_PASSWORD" ]; then
    echo "Logging in to registry..."
    echo "$DOCKER_PASSWORD" | docker login $REGISTRY -u $DOCKER_USERNAME --password-stdin
fi

# Push
docker push $REGISTRY/hjstream:$VERSION

if [ "$VERSION" != "latest" ]; then
    docker push $REGISTRY/hjstream:latest
fi

echo "✓ Push completed"