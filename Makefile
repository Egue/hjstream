.PHONY: help build dev release test clean install run check fmt lint

help:
	@echo "Comandos disponibles:"
	@echo "  make build      - Compilar (debug)"
	@echo "  make release    - Compilar (optimizado)"
	@echo "  make dev        - Compilar y ejecutar (debug)"
	@echo "  make run        - Ejecutar aplicación"
	@echo "  make test       - Ejecutar tests"
	@echo "  make check      - Verificar código"
	@echo "  make fmt        - Formatear código"
	@echo "  make lint       - Linter (clippy)"
	@echo "  make clean      - Limpiar builds"
	@echo "  make install    - Instalar dependencias"

build:
	cargo build

release:
	cargo build --release
	strip target/release/catv-transcoder

dev:
	cargo run

run:
	./target/release/catv-transcoder

test:
	cargo test --all-features

check:
	cargo check --all-features

fmt:
	cargo fmt

lint:
	cargo clippy --all-features -- -D warnings

clean:
	cargo clean
	rm -rf logs/*.log

install:
	sudo ./scripts/install-deps.sh

# Docker
docker-build:
	docker build -t catv-transcoder:latest .

docker-run:
	docker run -d \
		--name catv-transcoder \
		-v $(PWD)/config:/app/config \
		-v $(PWD)/logs:/app/logs \
		--network host \
		catv-transcoder:latest

# Producción
deploy: release
	@echo "Copiando binario a /usr/local/bin..."
	sudo cp target/release/catv-transcoder /usr/local/bin/
	@echo "Copiando servicio systemd..."
	sudo cp deploy/catv-transcoder.service /etc/systemd/system/
	sudo systemctl daemon-reload
	@echo "Deploy completado"