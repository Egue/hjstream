#!/bin/bash
# Script de testing y comparación de rendimiento

echo "=========================================="
echo "  Test de Rendimiento hjstreamc"
echo "=========================================="
echo ""

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Verificar dependencias
check_dependencies() {
    echo "Verificando dependencias..."
    
    deps=("gcc" "pkg-config" "ffmpeg")
    missing=()
    
    for dep in "${deps[@]}"; do
        if ! command -v $dep &> /dev/null; then
            missing+=($dep)
        fi
    done
    
    if [ ${#missing[@]} -ne 0 ]; then
        echo -e "${RED}Faltan dependencias: ${missing[*]}${NC}"
        echo "Instalar con:"
        echo "  Ubuntu/Debian: sudo apt-get install build-essential pkg-config libavformat-dev libavcodec-dev libavutil-dev"
        echo "  CentOS/RHEL:   sudo yum install gcc make pkgconfig ffmpeg-devel"
        exit 1
    fi
    
    echo -e "${GREEN}✓ Todas las dependencias instaladas${NC}"
}

# Compilar el programa
compile_program() {
    echo ""
    echo "Compilando stream_relay_ts..."
    
    gcc -Wall -O3 -pthread -march=native -mtune=native \
        -o stream_relay_ts stream_relay_ts.c \
        -lavformat -lavcodec -lavutil -lpthread -lm
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Compilación exitosa${NC}"
    else
        echo -e "${RED}✗ Error en compilación${NC}"
        exit 1
    fi
}

# Crear configuración de test
create_test_config() {
    local num_streams=$1
    local config_file="test_config_${num_streams}.txt"
    
    echo "Creando configuración de test con $num_streams streams..."
    
    cat > $config_file << EOF
# Configuración de test - $num_streams streams
# Usando streams de prueba públicos
EOF
    
    # URLs de test públicas (reemplazar con tus propias fuentes)
    test_urls=(
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ElephantsDream.mp4"
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerBlazes.mp4"
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerEscapes.mp4"
        "http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerFun.mp4"
    )
    
    for ((i=0; i<$num_streams; i++)); do
        url_index=$((i % ${#test_urls[@]}))
        output_ip="239.1.1.$((i+1))"
        output_port=$((5000 + i))
        
        echo "${test_urls[$url_index]} $output_ip $output_port" >> $config_file
    done
    
    echo -e "${GREEN}✓ Configuración creada: $config_file${NC}"
}

# Test de CPU
test_cpu_usage() {
    local num_streams=$1
    local duration=$2
    local config_file="test_config_${num_streams}.txt"
    
    echo ""
    echo "=========================================="
    echo "Test de CPU - $num_streams streams por $duration segundos"
    echo "=========================================="
    
    # Iniciar el programa en background
    ./stream_relay_ts $config_file &
    local pid=$!
    
    sleep 3  # Dar tiempo para inicializar
    
    echo "PID del proceso: $pid"
    echo "Monitoreando CPU..."
    
    # Capturar uso de CPU
    local cpu_samples=()
    for ((i=0; i<$duration; i++)); do
        if ps -p $pid > /dev/null; then
            cpu=$(ps -p $pid -o %cpu= | tr -d ' ')
            cpu_samples+=($cpu)
            echo "  T+${i}s: CPU = ${cpu}%"
            sleep 1
        else
            echo -e "${RED}Proceso terminó prematuramente${NC}"
            break
        fi
    done
    
    # Detener el proceso
    kill $pid 2>/dev/null
    wait $pid 2>/dev/null
    
    # Calcular promedio
    if [ ${#cpu_samples[@]} -gt 0 ]; then
        local sum=0
        for cpu in "${cpu_samples[@]}"; do
            sum=$(echo "$sum + $cpu" | bc)
        done
        local avg=$(echo "scale=2; $sum / ${#cpu_samples[@]}" | bc)
        
        echo ""
        echo -e "${GREEN}Resultado:${NC}"
        echo "  Streams:     $num_streams"
        echo "  CPU promedio: ${avg}%"
        echo "  CPU pico:     $(printf '%s\n' "${cpu_samples[@]}" | sort -n | tail -1)%"
    fi
}

# Test de memoria
test_memory_usage() {
    local num_streams=$1
    local config_file="test_config_${num_streams}.txt"
    
    echo ""
    echo "=========================================="
    echo "Test de Memoria - $num_streams streams"
    echo "=========================================="
    
    ./stream_relay_ts $config_file &
    local pid=$!
    
    sleep 5  # Esperar estabilización
    
    if ps -p $pid > /dev/null; then
        local mem_kb=$(ps -p $pid -o rss= | tr -d ' ')
        local mem_mb=$(echo "scale=2; $mem_kb / 1024" | bc)
        
        echo -e "${GREEN}Resultado:${NC}"
        echo "  Streams: $num_streams"
        echo "  Memoria: ${mem_mb} MB"
        
        kill $pid 2>/dev/null
        wait $pid 2>/dev/null
    else
        echo -e "${RED}Error iniciando proceso${NC}"
    fi
}

# Comparación con FFmpeg
compare_with_ffmpeg() {
    echo ""
    echo "=========================================="
    echo "Comparación: stream_relay_ts vs FFmpeg"
    echo "=========================================="
    echo ""
    
    local test_url="http://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
    local output_udp="udp://239.1.1.1:5000"
    
    # Test FFmpeg
    echo "Test 1: FFmpeg"
    echo "Comando: ffmpeg -re -i $test_url -c copy -f mpegts $output_udp"
    
    timeout 10 ffmpeg -re -i "$test_url" -c copy -f mpegts "$output_udp" &
    local ffmpeg_pid=$!
    sleep 3
    
    if ps -p $ffmpeg_pid > /dev/null; then
        local ffmpeg_cpu=$(ps -p $ffmpeg_pid -o %cpu= | tr -d ' ')
        local ffmpeg_mem=$(ps -p $ffmpeg_pid -o rss= | tr -d ' ')
        echo "  CPU: ${ffmpeg_cpu}%"
        echo "  MEM: $((ffmpeg_mem / 1024)) MB"
        kill $ffmpeg_pid 2>/dev/null
    fi
    
    sleep 2
    
    # Test stream_relay_ts
    echo ""
    echo "Test 2: stream_relay_ts"
    
    cat > test_compare.txt << EOF
$test_url 239.1.1.1 5000
EOF
    
    timeout 10 ./stream_relay_ts test_compare.txt &
    local relay_pid=$!
    sleep 3
    
    if ps -p $relay_pid > /dev/null; then
        local relay_cpu=$(ps -p $relay_pid -o %cpu= | tr -d ' ')
        local relay_mem=$(ps -p $relay_pid -o rss= | tr -d ' ')
        echo "  CPU: ${relay_cpu}%"
        echo "  MEM: $((relay_mem / 1024)) MB"
        kill $relay_pid 2>/dev/null
    fi
    
    echo ""
    echo "Nota: Esta es una comparación básica con 1 stream."
    echo "La ventaja de stream_relay_ts se nota con 20+ streams simultáneos."
}

# Menú principal
show_menu() {
    echo ""
    echo "=========================================="
    echo "  Menú de Testing"
    echo "=========================================="
    echo "1. Verificar dependencias"
    echo "2. Compilar programa"
    echo "3. Test CPU (10 streams, 30 seg)"
    echo "4. Test CPU (50 streams, 30 seg)"
    echo "5. Test Memoria (50 streams)"
    echo "6. Comparar con FFmpeg"
    echo "7. Test personalizado"
    echo "8. Salir"
    echo ""
    read -p "Selecciona opción: " option
    
    case $option in
        1) check_dependencies ;;
        2) compile_program ;;
        3) 
            create_test_config 10
            test_cpu_usage 10 30
            ;;
        4)
            create_test_config 50
            test_cpu_usage 50 30
            ;;
        5)
            create_test_config 50
            test_memory_usage 50
            ;;
        6) compare_with_ffmpeg ;;
        7)
            read -p "Número de streams: " num
            read -p "Duración (segundos): " dur
            create_test_config $num
            test_cpu_usage $num $dur
            ;;
        8) exit 0 ;;
        *) echo -e "${RED}Opción inválida${NC}" ;;
    esac
    
    show_menu
}

# Inicio
if [ "$1" == "--auto" ]; then
    # Modo automático para CI/CD
    check_dependencies
    compile_program
    create_test_config 10
    test_cpu_usage 10 20
else
    # Modo interactivo
    show_menu
fi
