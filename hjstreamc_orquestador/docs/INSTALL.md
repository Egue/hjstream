# Guía de Instalación Detallada

## Pre-requisitos

### 1. Sistema Operativo

Ubuntu 20.04 LTS o superior (recomendado Ubuntu 22.04 LTS)

```bash
# Verificar versión
lsb_release -a
```

### 2. MediaMTX

El streamer asume que MediaMTX ya está instalado y configurado.

**Instalación de MediaMTX:**

```bash
# Descargar última versión
wget https://github.com/bluenviron/mediamtx/releases/download/v1.5.1/mediamtx_v1.5.1_linux_amd64.tar.gz
tar -xzf mediamtx_v1.5.1_linux_amd64.tar.gz
sudo mv mediamtx /usr/local/bin/
sudo mv mediamtx.yml /etc/mediamtx.yml
```

**Configuración básica de MediaMTX** (`/etc/mediamtx.yml`):

```yaml
logLevel: info
logDestinations: [stdout]

# RTSP server
rtspAddress: :8554
protocols: [tcp]

# Paths (ejemplo para 3 canales)
paths:
  stream1:
    source: rtmp://source.server.com/live/stream1
    sourceProtocol: rtmp
    
  stream2:
    source: srt://source.server.com:9000?streamid=stream2
    sourceProtocol: srt
    
  stream3:
    source: rtmp://another.server.com/app/stream3
    sourceProtocol: rtmp
```

**Servicio systemd para MediaMTX:**

```bash
sudo nano /etc/systemd/system/mediamtx.service
```

```ini
[Unit]
Description=MediaMTX RTSP Server
After=network.target

[Service]
Type=simple
User=mediamtx
ExecStart=/usr/local/bin/mediamtx /etc/mediamtx.yml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo useradd -r -s /bin/false mediamtx
sudo systemctl daemon-reload
sudo systemctl enable mediamtx
sudo systemctl start mediamtx
```

### 3. Verificar MediaMTX

```bash
# Estado del servicio
sudo systemctl status mediamtx

# Probar conexión RTSP
gst-launch-1.0 rtspsrc location=rtsp://127.0.0.1:8554/stream1 ! fakesink

# Alternativamente con ffprobe
ffprobe -rtsp_transport tcp rtsp://127.0.0.1:8554/stream1
```

## Instalación de Multicast Streamer

### Paso 1: Instalar dependencias

```bash
sudo apt update
sudo apt install -y \
    build-essential \
    cmake \
    pkg-config \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    gstreamer1.0-rtsp \
    gstreamer1.0-tools \
    ethtool \
    iftop \
    net-tools
```

**Verificar instalación de GStreamer:**

```bash
gst-inspect-1.0 --version
# Debe mostrar GStreamer 1.14.0 o superior
```

### Paso 2: Extraer y compilar

```bash
# Extraer archivo
tar -xzf multicast-streamer-1.0.0.tar.gz
cd multicast-streamer-1.0.0

# Crear directorio de build
mkdir build
cd build

# Configurar con CMake
cmake ..

# Compilar (usa todos los cores disponibles)
make -j$(nproc)

# Instalar (requiere sudo)
sudo make install
```

**Ubicaciones de instalación:**
- Binario: `/usr/local/bin/multicast-streamer`
- Configuración: `/etc/multicast-streamer/`
- Servicio: `/lib/systemd/system/multicast-streamer.service`
- Scripts: `/usr/local/share/multicast-streamer/`

### Paso 3: Configuración del sistema

```bash
sudo /usr/local/share/multicast-streamer/setup.sh
```

Este script:
1. Configura parámetros de kernel (sysctl)
2. Optimiza interfaz de red eno1
3. Crea usuario `streamer`
4. Crea directorios necesarios
5. Registra servicio systemd

**Verificar configuración:**

```bash
# Parámetros de kernel
sysctl net.core.rmem_max
sysctl net.ipv4.igmp_max_memberships

# Usuario
id streamer

# Directorios
ls -l /etc/multicast-streamer/
ls -l /var/log/multicast-streamer/
```

### Paso 4: Configurar canales

```bash
sudo nano /etc/multicast-streamer/canales.txt
```

**Ejemplo para 10 canales:**

```
# Canal,RTSP Path,IP Multicast,Puerto
Canal_News,/stream1,239.1.1.1,5000
Canal_Sports,/stream2,239.1.1.2,5000
Canal_Movies,/stream3,239.1.1.3,5000
Canal_Kids,/stream4,239.1.1.4,5000
Canal_Music,/stream5,239.1.1.5,5000
Canal_Documentary,/stream6,239.1.1.6,5000
Canal_Local,/stream7,239.1.1.7,5000
Canal_International,/stream8,239.1.1.8,5000
Canal_Premium1,/stream9,239.1.1.9,5000
Canal_Premium2,/stream10,239.1.1.10,5000
```

**Consideraciones:**
- Las rutas RTSP deben coincidir con los paths en `mediamtx.yml`
- Usa IPs multicast únicas (239.0.0.0 - 239.255.255.255)
- Puedes reutilizar puertos si las IPs son diferentes
- No uses espacios en los nombres de canales

### Paso 5: Test manual (opcional)

Antes de habilitar el servicio, prueba manualmente:

```bash
# Ejecutar en foreground
multicast-streamer -c /etc/multicast-streamer/canales.txt -r 5

# En otra terminal, verificar recepción
gst-launch-1.0 udpsrc uri=udp://239.1.1.1:5000 ! tsdemux ! h264parse ! avdec_h264 ! autovideosink
```

**Presiona Ctrl+C** para detener cuando hayas verificado que funciona.

### Paso 6: Habilitar servicio

```bash
# Habilitar inicio automático
sudo systemctl enable multicast-streamer

# Iniciar servicio
sudo systemctl start multicast-streamer

# Verificar estado
sudo systemctl status multicast-streamer
```

**Output esperado:**

```
● multicast-streamer.service - Multicast Streamer - Broadcast-grade streaming server
     Loaded: loaded (/lib/systemd/system/multicast-streamer.service; enabled; vendor preset: enabled)
     Active: active (running) since ...
```

### Paso 7: Monitoreo

```bash
# Ver logs en tiempo real
sudo journalctl -u multicast-streamer -f

# Ver últimas 100 líneas
sudo journalctl -u multicast-streamer -n 100

# Ver logs de hoy
sudo journalctl -u multicast-streamer --since today
```

**Logs esperados:**

```
✓ GStreamer inicializado: GStreamer 1.20.3
✓ Usando archivo de configuración: /etc/multicast-streamer/canales.txt
✓ Cargados 10 canales desde /etc/multicast-streamer/canales.txt

=== Iniciando 10 pipelines ===
[Canal_News] ✓ Pipeline iniciado → 239.1.1.1:5000
[Canal_Sports] ✓ Pipeline iniciado → 239.1.1.2:5000
...
✓ 10/10 pipelines iniciados correctamente
✓ Monitor de canales iniciado (intervalo: 5s)
```

## Configuración de Red

### Routing multicast

Si los receptores están en otra red, configura routing:

```bash
# Temporal
sudo ip route add 239.0.0.0/8 dev eno1

# Permanente (en /etc/network/interfaces o netplan)
# Ubuntu con netplan:
sudo nano /etc/netplan/01-netcfg.yaml
```

```yaml
network:
  version: 2
  ethernets:
    eno1:
      dhcp4: no
      addresses: [192.168.1.100/24]
      routes:
        - to: 239.0.0.0/8
          via: 0.0.0.0
          scope: link
```

```bash
sudo netplan apply
```

### Firewall

Si usas UFW o iptables:

```bash
# UFW
sudo ufw allow out on eno1 to 239.0.0.0/8

# iptables
sudo iptables -A OUTPUT -o eno1 -d 239.0.0.0/8 -j ACCEPT
```

### Switch/Router

Asegúrate de que tu switch soporte IGMP snooping:

```bash
# En switches Cisco:
(config)# ip igmp snooping

# En switches HP/Aruba:
(config)# ip igmp
```

## Verificación Completa

### 1. Verificar pipelines activos

```bash
# Desde el servidor
ps aux | grep multicast-streamer
sudo netstat -g | grep 239
```

### 2. Verificar tráfico multicast

```bash
# Monitorear interfaz eno1
sudo iftop -i eno1 -f "dst net 239.0.0.0/8"

# Capturar paquetes
sudo tcpdump -i eno1 -nn dst net 239.0.0.0/8
```

### 3. Probar recepción desde cliente

**Desde un cliente en la misma red:**

```bash
# Con VLC (GUI)
vlc udp://@239.1.1.1:5000

# Con ffplay (CLI)
ffplay -fflags nobuffer udp://239.1.1.1:5000

# Con GStreamer
gst-launch-1.0 udpsrc uri=udp://239.1.1.1:5000 ! tsdemux ! decodebin ! autovideosink
```

### 4. Verificar estadísticas

```bash
# Estado del servicio
sudo systemctl status multicast-streamer

# Recursos del sistema
top -p $(pgrep multicast-streamer)

# Estadísticas de red
ethtool -S eno1 | grep -E "rx_|tx_"
```

## Escalado a 70 Canales

Para 70 canales simultáneos:

### 1. Aumentar límites del sistema

```bash
sudo nano /etc/sysctl.d/99-multicast-streamer.conf
```

```ini
# Para 70+ canales
net.core.rmem_max = 268435456
net.core.wmem_max = 268435456
net.ipv4.igmp_max_memberships = 300
net.core.netdev_max_backlog = 10000
```

```bash
sudo sysctl -p /etc/sysctl.d/99-multicast-streamer.conf
```

### 2. Aumentar file descriptors

```bash
sudo nano /etc/systemd/system/multicast-streamer.service
```

```ini
[Service]
LimitNOFILE=131072
LimitNPROC=8192
```

```bash
sudo systemctl daemon-reload
sudo systemctl restart multicast-streamer
```

### 3. Configurar canales.txt

Genera automáticamente 70 entradas:

```bash
for i in {1..70}; do
  echo "Canal${i},/stream${i},239.1.1.${i},5000"
done | sudo tee /etc/multicast-streamer/canales.txt
```

### 4. Monitoreo continuo

Considera herramientas profesionales:

- **Prometheus + Grafana**: Métricas en tiempo real
- **Zabbix**: Alertas automáticas
- **Nagios**: Monitoreo de servicios

## Troubleshooting Avanzado

### Pipeline individual falla repetidamente

```bash
# Debug con GST_DEBUG
sudo systemctl stop multicast-streamer

# Modo debug
GST_DEBUG=3 multicast-streamer -c /etc/multicast-streamer/canales.txt

# Probar stream específico
gst-launch-1.0 -v rtspsrc location=rtsp://127.0.0.1:8554/stream1 latency=100 ! fakesink
```

### Drops de paquetes

```bash
# Verificar drops
ethtool -S eno1 | grep drop

# Si hay drops, aumentar ring buffers
sudo ethtool -g eno1  # Ver máximo soportado
sudo ethtool -G eno1 rx 8192 tx 8192  # Aumentar
```

### Latencia alta

1. Reducir latencia de GStreamer (en `GstPipeline.h`):
   ```cpp
   int latency_ms = 50;  // En lugar de 100
   ```

2. Priorizar proceso:
   ```bash
   sudo renice -10 $(pgrep multicast-streamer)
   ```

3. CPU pinning (en producción):
   ```bash
   sudo taskset -cp 2-7 $(pgrep multicast-streamer)
   ```

## Próximos Pasos

1. **Backup de configuración**:
   ```bash
   sudo cp /etc/multicast-streamer/canales.txt /etc/multicast-streamer/canales.txt.backup
   ```

2. **Automatizar health checks**: Ver scripts de monitoreo en `/docs/monitoring.md`

3. **Configurar alertas**: Integración con tu sistema de monitoreo

4. **Documentar flujo**: Mantén registro de IPs multicast asignadas

---

**¿Problemas?** Revisa los logs con `journalctl -u multicast-streamer -f` y busca mensajes de error.
