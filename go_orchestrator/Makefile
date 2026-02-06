# Makefile para MediaMTX Orchestrator

APP_NAME=orchestrator
MAIN_PATH=cmd/server/main.go

# Variables de compilación
LDFLAGS=-ldflags="-s -w"
BUILD_DIR=build

.PHONY: all build clean run dev deps help linux windows macos arm64

# Comando por defecto
all: build

# Ayuda
help:
	@echo "MediaMTX Orchestrator - Comandos disponibles:"
	@echo ""
	@echo "  make build      - Compilar para la plataforma actual"
	@echo "  make linux      - Compilar para Linux x86_64"
	@echo "  make windows    - Compilar para Windows x86_64"
	@echo "  make macos      - Compilar para macOS"
	@echo "  make arm64      - Compilar para Linux ARM64 (Raspberry Pi)"
	@echo "  make all-platforms - Compilar para todas las plataformas"
	@echo "  make run        - Ejecutar la aplicación"
	@echo "  make dev        - Ejecutar en modo desarrollo (con hot reload)"
	@echo "  make deps       - Descargar dependencias"
	@echo "  make clean      - Limpiar binarios compilados"
	@echo ""

# Descargar dependencias
deps:
	@echo "Descargando dependencias..."
	go mod download
	go mod tidy

# Compilar para la plataforma actual
build: deps
	@echo "Compilando para plataforma actual..."
	go build $(LDFLAGS) -o $(APP_NAME) $(MAIN_PATH)
	@echo "✓ Binario compilado: ./$(APP_NAME)"

# Compilar para Linux x86_64
linux: deps
	@echo "Compilando para Linux x86_64..."
	@mkdir -p $(BUILD_DIR)
	GOOS=linux GOARCH=amd64 go build $(LDFLAGS) -o $(BUILD_DIR)/$(APP_NAME)-linux-amd64 $(MAIN_PATH)
	@echo "✓ Binario compilado: $(BUILD_DIR)/$(APP_NAME)-linux-amd64"

# Compilar para Windows x86_64
windows: deps
	@echo "Compilando para Windows x86_64..."
	@mkdir -p $(BUILD_DIR)
	GOOS=windows GOARCH=amd64 go build $(LDFLAGS) -o $(BUILD_DIR)/$(APP_NAME)-windows-amd64.exe $(MAIN_PATH)
	@echo "✓ Binario compilado: $(BUILD_DIR)/$(APP_NAME)-windows-amd64.exe"

# Compilar para macOS
macos: deps
	@echo "Compilando para macOS..."
	@mkdir -p $(BUILD_DIR)
	GOOS=darwin GOARCH=amd64 go build $(LDFLAGS) -o $(BUILD_DIR)/$(APP_NAME)-darwin-amd64 $(MAIN_PATH)
	@echo "✓ Binario compilado: $(BUILD_DIR)/$(APP_NAME)-darwin-amd64"

# Compilar para Linux ARM64 (Raspberry Pi, etc)
arm64: deps
	@echo "Compilando para Linux ARM64..."
	@mkdir -p $(BUILD_DIR)
	GOOS=linux GOARCH=arm64 go build $(LDFLAGS) -o $(BUILD_DIR)/$(APP_NAME)-linux-arm64 $(MAIN_PATH)
	@echo "✓ Binario compilado: $(BUILD_DIR)/$(APP_NAME)-linux-arm64"

# Compilar para todas las plataformas
all-platforms: linux windows macos arm64
	@echo "✓ Todos los binarios compilados en $(BUILD_DIR)/"

# Ejecutar la aplicación
run: build
	@echo "Iniciando aplicación..."
	./$(APP_NAME)

# Modo desarrollo (requiere air: go install github.com/cosmtrek/air@latest)
dev:
	@echo "Iniciando en modo desarrollo..."
	@which air > /dev/null || (echo "Instalando air..." && go install github.com/cosmtrek/air@latest)
	air

# Limpiar binarios
clean:
	@echo "Limpiando binarios..."
	@rm -f $(APP_NAME)
	@rm -rf $(BUILD_DIR)
	@echo "✓ Limpieza completada"

# Test
test:
	@echo "Ejecutando tests..."
	go test -v ./...

# Formatear código
fmt:
	@echo "Formateando código..."
	go fmt ./...

# Vet (análisis estático)
vet:
	@echo "Analizando código..."
	go vet ./...
