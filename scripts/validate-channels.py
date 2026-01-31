#!/usr/bin/env python3
"""
Validador de configuracion de canales hjStream
Verifica que todos los archivos JSON generados desde Flussonic sean validos
"""

import json
from pathlib import Path
from collections import defaultdict

def validate_channels():
    """Valida todos los archivos de canales generados"""
    
    config_dir = Path('config/channels')
    
    # Patrones validos
    valid_input_types = {'srt', 'rtmp', 'hls', 'http'}
    valid_output_types = {'udp', 'rtp'}
    valid_modes = {'passthrough', 'transcode'}
    
    # Estadisticas
    stats = {
        'total': 0,
        'valid': 0,
        'invalid': 0,
        'errors': [],
        'protocols': defaultdict(int),
        'local_interfaces': defaultdict(int),
        'multicast_ranges': defaultdict(int)
    }
    
    # Archivos ignorados
    ignored_files = {'flussonic.json', 'channel_001.json', 'simple_passthrough.json', 'with_transcoding.json'}
    
    json_files = sorted([f for f in config_dir.glob('*.json') if f.name not in ignored_files])
    
    print(f"[*] Validando {len(json_files)} archivos de canales...\n")
    
    for json_file in json_files:
        stats['total'] += 1
        
        try:
            with open(json_file, 'r', encoding='utf-8') as f:
                config = json.load(f)
            
            errors = []
            
            # Validar campos requeridos
            if not config.get('id'):
                errors.append("Falta campo 'id'")
            
            if not config.get('name'):
                errors.append("Falta campo 'name'")
            
            # Validar input
            if not config.get('input'):
                errors.append("Falta seccion 'input'")
            else:
                input_type = config['input'].get('type', '').lower()
                if input_type not in valid_input_types:
                    errors.append(f"Input type '{input_type}' no valido")
                else:
                    stats['protocols'][input_type] += 1
                
                if not config['input'].get('url'):
                    errors.append("Falta 'input.url'")
            
            # Validar output
            if not config.get('output'):
                errors.append("Falta seccion 'output'")
            else:
                output_type = config['output'].get('type', '').lower()
                if output_type not in valid_output_types:
                    errors.append(f"Output type '{output_type}' no valido")
                
                if not config['output'].get('url'):
                    errors.append("Falta 'output.url'")
                else:
                    # Extraer IP multicast
                    url = config['output']['url']
                    if url.startswith('udp://'):
                        ip = url.split('//')[1].split(':')[0]
                        if ip.startswith('232.'):
                            multicast_prefix = '.'.join(ip.split('.')[:3])
                            stats['multicast_ranges'][multicast_prefix] += 1
                
                # Verificar local_interface
                local_interface = config['output'].get('local_interface')
                if not local_interface:
                    errors.append("Falta 'output.local_interface'")
                else:
                    stats['local_interfaces'][local_interface] += 1
                    if local_interface != '192.168.2.140':
                        errors.append(f"Local interface '{local_interface}' no es 192.168.2.140")
                
                ttl = config['output'].get('ttl')
                if ttl != 32:
                    errors.append(f"TTL '{ttl}' no es 32 (recomendado)")
            
            # Validar modo
            mode = config.get('mode', 'passthrough')
            if mode not in valid_modes:
                errors.append(f"Modo '{mode}' no valido")
            
            # Reportar resultado
            if errors:
                stats['invalid'] += 1
                print(f"[!] {json_file.name}: ERRORES")
                for error in errors:
                    print(f"    - {error}")
                stats['errors'].append((json_file.name, errors))
            else:
                stats['valid'] += 1
                channel_name = config.get('name', 'unknown')[:20]
                input_type = config.get('input', {}).get('type', 'unknown')
                print(f"[+] {channel_name.ljust(20)} - {input_type} -> udp (local_interface: {local_interface})")
        
        except json.JSONDecodeError as e:
            stats['invalid'] += 1
            stats['errors'].append((json_file.name, [f"JSON invalido: {e}"]))
            print(f"[!] {json_file.name}: JSON INVALIDO - {e}")
        except Exception as e:
            stats['invalid'] += 1
            stats['errors'].append((json_file.name, [f"Error: {e}"]))
            print(f"[!] {json_file.name}: ERROR - {e}")
    
    # Resumen
    print(f"\n{'='*60}")
    print(f"[*] RESUMEN DE VALIDACION")
    print(f"{'='*60}")
    print(f"Total de archivos:    {stats['total']}")
    print(f"Validos:              {stats['valid']} ({stats['valid']*100//stats['total']}%)")
    print(f"Invalidos:            {stats['invalid']} ({stats['invalid']*100//stats['total']}%)")
    
    print(f"\n[*] PROTOCOLOS DE ENTRADA DETECTADOS:")
    for protocol, count in sorted(stats['protocols'].items()):
        percentage = count * 100 // stats['valid']
        print(f"    {protocol.upper():6} : {count:2} canales ({percentage:2}%)")
    
    print(f"\n[*] INTERFACES LOCALES UTILIZADAS:")
    for interface, count in sorted(stats['local_interfaces'].items()):
        status = "[OK]" if interface == '192.168.2.140' else "[!]"
        print(f"    {status} {interface} : {count} canales")
    
    print(f"\n[*] RANGOS MULTICAST UTILIZADOS:")
    for multicast_range, count in sorted(stats['multicast_ranges'].items()):
        print(f"    {multicast_range}.* : {count} canales")
    
    if stats['errors']:
        print(f"\n[!] ERRORES ENCONTRADOS:")
        for filename, errors in stats['errors']:
            print(f"    {filename}:")
            for error in errors:
                print(f"      - {error}")
    
    print(f"\n[OK] Validacion completada!\n")
    
    return stats['invalid'] == 0

if __name__ == '__main__':
    success = validate_channels()
    exit(0 if success else 1)
