# Multi-stage build para optimizar tamaño de imagen

# ============================================================================
# Stage 1: Builder
# ============================================================================
FROM rust:1.75-slim-bookworm AS builder

# Instalar dependencias de compilación
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Crear directorio de trabajo
WORKDIR /build

# Copiar archivos de dependencias primero (para cache de Docker)
COPY Cargo.toml Cargo.lock ./

# Crear dummy main para compilar dependencias
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copiar código fuente real
COPY src ./src

# Compilar aplicación
RUN cargo build --release

# Strip para reducir tamaño
RUN strip /build/target/release/hjstream

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM debian:bookworm-slim

# Instalar dependencias de runtime
RUN apt-get update && apt-get install -y \
    ffmpeg \
    libssl3 \
    ca-certificates \
    curl \
    iproute2 \
    net-tools \
    && rm -rf /var/lib/apt/lists/*

# Crear usuario no-root
RUN useradd -m -u 1000 -s /bin/bash transcoder

# Crear directorios necesarios
RUN mkdir -p /app/config/channels \
             /app/config/backup \
             /app/logs \
    && chown -R transcoder:transcoder /app

# Copiar binario desde builder
COPY --from=builder /build/target/release/hjstream /app/hjstream

# Copiar archivos de configuración
COPY --chown=transcoder:transcoder config/ /app/config/
COPY --chown=transcoder:transcoder .env.example /app/.env

# Cambiar a usuario no-root
USER transcoder
WORKDIR /app

# Exponer puertos
EXPOSE 8080 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Variables de entorno por defecto
ENV RUST_LOG=info \
    SERVER_PORT=8080 \
    METRICS_PORT=9090

# Entry point
ENTRYPOINT ["/app/hjstream"]