#!/usr/bin/env python3
"""
Convertidor de Flussonic a hjStream JSON
Convierte configuracion de Flussonic a formato hjStream
"""

import json
import re
from pathlib import Path

def parse_flussonic_config(content):
    """Parsea configuracion de Flussonic y retorna lista de canales"""
    channels = []
    
    # Expresion regular para extraer streams
    stream_pattern = r'stream\s+(\w+)\s*\{([^}]+)\}'
    
    for match in re.finditer(stream_pattern, content):
        channel_name = match.group(1)
        stream_content = match.group(2)
        
        # Extraer disabled, input y push
        disabled_match = re.search(r'disabled\s+(\w+)', stream_content)
        disabled = disabled_match and 'true' in disabled_match.group(1).lower()
        
        input_match = re.search(r'input\s+([^;]+)', stream_content)
        input_url = input_match.group(1).strip() if input_match else None
        
        push_match = re.search(r'push\s+([^;]+)', stream_content)
        push_url = push_match.group(1).strip() if push_match else None
        
        if input_url and push_url:
            channels.append({
                'name': channel_name,
                'enabled': not disabled,
                'input_url': input_url,
                'output_url': push_url
            })
    
    return channels

def parse_output_url(url):
    """Parsea URL de salida UDP para extraer IP, puerto e interfaz"""
    # Formato: udp://ens19@232.2.3.2:1002
    # Siempre usa 192.168.2.140 como local_interface (IP de ens19)
    match = re.match(r'udp://([^@]*@)?(.+):(\d+)', url)
    if match:
        ip = match.group(2)
        port = match.group(3)
        return '192.168.2.140', ip, port
    return '192.168.2.140', '232.2.3.2', '1002'

def detect_input_type(url):
    """Detecta el tipo de protocolo de entrada"""
    if url.startswith('srt://'):
        return 'srt'
    elif url.startswith('rtmp://'):
        return 'rtmp'
    elif url.startswith('hls://') or 'hls' in url:
        return 'hls'
    elif url.startswith('http://') or url.startswith('https://'):
        return 'http'
    else:
        return 'srt'

def create_channel_json(channel):
    """Crea JSON de hjStream para un canal"""
    interface, ip, port = parse_output_url(channel['output_url'])
    input_type = detect_input_type(channel['input_url'])
    
    channel_json = {
        "id": channel['name'],
        "name": channel['name'],
        "enabled": channel['enabled'],
        "mode": "passthrough",
        "input": {
            "type": input_type,
            "url": channel['input_url']
        },
        "output": {
            "type": "udp",
            "url": f"udp://{ip}:{port}",
            "local_interface": interface,
            "ttl": 32
        },
        "metadata": {
            "source": "Migrado desde Flussonic",
            "notes": f"Canal original: {channel['name']}"
        }
    }
    
    return channel_json

def main():
    """Script principal"""
    
    # Leer configuracion de Flussonic
    flussonic_file = Path('config/channels/flussonic.json')
    
    if not flussonic_file.exists():
        print(f"[!] Archivo {flussonic_file} no encontrado")
        return
    
    print(f"[*] Leyendo {flussonic_file}...")
    content = flussonic_file.read_text(encoding='utf-8')
    
    # Parsear canales
    channels = parse_flussonic_config(content)
    print(f"[+] {len(channels)} canales encontrados\n")
    
    # Crear directorio si no existe
    output_dir = Path('config/channels')
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Generar archivos JSON
    generated = 0
    skipped = 0
    
    for channel in channels:
        channel_json = create_channel_json(channel)
        output_file = output_dir / f"{channel['name'].lower()}.json"
        
        # Escribir archivo
        output_file.write_text(json.dumps(channel_json, indent=2, ensure_ascii=False), encoding='utf-8')
        channel_display = channel['name'][:20].ljust(20)
        file_display = output_file.name[:25].ljust(25)
        input_preview = channel['input_url'][:35]
        print(f"[+] {channel_display} -> {file_display} ({input_preview}...)")
        generated += 1
    
    print(f"\n[*] Resumen:")
    print(f"    Generados: {generated}")
    print(f"    Saltados:  {skipped}")
    print(f"    Ubicacion: {output_dir}/")
    print(f"\n[OK] Conversion completada!")

if __name__ == '__main__':
    main()
