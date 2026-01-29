#!/bin/bash

# Post-stop cleanup para systemd

INSTALL_DIR="/opt/hjstream"

# Limpiar PID file si existe
rm -f "$INSTALL_DIR/transcoder.pid"

# Rotar logs si son muy grandes (más de 100MB)
LOG_FILE="$INSTALL_DIR/logs/transcoder.log"
if [ -f "$LOG_FILE" ]; then
    SIZE=$(du -m "$LOG_FILE" | cut -f1)
    if [ "$SIZE" -gt 100 ]; then
        TIMESTAMP=$(date +%Y%m%d_%H%M%S)
        mv "$LOG_FILE" "$LOG_FILE.$TIMESTAMP"
        gzip "$LOG_FILE.$TIMESTAMP" &
    fi
fi

exit 0