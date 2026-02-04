# Changelog

Todos los cambios notables en este proyecto serán documentados en este archivo.

El formato está basado en [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
y este proyecto adhiere a [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-02-03

### Agregado
- Sistema de streaming multicast broadcast-grade
- Soporte para 70+ canales simultáneos
- Orquestador C++ con GStreamer
- Reconexión automática de pipelines fallidos
- Monitoreo de estado en tiempo real
- Integración con systemd
- Script de configuración del sistema
- Documentación completa
- Ejemplos de configuración
- Soporte para H.264 + AAC passthrough
- Pipeline optimizado para baja latencia (<200ms)
- Logs estructurados con journald

### Características
- Sin transcodificación (passthrough nativo)
- Múltiples canales por proceso
- Buffer configurable por canal
- Interfaz de red configurable
- Rate limiting automático
- Health checks continuos
- Reintentos con backoff

### Documentación
- README.md completo con ejemplos
- Guía de instalación detallada (INSTALL.md)
- Troubleshooting y debugging
- Configuración de red para multicast
- Benchmarks de performance

### Sistema
- Servicio systemd con reinicio automático
- Configuración de kernel optimizada
- Tuning de interfaz de red
- Usuario dedicado para el servicio
- Límites de recursos configurados

## [Unreleased]

### Planeado
- Métricas Prometheus
- API REST para control
- Dashboard web de monitoreo
- Soporte para más codecs (HEVC, VP9)
- Configuración hot-reload
- Clustering/HA
- Estadísticas de bitrate en tiempo real
