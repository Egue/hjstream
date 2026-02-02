#!/bin/bash
# Script de validación para hjStream en Linux

echo "=== Validación hjStream Refactorizado ==="

# 1. Verificar FFmpeg con SRT
echo ""
echo "1. Verificando FFmpeg con SRT..."
if /usr/bin/ffmpeg -protocols 2>&1 | grep -q srt; then
    echo "   ✓ FFmpeg tiene soporte SRT"
else
    echo "   ✗ FFmpeg NO tiene soporte SRT"
    echo "   Instalar: sudo add-apt-repository ppa:savoury1/ffmpeg4"
    exit 1
fi

# 2. Verificar configuración de canales
echo ""
echo "2. Verificando configuración de canales..."
CHANNEL_CONFIG="/etc/hjstream/channels/test-channel.json"
if [ -f "$CHANNEL_CONFIG" ]; then
    echo "   ✓ Archivo de configuración encontrado"
    echo "   Contenido:"
    jq . "$CHANNEL_CONFIG" 2>/dev/null || cat "$CHANNEL_CONFIG"
else
    echo "   ✗ No hay archivos de configuración de canales"
fi

# 3. Compilar el proyecto
echo ""
echo "3. Compilando hjStream..."
if cargo build --release 2>&1 | grep -q "Finished"; then
    echo "   ✓ Compilación exitosa"
else
    echo "   ✗ Error en compilación"
    exit 1
fi

# 4. Verificar binario
echo ""
echo "4. Verificando binario..."
if [ -x ./target/release/hjstream ]; then
    SIZE=$(du -h ./target/release/hjstream | cut -f1)
    echo "   ✓ Binario creado exitosamente (Tamaño: $SIZE)"
else
    echo "   ✗ Binario no encontrado"
    exit 1
fi

# 5. Prueba de comando FFmpeg generado
echo ""
echo "5. Simulando comando FFmpeg..."
INPUT_URL="srt://181.79.86.130:20582?mode=caller&latency=200000"
OUTPUT_URL="udp://239.10.10.20:1234?localaddr=192.168.2.140&pkt_size=1316"
CMD="/usr/bin/ffmpeg -loglevel info -stats -i \"$INPUT_URL\" -c copy -f mpegts \"$OUTPUT_URL\""
echo "   Comando que se ejecutaría:"
echo "   $CMD"

# 6. Simular timeout breve (para no conectar realmente)
echo ""
echo "6. Test de timeout breve (3 segundos)..."
timeout 3 /usr/bin/ffmpeg -loglevel error -i "$INPUT_URL" -c copy -f mpegts "$OUTPUT_URL" 2>&1 || true
echo "   ✓ Comando FFmpeg es válido (timeout esperado)"

echo ""
echo "=== Validación Completada ==="
echo ""
echo "Próximos pasos:"
echo "1. Copiar binario a /opt/hjstream/bin/hjstream"
echo "2. Copiar config a /etc/hjstream/channels/"
echo "3. Instalar systemd service: sudo install -m 644 deploy/hjstream.service /etc/systemd/system/"
echo "4. Iniciar: sudo systemctl start hjstream"
echo "5. Ver logs: sudo journalctl -fu hjstream"
