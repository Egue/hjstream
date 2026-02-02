#!/bin/bash

# Script para monitorear logs de canales en tiempo real
# Muestra salida de FFmpeg mientras está corriendo

if [ $# -eq 0 ]; then
    echo "Usage: $0 <channel-name>"
    echo ""
    echo "Available channels:"
    ls -1 /var/log/hjstream/*.log 2>/dev/null | xargs -n1 basename | sed 's/.log$//' | sed 's/^/  - /'
    exit 1
fi

CHANNEL_NAME=$1
LOG_FILE="/var/log/hjstream/${CHANNEL_NAME}.log"

if [ ! -f "$LOG_FILE" ]; then
    echo "Error: Log file not found: $LOG_FILE"
    echo ""
    echo "Available channels:"
    ls -1 /var/log/hjstream/*.log 2>/dev/null | xargs -n1 basename | sed 's/.log$//' | sed 's/^/  - /'
    exit 1
fi

echo "========================================="
echo "Monitoring channel: $CHANNEL_NAME"
echo "Log file: $LOG_FILE"
echo "========================================="
echo ""
echo "Press Ctrl+C to stop monitoring"
echo ""

# Highlight important patterns
tail -f "$LOG_FILE" | while read line; do
    if echo "$line" | grep -qi "error"; then
        echo -e "\033[0;31m$line\033[0m"  # Red for errors
    elif echo "$line" | grep -qi "warning"; then
        echo -e "\033[1;33m$line\033[0m"  # Yellow for warnings
    elif echo "$line" | grep -qi "starting\|reconnecting"; then
        echo -e "\033[0;32m$line\033[0m"  # Green for status changes
    elif echo "$line" | grep -qi "terminated\|stopped"; then
        echo -e "\033[0;35m$line\033[0m"  # Magenta for termination
    else
        echo "$line"
    fi
done
