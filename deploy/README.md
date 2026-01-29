# Deployment Nativo - Guía Completa

## Instalación Inicial

### 1. Preparar Sistema
```bash
# Instalar dependencias
sudo apt-get update
sudo apt-get install -y ffmpeg build-essential

# Instalar Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Compilar
```bash
cd client-rust
cargo build --release
```

### 3. Instalar
```bash
sudo ./deploy/install.sh
```

### 4. Configurar
```bash
# Editar configuración principal
sudo nano /opt/hjstream/.env

# Crear configuración de canales
sudo cp config/templates/srt_transcoder.template.json \
       /opt/hjstream/config/channels/channel_001.json

sudo nano /opt/hjstream/config/channels/channel_001.json
```

### 5. Iniciar
```bash
# Habilitar inicio automático
sudo systemctl enable hjstream

# Iniciar servicio
sudo systemctl start hjstream

# Ver estado
sudo systemctl status hjstream
```

## Gestión del Servicio

### Comandos Básicos
```bash
# Iniciar
sudo systemctl start hjstream

# Detener
sudo systemctl stop hjstream

# Reiniciar
sudo systemctl restart hjstream

# Recargar configuración (sin detener)
sudo systemctl reload hjstream

# Ver estado
sudo systemctl status hjstream

# Ver si está habilitado
sudo systemctl is-enabled hjstream
```

### Ver Logs
```bash
# Logs en tiempo real
sudo journalctl -u hjstream -f

# Últimas 100 líneas
sudo journalctl -u hjstream -n 100

# Logs desde fecha específica
sudo journalctl -u hjstream --since "2024-01-28 10:00:00"

# Solo errores
sudo journalctl -u hjstream -p err
```

## Múltiples Instancias

### Setup
```bash
# Copiar directorio base para cada instancia
sudo cp -r /opt/hjstream /opt/hjstream-01
sudo cp -r /opt/hjstream /opt/hjstream-02

# Editar configuración de cada instancia
sudo nano /opt/hjstream-01/.env  # SERVER_PORT=8080
sudo nano /opt/hjstream-02/.env  # SERVER_PORT=8081

# Iniciar instancias
sudo systemctl start hjstream@01
sudo systemctl start hjstream@02

# Ver estado de todas
sudo systemctl status 'hjstream@*'
```

## Actualización
```bash
# Método 1: Script de actualización
cd client-rust
cargo build --release
sudo ./deploy/update.sh

# Método 2: Manual
sudo systemctl stop hjstream
sudo cp target/release/hjstream /opt/hjstream/bin/
sudo systemctl start hjstream
```

## Monitoreo

### Recursos del Sistema
```bash
# Ver uso de CPU/memoria
sudo systemctl status hjstream

# Detalles de recursos
systemd-cgtop

# Logs de rendimiento
sudo journalctl -u hjstream | grep -i "memory\|cpu"
```

### Health Check
```bash
# Verificar salud del servicio
curl http://localhost:8080/health

# Script de health check
/opt/hjstream/scripts/health-check.sh
```

## Troubleshooting

### Servicio no inicia
```bash
# Ver logs detallados
sudo journalctl -u hjstream -n 50 --no-pager

# Verificar permisos
ls -la /opt/hjstream/
sudo -u transcoder /opt/hjstream/bin/hjstream

# Verificar configuración
sudo /opt/hjstream/scripts/pre-start-check.sh
```

### Alto uso de recursos
```bash
# Ver límites del servicio
systemctl show hjstream | grep -i limit

# Ajustar límites en /etc/systemd/system/hjstream.service
sudo nano /etc/systemd/system/hjstream.service

# Recargar
sudo systemctl daemon-reload
sudo systemctl restart hjstream
```

### Logs llenos
```bash
# Rotar logs manualmente
sudo logrotate -f /etc/logrotate.d/hjstream

# Limpiar journal
sudo journalctl --vacuum-time=7d
sudo journalctl --vacuum-size=500M
```

## Desinstalación
```bash
sudo ./deploy/uninstall.sh
```

## Backup y Restore

### Backup
```bash
# Backup completo
sudo tar -czf hjstream-backup-$(date +%Y%m%d).tar.gz \
    /opt/hjstream/config \
    /opt/hjstream/.env
```

### Restore
```bash
# Restore
sudo tar -xzf hjstream-backup-20240128.tar.gz -C /
sudo systemctl restart hjstream
```