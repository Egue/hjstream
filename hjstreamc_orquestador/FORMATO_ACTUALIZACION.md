# Cambio en Formato de Configuración

## ⚠️ IMPORTANTE: Nuevo formato del archivo canales.txt

El formato del archivo de configuración ha sido actualizado para permitir especificar la **interfaz de red por canal**.

### Formato Anterior (NO USAR):
```
nombre,/ruta_rtsp,ip_multicast,puerto
```

### Formato Nuevo (USAR):
```
nombre,/ruta_rtsp,ip_multicast,puerto,interfaz
```

## Ejemplos

### Básico (todos los canales en eno1):
```
Canal1,/stream1,239.1.1.1,5000,eno1
Canal2,/stream2,239.1.1.2,5000,eno1
Canal3,/stream3,239.1.1.3,5000,eno1
```

### Balanceo de carga (diferentes interfaces):
```
Canal1,/stream1,239.1.1.1,5000,eno1
Canal2,/stream2,239.1.1.2,5000,eno2
Canal3,/stream3,239.1.1.3,5000,eno1
Canal4,/stream4,239.1.1.4,5000,eno2
```

### Redundancia (mismo stream, diferentes interfaces):
```
Canal_Primary,/stream1,239.1.1.1,5000,eno1
Canal_Backup,/stream1,239.1.1.1,5000,eno2
```

## Migración desde formato anterior

Si tienes un archivo con el formato anterior, agregar la interfaz con `sed`:

```bash
# Agregar eno1 a todas las líneas
sed -i 's/$/,eno1/' /etc/multicast-streamer/canales.txt

# O para ser más seguro (solo líneas que no sean comentarios):
sed -i '/^[^#]/s/$/,eno1/' /etc/multicast-streamer/canales.txt
```

## Ventajas del nuevo formato

✅ **Flexibilidad**: Cada canal puede usar una interfaz diferente  
✅ **Balanceo de carga**: Distribuir canales entre múltiples NICs  
✅ **Redundancia**: Mismo stream por diferentes interfaces  
✅ **Sin recompilación**: Cambiar interfaz editando solo el .txt  

## Verificar interfaces disponibles

```bash
# Listar todas las interfaces
ip link show

# Ver detalles de una interfaz específica
ip addr show eno1

# Verificar que la interfaz soporta multicast
ip link show eno1 | grep MULTICAST
```

## Archivos de ejemplo incluidos

- `canales.txt.example` - Configuración básica (5 canales)
- `canales_70.txt.example` - Configuración completa (70 canales)

Ambos incluyen el nuevo formato con la columna de interfaz.
