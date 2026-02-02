#!/bin/bash

# hjstream - API Test Scripts
# Ejemplos de uso de la API REST

API_URL="http://localhost:31337"

echo "========================================="
echo "hjstream - API Examples"
echo "========================================="
echo ""

# 1. Health Check
echo "1. Health Check"
echo "curl $API_URL/health"
curl -s $API_URL/health | jq
echo ""
echo "---"
echo ""

# 2. Crear canal SRT
echo "2. Crear canal SRT (similar a tu configuración actual)"
echo ""
CHANNEL_SRT=$(curl -s -X POST $API_URL/api/channels \
  -H "Content-Type: application/json" \
  -d '{
    "name": "10chv",
    "description": "Canal 10 CHV - SRT to UDP",
    "input": {
      "format": "srt",
      "url": "192.x.x.x:111",
      "mode": "caller",
      "latency": 200000
    },
    "output": {
      "multicast_ip": "239.10.10.10",
      "port": 1010,
      "local_addr": "192.168.2.140",
      "pkt_size": 1316,
      "ttl": 64
    }
  }')

echo "$CHANNEL_SRT" | jq
CHANNEL_ID_1=$(echo "$CHANNEL_SRT" | jq -r '.channel.id')
echo ""
echo "Canal creado con ID: $CHANNEL_ID_1"
echo "---"
echo ""

# 3. Crear canal RTMP
echo "3. Crear canal RTMP"
echo ""
CHANNEL_RTMP=$(curl -s -X POST $API_URL/api/channels \
  -H "Content-Type: application/json" \
  -d '{
    "name": "canal-rtmp-test",
    "description": "Canal RTMP de prueba",
    "input": {
      "format": "rtmp",
      "url": "rtmp://live.example.com/stream/test"
    },
    "output": {
      "multicast_ip": "239.10.10.20",
      "port": 1020,
      "local_addr": "192.168.2.140"
    }
  }')

echo "$CHANNEL_RTMP" | jq
CHANNEL_ID_2=$(echo "$CHANNEL_RTMP" | jq -r '.channel.id')
echo ""
echo "Canal creado con ID: $CHANNEL_ID_2"
echo "---"
echo ""

# 4. Crear canal HLS
echo "4. Crear canal HLS"
echo ""
CHANNEL_HLS=$(curl -s -X POST $API_URL/api/channels \
  -H "Content-Type: application/json" \
  -d '{
    "name": "canal-hls-test",
    "description": "Canal HLS de prueba",
    "input": {
      "format": "hls",
      "url": "https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8"
    },
    "output": {
      "multicast_ip": "239.10.10.30",
      "port": 1030,
      "local_addr": "192.168.2.140"
    }
  }')

echo "$CHANNEL_HLS" | jq
CHANNEL_ID_3=$(echo "$CHANNEL_HLS" | jq -r '.channel.id')
echo ""
echo "Canal creado con ID: $CHANNEL_ID_3"
echo "---"
echo ""

# 5. Listar todos los canales
echo "5. Listar todos los canales"
echo "curl $API_URL/api/channels"
curl -s $API_URL/api/channels | jq
echo ""
echo "---"
echo ""

# 6. Obtener detalles de un canal
echo "6. Obtener detalles del primer canal"
echo "curl $API_URL/api/channels/$CHANNEL_ID_1"
curl -s $API_URL/api/channels/$CHANNEL_ID_1 | jq
echo ""
echo "---"
echo ""

# 7. Iniciar un canal
echo "7. Iniciar el primer canal (SRT)"
echo "curl -X PUT $API_URL/api/channels/$CHANNEL_ID_1/start"
curl -s -X PUT $API_URL/api/channels/$CHANNEL_ID_1/start | jq
echo ""
echo "---"
echo ""

# Esperar un momento
echo "Esperando 5 segundos para que el canal inicie..."
sleep 5

# 8. Ver el estado actualizado
echo "8. Ver estado actualizado del canal"
echo "curl $API_URL/api/channels/$CHANNEL_ID_1"
curl -s $API_URL/api/channels/$CHANNEL_ID_1 | jq
echo ""
echo "---"
echo ""

# 9. Detener el canal
echo "9. Detener el canal"
echo "curl -X PUT $API_URL/api/channels/$CHANNEL_ID_1/stop"
curl -s -X PUT $API_URL/api/channels/$CHANNEL_ID_1/stop | jq
echo ""
echo "---"
echo ""

# 10. Reiniciar el canal
echo "10. Reiniciar el canal"
echo "curl -X PUT $API_URL/api/channels/$CHANNEL_ID_1/restart"
curl -s -X PUT $API_URL/api/channels/$CHANNEL_ID_1/restart | jq
echo ""
echo "---"
echo ""

# 11. Eliminar canales de prueba (opcional)
read -p "¿Desea eliminar los canales de prueba creados? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo "11. Eliminando canales de prueba..."
    
    echo "Deteniendo y eliminando canal RTMP..."
    curl -s -X PUT $API_URL/api/channels/$CHANNEL_ID_2/stop > /dev/null
    curl -s -X DELETE $API_URL/api/channels/$CHANNEL_ID_2
    echo "✓ Canal RTMP eliminado"
    
    echo "Deteniendo y eliminando canal HLS..."
    curl -s -X PUT $API_URL/api/channels/$CHANNEL_ID_3/stop > /dev/null
    curl -s -X DELETE $API_URL/api/channels/$CHANNEL_ID_3
    echo "✓ Canal HLS eliminado"
    
    echo ""
    echo "Canales restantes:"
    curl -s $API_URL/api/channels | jq
fi

echo ""
echo "========================================="
echo "Ejemplos completados!"
echo "========================================="
echo ""
echo "Para más información, consulta el README.md"
