# Guía de Contribución

¡Gracias por tu interés en contribuir al proyecto CATV Transcoding System!

## Código de Conducta

Este proyecto adhiere a un código de conducta. Al participar, se espera que mantengas este código.

## Cómo Contribuir

### Reportar Bugs

Usa los GitHub Issues con el template de bug report:
```markdown
**Descripción del bug**
Una descripción clara del problema.

**Reproducir**
Pasos para reproducir:
1. Configurar '...'
2. Ejecutar '...'
3. Ver error

**Comportamiento esperado**
Lo que esperabas que sucediera.

**Screenshots**
Si aplica, agrega screenshots.

**Ambiente:**
 - OS: [ej. Ubuntu 22.04]
 - Rust version: [ej. 1.70]
 - FFmpeg version: [ej. 4.4.2]
```

### Proponer Funcionalidades

Usa GitHub Issues con el template de feature request.

### Pull Requests

1. Fork el repo
2. Crea tu branch (`git checkout -b feature/AmazingFeature`)
3. Commit tus cambios (`git commit -m 'Add some AmazingFeature'`)
4. Push al branch (`git push origin feature/AmazingFeature`)
5. Abre un Pull Request

### Estándares de Código

- Usa `cargo fmt` antes de commit
- Ejecuta `cargo clippy` y corrige warnings
- Agrega tests para nuevas funcionalidades
- Actualiza documentación si es necesario
- Mantén commits atómicos y descriptivos

### Tests
```bash
# Ejecutar tests
cargo test

# Con coverage
cargo tarpaulin

# Tests de integración
cargo test --test '*'
```

## Licencia

Al contribuir, aceptas que tus contribuciones serán licenciadas bajo la licencia MIT.