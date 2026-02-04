# Quick Start Guide

Guía de 5 minutos para poner en marcha el Multicast Streamer.

## Pre-requisitos

✅ Ubuntu 20.04+ o Debian 11+  
✅ MediaMTX corriendo en `127.0.0.1:8554`  
✅ Streams disponibles como `rtsp://127.0.0.1:8554/streamN`  

## Instalación Rápida

### 1. Extraer y compilar

```bash
tar -xzf multicast-streamer-1.0.0.tar.gz
cd multicast-streamer-1.0.0
./build.sh install
```

### 2. Configurar el sistema

```bash
sudo /usr/local/share/multicast-streamer/setup.sh
```

### 3. Editar configuración

```bash
sudo nano /etc/multicast-streamer/canales.txt
```

Ejemplo mínimo:
```
Canal1,/stream1,239.1.1.1,5000
Canal2,/stream2,239.1.1.2,5000
Canal3,/stream3,239.1.1.3,5000
```

### 4. Iniciar servicio

```bash
sudo systemctl start multicast-streamer
sudo systemctl status multicast-streamer
```

### 5. Verificar

```bash
# Ver logs
sudo journalctl -u multicast-streamer -f

# Probar recepción
gst-launch-1.0 udpsrc uri=udp://239.1.1.1:5000 ! tsdemux ! h264parse ! avdec_h264 ! autovideosink
```

## Comandos Útiles

```bash
# Ver estado
sudo systemctl status multicast-streamer

# Reiniciar
sudo systemctl restart multicast-streamer

# Ver logs
sudo journalctl -u multicast-streamer -n 100

# Detener
sudo systemctl stop multicast-streamer

# Ver tráfico multicast
sudo iftop -i eno1 -f "dst net 239.0.0.0/8"
```

## Troubleshooting Rápido

**Pipeline no inicia:**
```bash
# Verificar que MediaMTX está corriendo
curl -I http://127.0.0.1:8554

# Probar stream manualmente
gst-launch-1.0 rtspsrc location=rtsp://127.0.0.1:8554/stream1 ! fakesink
```

**No se recibe multicast:**
```bash
# Verificar routing
ip route | grep 239

# Agregar ruta si falta
sudo ip route add 239.0.0.0/8 dev eno1
```

**Drops de paquetes:**
```bash
# Ver estadísticas
ethtool -S eno1 | grep drop

# Aumentar buffers
sudo sysctl -w net.core.rmem_max=268435456
```

## Para 70 Canales

Usa el ejemplo completo:
```bash
sudo cp config/canales_70.txt.example /etc/multicast-streamer/canales.txt
sudo systemctl restart multicast-streamer
```

Ajusta límites del sistema:
```bash
sudo nano /etc/sysctl.d/99-multicast-streamer.conf
# Duplicar valores de rmem_max y wmem_max
sudo sysctl -p /etc/sysctl.d/99-multicast-streamer.conf
```

## Próximos Pasos

- 📖 Lee el [README.md](README.md) completo
- 📚 Consulta [INSTALL.md](docs/INSTALL.md) para detalles
- 🔧 Personaliza pipelines en `src/GstPipeline.cpp`
- 📊 Configura monitoreo con Prometheus/Grafana

---

**¿Problemas?** Abre un issue en GitHub con los logs de `journalctl -u multicast-streamer -n 200`
