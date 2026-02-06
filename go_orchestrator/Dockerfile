# Build stage
FROM golang:1.21-alpine AS builder

# Instalar dependencias de compilación
RUN apk add --no-cache git make

# Establecer directorio de trabajo
WORKDIR /app

# Copiar go mod files
COPY go.mod go.sum ./

# Descargar dependencias
RUN go mod download

# Copiar código fuente
COPY . .

# Compilar aplicación
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-s -w" -o orchestrator cmd/server/main.go

# Runtime stage
FROM alpine:latest

# Instalar certificados CA
RUN apk --no-cache add ca-certificates tzdata

# Crear usuario no-root
RUN addgroup -g 1000 orchestrator && \
    adduser -D -u 1000 -G orchestrator orchestrator

# Establecer directorio de trabajo
WORKDIR /home/orchestrator

# Copiar binario desde build stage
COPY --from=builder /app/orchestrator .

# Cambiar propietario
RUN chown -R orchestrator:orchestrator /home/orchestrator

# Cambiar a usuario no-root
USER orchestrator

# Exponer puerto
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/ || exit 1

# Comando de inicio
CMD ["./orchestrator"]
