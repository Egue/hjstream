# hjstream - Guía de Mejores Prácticas

## Configuración de Red

### Multicast UDP
Para que el multicast UDP funcione correctamente:

1. **Verificar que el multicast esté habilitado en la interfaz**:
```bash
# Ver configuración de multicast
ip maddr show

# Habilitar multicast en una interfaz
sudo ip link set dev eth0 multicast on
```

2. **Añadir rutas de multicast si es necesario**:
```bash
sudo route add -net 224.0.0.0 netmask 240.0.0.0 dev eth0
```

3. **Verificar TTL (Time To Live)**:
- Para redes locales (mismo switch): TTL = 1
- Para múltiples switches: TTL = 64
- Para enrutamiento: TTL = 128

### Firewall
Si usas firewall, permite el tráfico UDP multicast:

```bash
# UFW
sudo ufw allow from 239.0.0.0/8 to any

# iptables
sudo iptables -A INPUT -p udp -d 239.0.0.0/8 -j ACCEPT
sudo iptables -A OUTPUT -p udp -d 239.0.0.0/8 -j ACCEPT
```

## Configuración de FFmpeg

### Tamaño de Paquete (pkt_size)
- **1316**: Valor recomendado para evitar fragmentación IP
- **1500**: Tamaño máximo teórico (MTU estándar)
- **Ajustar según red**: Para redes con MTU menor, reducir este valor

### Latencia en SRT
```json
{
  "latency": 200000  // 200ms - Bueno para redes locales estables
  "latency": 500000  // 500ms - Mejor para redes con mayor variación
  "latency": 1000000 // 1s - Para conexiones WAN o satélite
}
```

### Buffering
Para reducir problemas de buffering:

1. **Aumentar el buffer size**:
```bash
# En el comando FFmpeg (si necesitas customización adicional)
ffmpeg -i input -buffer_size 10M -c copy output
```

2. **Ajustar la latencia SRT** según tu red

## Monitoreo y Logging

### Ver logs en tiempo real
```bash
# Logs del servicio
sudo journalctl -u hjstream -f

# Logs de un canal específico
tail -f /var/log/hjstream/nombre-canal.log
```

### Estadísticas de red
```bash
# Ver estadísticas de multicast
netstat -g

# Monitorear tráfico UDP
sudo tcpdump -i eth0 udp and dst net 239.0.0.0/8

# Ver conexiones SRT
ss -u | grep 8890
```

### Uso de recursos
```bash
# Ver procesos FFmpeg
ps aux | grep ffmpeg

# Memoria y CPU por proceso
top -p $(pgrep -d',' ffmpeg)

# Recursos del orquestador
systemctl status hjstream
```

## Optimización de Performance

### Límites del Sistema
Editar `/etc/security/limits.conf`:
```
*  soft  nofile  1048576
*  hard  nofile  1048576
*  soft  nproc   512
*  hard  nproc   512
```

### Network Tuning
Editar `/etc/sysctl.conf`:
```bash
# Aumentar buffers de red
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.core.rmem_default = 67108864
net.core.wmem_default = 67108864

# UDP específico
net.ipv4.udp_mem = 8388608 12582912 16777216
net.ipv4.udp_rmem_min = 16384
net.ipv4.udp_wmem_min = 16384

# Multicast
net.ipv4.igmp_max_memberships = 100
```

Aplicar cambios:
```bash
sudo sysctl -p
```

## Troubleshooting Común

### Problema: Canal no inicia
**Síntomas**: Estado permanece en "starting" o cambia a "error"

**Soluciones**:
1. Verificar logs del canal:
   ```bash
   cat /var/log/hjstream/nombre-canal.log
   ```

2. Probar FFmpeg manualmente:
   ```bash
   ffmpeg -i "srt://ip:puerto?mode=caller&latency=200000" -c copy -f mpegts "udp://239.x.x.x:port?pkt_size=1316&localaddr=192.168.x.x"
   ```

3. Verificar que FFmpeg esté instalado:
   ```bash
   which ffmpeg
   ffmpeg -version
   ```

### Problema: Reconexiones frecuentes
**Síntomas**: Canal alterna entre "running" y "reconnecting"

**Soluciones**:
1. **Aumentar latencia SRT**:
   ```json
   "latency": 500000  // Cambiar de 200000 a 500000
   ```

2. **Verificar estabilidad de red**:
   ```bash
   ping -c 100 <ip-servidor-srt>
   ```

3. **Revisar calidad del stream de entrada**:
   - Verificar que el servidor SRT esté estable
   - Comprobar ancho de banda disponible

### Problema: Paquetes perdidos
**Síntomas**: Imagen pixelada o congelada en el receptor

**Soluciones**:
1. **Aumentar buffer UDP**:
   ```bash
   # Temporal
   sudo sysctl -w net.core.rmem_max=134217728
   ```

2. **Reducir tamaño de paquete**:
   ```json
   "pkt_size": 1200  // Cambiar de 1316
   ```

3. **Verificar tráfico de red**:
   ```bash
   iftop -i eth0
   ```

### Problema: Permisos denegados
**Síntomas**: Error al escribir logs o configuración

**Soluciones**:
```bash
# Corregir permisos
sudo chown -R root:root /var/log/hjstream
sudo chown -R root:root /etc/hjstream
sudo chmod 755 /var/log/hjstream
sudo chmod 755 /etc/hjstream
```

## Estrategias de Alta Disponibilidad

### Múltiples instancias
Para alta disponibilidad, ejecutar múltiples instancias:

1. **Servidor primario**: Puerto 31337
2. **Servidor secundario**: Puerto 3001

Usar un load balancer (nginx/HAProxy) para distribuir:

```nginx
upstream orchestrator {
    server 127.0.0.1:31337;
    server 127.0.0.1:3001 backup;
}
```

### Backup de configuración
Hacer backup regular de la configuración:

```bash
#!/bin/bash
# backup-channels.sh
DATE=$(date +%Y%m%d_%H%M%S)
cp /etc/hjstream/channels.json /backup/channels_$DATE.json
```

Agregar a cron:
```bash
0 2 * * * /usr/local/bin/backup-channels.sh
```

### Monitoreo con watchdog
Crear script de monitoreo:

```bash
#!/bin/bash
# watchdog.sh
if ! systemctl is-active --quiet hjstream; then
    echo "Service down, restarting..."
    systemctl start hjstream
    # Enviar alerta (email, Slack, etc.)
fi
```

## Integración con Sistemas Externos

### Webhook para notificaciones
Modificar el código para enviar webhooks cuando:
- Un canal cambia de estado
- Ocurre un error
- Se alcanza un threshold de reconexiones

### API de métricas
Exponer métricas de Prometheus:
- Número de canales activos
- Tasa de reconexiones
- Uso de CPU/memoria por canal

### Dashboard Web
Crear un frontend con:
- Estado en tiempo real de canales
- Gráficos de métricas
- Control de canales (start/stop/restart)

## Mantenimiento

### Rotación de logs
Crear `/etc/logrotate.d/hjstream`:

```
/var/log/hjstream/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0644 root root
}
```

### Actualización del orquestador
```bash
# 1. Detener el servicio
sudo systemctl stop hjstream

# 2. Hacer backup
sudo cp /usr/local/bin/hjstream /usr/local/bin/hjstream.bak
sudo cp /etc/hjstream/channels.json /etc/hjstream/channels.json.bak

# 3. Compilar nueva versión
cd /path/to/hjstream
git pull
cargo build --release

# 4. Instalar
sudo cp target/release/hjstream /usr/local/bin/

# 5. Reiniciar
sudo systemctl start hjstream

# 6. Verificar
sudo systemctl status hjstream
```

## Seguridad

### Autenticación API
Añadir autenticación básica con tokens:

1. Generar token:
```bash
openssl rand -hex 32
```

2. Configurar en headers:
```bash
curl -H "Authorization: Bearer YOUR_TOKEN" http://localhost:31337/api/channels
```

### HTTPS
Usar nginx como reverse proxy con SSL:

```nginx
server {
    listen 443 ssl;
    server_name orchestrator.example.com;

    ssl_certificate /etc/ssl/certs/cert.pem;
    ssl_certificate_key /etc/ssl/private/key.pem;

    location / {
        proxy_pass http://localhost:31337;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### Restricción de IPs
Limitar acceso a IPs específicas:

```nginx
location /api {
    allow 192.168.1.0/24;
    allow 10.0.0.0/8;
    deny all;
    
    proxy_pass http://localhost:31337;
}
```

## Recursos Adicionales

- [FFmpeg Documentation](https://ffmpeg.org/documentation.html)
- [SRT Protocol](https://github.com/Haivision/srt)
- [Multicast Networking](https://en.wikipedia.org/wiki/IP_multicast)
- [Rust Tokio](https://tokio.rs/)
