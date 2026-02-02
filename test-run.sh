#!/bin/bash
# Test script para hjStream

# Crear directorio de logs si no existe
mkdir -p /opt/hjstream/logs

# Ejecutar el binario en modo debug
RUST_LOG=debug ./target/release/hjstream 2>&1 | tee /opt/hjstream/logs/test.log &

# Guardar el PID
PID=$!

# Esperar 10 segundos para ver si funciona
sleep 10

# Verificar que el proceso está corriendo
if ps -p $PID > /dev/null; then
    echo "✓ Proceso hjStream está corriendo (PID: $PID)"
else
    echo "✗ Proceso hjStream no está corriendo"
    exit 1
fi

# Probar API
echo "Probando API..."
curl -s http://localhost:8080/health | jq . || echo "Error en /health"
curl -s http://localhost:8080/channels | jq . || echo "Error en /channels"

# Detener proceso
kill $PID
wait $PID 2>/dev/null

echo "✓ Test completado"
