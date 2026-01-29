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
	strip target/release/hjstream

dev:
	cargo run

run:
	./target/release/hjstream

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


# Producción
deploy: release
	@echo "Copiando binario a /usr/local/bin..."
	sudo cp target/release/hjstream /usr/local/bin/
	@echo "Copiando servicio systemd..."
	sudo cp deploy/hjstream.service /etc/systemd/system/
	sudo systemctl daemon-reload
	@echo "Deploy completado"


# Docker targets
docker-build:
	./scripts/docker-build.sh $(VERSION)

docker-push:
	./scripts/docker-push.sh $(VERSION)

docker-run:
	docker-compose up -d

docker-stop:
	docker-compose down

docker-logs:
	docker-compose logs -f transcoder

docker-shell:
	docker exec -it hjstream /bin/bash

docker-clean:
	docker-compose down -v
	docker rmi hjstream:latest

# Docker development
docker-dev:
	docker-compose -f docker-compose.dev.yml up

docker-dev-build:
	docker-compose -f docker-compose.dev.yml build

# Docker production
docker-prod:
	docker-compose -f docker-compose.prod.yml up -d

docker-prod-stop:
	docker-compose -f docker-compose.prod.yml down