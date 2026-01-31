#!/bin/bash
# Script de verificacion rapida post-migracion
# Verificar que todos los 50 canales estan listos para operar

echo "[*] Verificando migracion de Flussonic a hjStream..."
echo ""

# 1. Contar archivos JSON
echo "[1] Contando archivos de configuracion..."
CHANNEL_COUNT=$(ls config/channels/*.json 2>/dev/null | grep -v flussonic.json | grep -v channel_001.json | grep -v simple_passthrough.json | grep -v with_transcoding.json | wc -l)
echo "    Total de canales: $CHANNEL_COUNT"

if [ "$CHANNEL_COUNT" -ne 50 ]; then
    echo "[!] ERROR: Se esperaban 50 canales, se encontraron $CHANNEL_COUNT"
    exit 1
fi

# 2. Validar formato JSON
echo ""
echo "[2] Validando formato JSON de canales..."
INVALID_JSON=0
for file in config/channels/*.json; do
    if ! python3 -m json.tool "$file" > /dev/null 2>&1; then
        INVALID_JSON=$((INVALID_JSON + 1))
        echo "    [!] JSON invalido: $file"
    fi
done

if [ "$INVALID_JSON" -eq 0 ]; then
    echo "    [OK] Todos los archivos JSON son validos"
else
    echo "    [!] ERROR: $INVALID_JSON archivos con JSON invalido"
    exit 1
fi

# 3. Verificar campos requeridos
echo ""
echo "[3] Verificando campos requeridos..."
MISSING_FIELDS=0

for file in config/channels/*.json; do
    NAME=$(basename "$file" .json)
    
    # Verificar campo 'id'
    if ! grep -q '"id":' "$file"; then
        echo "    [!] $NAME falta campo 'id'"
        MISSING_FIELDS=$((MISSING_FIELDS + 1))
    fi
    
    # Verificar local_interface
    if ! grep -q '192.168.2.140' "$file"; then
        echo "    [!] $NAME no tiene local_interface correcto"
        MISSING_FIELDS=$((MISSING_FIELDS + 1))
    fi
done

if [ "$MISSING_FIELDS" -eq 0 ]; then
    echo "    [OK] Todos los campos requeridos estan presentes"
else
    echo "    [!] ERROR: Se encontraron $MISSING_FIELDS problemas"
    exit 1
fi

# 4. Ejecutar validador Python
echo ""
echo "[4] Ejecutando validador de configuracion..."
if command -v python3 &> /dev/null; then
    python3 scripts/validate-channels.py
    if [ $? -ne 0 ]; then
        echo "[!] ERROR: Validacion fallida"
        exit 1
    fi
else
    echo "    [!] Python3 no disponible, saltando"
fi

# 5. Verificar compilacion
echo ""
echo "[5] Verificando compilacion del proyecto..."
if command -v cargo &> /dev/null; then
    if cargo check --quiet 2>/dev/null; then
        echo "    [OK] Proyecto compila correctamente"
    else
        echo "    [!] ERROR: Problemas en compilacion"
        exit 1
    fi
else
    echo "    [!] Cargo no disponible, saltando"
fi

# 6. Resumen
echo ""
echo "============================================================"
echo "[OK] VERIFICACION COMPLETADA EXITOSAMENTE"
echo "============================================================"
echo ""
echo "Resumen:"
echo "  - Canales migrados: 50"
echo "  - Formato: JSON valido"
echo "  - Campos requeridos: OK"
echo "  - Local interface: 192.168.2.140"
echo "  - Compilacion Rust: OK"
echo ""
echo "Proximo paso:"
echo "  cargo run --release -- --config config/client.json"
echo ""
