#!/bin/bash

# Script de diagnóstico para FFmpeg Orchestrator
# Ayuda a identificar problemas de red, conectividad y configuración

echo "========================================="
echo "FFmpeg Orchestrator - Diagnostic Tool"
echo "========================================="
echo ""

# Colores
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_check() {
    echo -e "${BLUE}[CHECK]${NC} $1"
}

print_ok() {
    echo -e "${GREEN}[OK]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

echo "=== System Information ==="
print_check "Operating System"
uname -a

print_check "FFmpeg Version"
if command -v ffmpeg &> /dev/null; then
    ffmpeg -version | head -n 1
    print_ok "FFmpeg is installed"
else
    print_error "FFmpeg is NOT installed"
fi

echo ""
echo "=== Network Configuration ==="

print_check "Network Interfaces"
ip addr show | grep -E "^[0-9]+:|inet "

print_check "Multicast Support"
ip maddr show

print_check "Default Routes"
ip route show

echo ""
echo "=== Multicast Testing ==="

# Test multicast route
print_check "Checking multicast route for 239.0.0.0/8"
if ip route | grep -q "239.0.0.0"; then
    print_ok "Multicast route exists"
else
    print_warn "No multicast route found - you may need to add one"
    echo "      To add: sudo route add -net 224.0.0.0 netmask 240.0.0.0 dev eth0"
fi

echo ""
echo "=== SRT Connectivity Test ==="

if [ $# -eq 2 ]; then
    SRT_HOST=$1
    SRT_PORT=$2
    
    print_check "Testing TCP connectivity to $SRT_HOST:$SRT_PORT"
    if timeout 5 bash -c "cat < /dev/null > /dev/tcp/$SRT_HOST/$SRT_PORT" 2>/dev/null; then
        print_ok "Port $SRT_PORT is reachable on $SRT_HOST"
    else
        print_error "Cannot reach $SRT_HOST:$SRT_PORT"
        echo "      This could be:"
        echo "        - Firewall blocking the connection"
        echo "        - SRT server is down"
        echo "        - Wrong IP/port"
    fi
    
    print_check "Ping test to $SRT_HOST"
    if ping -c 3 -W 2 $SRT_HOST &>/dev/null; then
        print_ok "Host $SRT_HOST is reachable"
    else
        print_warn "Cannot ping $SRT_HOST (may be normal if ICMP is blocked)"
    fi
    
    print_check "Testing with FFmpeg (5 second test)"
    echo "      Command: ffmpeg -i 'srt://$SRT_HOST:$SRT_PORT?mode=caller&latency=200000' -t 5 -f null -"
    
    if timeout 10 ffmpeg -hide_banner -loglevel error -stats -i "srt://$SRT_HOST:$SRT_PORT?mode=caller&latency=200000" -t 5 -f null - 2>&1 | tee /tmp/ffmpeg_test.log; then
        print_ok "FFmpeg can connect to SRT stream"
    else
        print_error "FFmpeg cannot connect to SRT stream"
        echo "      Error output:"
        cat /tmp/ffmpeg_test.log | sed 's/^/        /'
    fi
else
    print_warn "No SRT host/port provided for testing"
    echo "      Usage: $0 <SRT_HOST> <SRT_PORT>"
    echo "      Example: $0 131.221.42.62 8890"
fi

echo ""
echo "=== UDP/Multicast Testing ==="

LOCAL_ADDR=${3:-"192.168.2.140"}
MCAST_IP=${4:-"239.10.10.10"}
MCAST_PORT=${5:-"1010"}

print_check "Testing UDP multicast capability"
echo "      Local address: $LOCAL_ADDR"
echo "      Multicast: $MCAST_IP:$MCAST_PORT"

# Check if local address exists
if ip addr | grep -q "$LOCAL_ADDR"; then
    print_ok "Local address $LOCAL_ADDR is configured"
else
    print_error "Local address $LOCAL_ADDR is NOT configured on any interface"
    echo "      Available addresses:"
    ip addr | grep "inet " | awk '{print "        " $2}'
fi

echo ""
echo "=== System Limits ==="

print_check "File descriptor limits"
echo "      Soft limit: $(ulimit -Sn)"
echo "      Hard limit: $(ulimit -Hn)"

if [ $(ulimit -Sn) -lt 1048576 ]; then
    print_warn "File descriptor limit is low, consider increasing to 1048576"
fi

print_check "Process limits"
echo "      Max user processes: $(ulimit -u)"

echo ""
echo "=== Firewall Status ==="

print_check "UFW Status"
if command -v ufw &> /dev/null; then
    sudo ufw status | head -n 5
else
    print_warn "UFW not installed"
fi

print_check "iptables Rules (first 10)"
if command -v iptables &> /dev/null; then
    sudo iptables -L | head -n 10
else
    print_warn "iptables not available"
fi

echo ""
echo "=== FFmpeg Orchestrator Status ==="

print_check "Service Status"
if systemctl is-active --quiet ffmpeg-orchestrator; then
    print_ok "Service is running"
    systemctl status ffmpeg-orchestrator --no-pager | head -n 10
else
    print_warn "Service is not running"
    echo "      Start with: sudo systemctl start ffmpeg-orchestrator"
fi

print_check "API Health Check"
if curl -s -f http://localhost:3000/health > /dev/null 2>&1; then
    print_ok "API is responding"
    curl -s http://localhost:3000/health
else
    print_error "API is not responding"
fi

echo ""
echo "=== Recent Logs ==="

print_check "Orchestrator logs (last 20 lines)"
if [ -f /var/log/hjstream/*.log ]; then
    tail -n 20 /var/log/hjstream/*.log 2>/dev/null || print_warn "No channel logs found"
else
    print_warn "No logs found in /var/log/hjstream/"
fi

echo ""
echo "=== Kernel Network Parameters ==="

print_check "UDP Buffer Sizes"
echo "      rmem_max: $(cat /proc/sys/net/core/rmem_max)"
echo "      wmem_max: $(cat /proc/sys/net/core/wmem_max)"
echo "      rmem_default: $(cat /proc/sys/net/core/rmem_default)"
echo "      wmem_default: $(cat /proc/sys/net/core/wmem_default)"

print_check "UDP Memory"
cat /proc/sys/net/ipv4/udp_mem

echo ""
echo "=== Common Issues and Solutions ==="
echo ""
echo "1. If FFmpeg disconnects after ~1 minute:"
echo "   - Check SRT latency (try increasing to 500000 or 1000000)"
echo "   - Verify network stability: ping -c 100 <SRT_HOST>"
echo "   - Check if SRT server has timeout settings"
echo "   - Look for 'Connection timeout' in logs"
echo ""
echo "2. If no logs appear:"
echo "   - Check permissions: sudo chown -R \$USER /var/log/hjstream"
echo "   - Verify FFmpeg is actually running: ps aux | grep ffmpeg"
echo "   - Check systemd logs: journalctl -u ffmpeg-orchestrator -n 50"
echo ""
echo "3. If multicast doesn't work:"
echo "   - Add multicast route: sudo route add -net 224.0.0.0 netmask 240.0.0.0 dev eth0"
echo "   - Enable multicast: sudo ip link set dev eth0 multicast on"
echo "   - Check firewall: sudo ufw allow from 239.0.0.0/8"
echo ""
echo "4. Test FFmpeg command manually:"
echo "   ffmpeg -i 'srt://HOST:PORT?mode=caller&latency=200000' \\"
echo "          -c copy -f mpegts \\"
echo "          'udp://239.10.10.10:1010?pkt_size=1316&localaddr=192.168.2.140&ttl=64'"
echo ""
echo "========================================="
echo "Diagnostic Complete"
echo "========================================="
