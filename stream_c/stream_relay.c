#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <pthread.h>
#include <unistd.h>
#include <signal.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <errno.h>
#include <time.h>

#include <libavformat/avformat.h>
#include <libavcodec/avcodec.h>
#include <libavutil/time.h>
#include <libavutil/opt.h>

#define MAX_STREAMS 100
#define BUFFER_SIZE 188 * 7  // 7 paquetes TS (1316 bytes)
#define UDP_PACKET_SIZE 1316  // 7 paquetes TS estándar
#define STATS_INTERVAL 10  // segundos

// Estructura para estadísticas de cada stream
typedef struct {
    char name[256];
    unsigned long packets_sent;
    unsigned long bytes_sent;
    unsigned long errors;
    time_t start_time;
    int active;
    double bitrate;
} StreamStats;

// Estructura para configuración de cada stream
typedef struct {
    int id;
    char input_url[512];
    char output_ip[64];
    int output_port;
    int running;
    pthread_t thread;
    StreamStats stats;
} StreamConfig;

// Variables globales
static StreamConfig streams[MAX_STREAMS];
static int stream_count = 0;
static int program_running = 1;
static pthread_mutex_t stats_mutex = PTHREAD_MUTEX_INITIALIZER;

// Función para mostrar estadísticas
void print_stats() {
    pthread_mutex_lock(&stats_mutex);
    
    printf("\n========== ESTADÍSTICAS DE STREAMS ==========\n");
    printf("Tiempo: %s", ctime(&(time_t){time(NULL)}));
    
    for (int i = 0; i < stream_count; i++) {
        if (streams[i].stats.active) {
            time_t elapsed = time(NULL) - streams[i].stats.start_time;
            double mbps = (streams[i].stats.bytes_sent * 8.0) / (elapsed * 1000000.0);
            
            printf("\n[Stream %d] %s\n", i, streams[i].stats.name);
            printf("  Input:  %s\n", streams[i].input_url);
            printf("  Output: %s:%d\n", streams[i].output_ip, streams[i].output_port);
            printf("  Paquetes: %lu | Bytes: %lu | Errores: %lu\n", 
                   streams[i].stats.packets_sent, 
                   streams[i].stats.bytes_sent,
                   streams[i].stats.errors);
            printf("  Bitrate: %.2f Mbps | Uptime: %ld seg\n", mbps, elapsed);
        }
    }
    
    printf("============================================\n\n");
    pthread_mutex_unlock(&stats_mutex);
}

// Thread para mostrar estadísticas periódicamente
void* stats_thread(void* arg) {
    while (program_running) {
        sleep(STATS_INTERVAL);
        print_stats();
    }
    return NULL;
}

// Función principal de re-streaming (sin transcodificación)
void* stream_relay_thread(void* arg) {
    StreamConfig* config = (StreamConfig*)arg;
    AVFormatContext* input_ctx = NULL;
    AVPacket* packet = NULL;
    int ret;
    int sockfd;
    struct sockaddr_in dest_addr;
    uint8_t buffer[UDP_PACKET_SIZE];
    int buffer_pos = 0;
    
    // Actualizar estadísticas
    pthread_mutex_lock(&stats_mutex);
    config->stats.active = 1;
    config->stats.start_time = time(NULL);
    snprintf(config->stats.name, sizeof(config->stats.name), "Stream_%d", config->id);
    pthread_mutex_unlock(&stats_mutex);
    
    printf("[Stream %d] Iniciando: %s -> %s:%d\n", 
           config->id, config->input_url, config->output_ip, config->output_port);
    
    // Crear socket UDP
    sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (sockfd < 0) {
        fprintf(stderr, "[Stream %d] Error creando socket: %s\n", config->id, strerror(errno));
        config->stats.active = 0;
        return NULL;
    }
    
    // Configurar dirección de destino
    memset(&dest_addr, 0, sizeof(dest_addr));
    dest_addr.sin_family = AF_INET;
    dest_addr.sin_port = htons(config->output_port);
    inet_pton(AF_INET, config->output_ip, &dest_addr.sin_addr);
    
    // Configurar opciones de socket para mejor rendimiento
    int sendbuf = 2 * 1024 * 1024; // 2MB buffer
    setsockopt(sockfd, SOL_SOCKET, SO_SNDBUF, &sendbuf, sizeof(sendbuf));
    
    // Abrir input stream
    AVDictionary* opts = NULL;
    av_dict_set(&opts, "timeout", "5000000", 0); // 5 segundos timeout
    av_dict_set(&opts, "buffer_size", "1048576", 0); // 1MB buffer
    av_dict_set(&opts, "rtbufsize", "150M", 0); // Buffer para RTMP/SRT
    av_dict_set(&opts, "analyzeduration", "2000000", 0);
    av_dict_set(&opts, "probesize", "1000000", 0);
    
    ret = avformat_open_input(&input_ctx, config->input_url, NULL, &opts);
    av_dict_free(&opts);
    
    if (ret < 0) {
        char errbuf[128];
        av_strerror(ret, errbuf, sizeof(errbuf));
        fprintf(stderr, "[Stream %d] Error abriendo input: %s\n", config->id, errbuf);
        close(sockfd);
        config->stats.active = 0;
        return NULL;
    }
    
    // Obtener información del stream
    ret = avformat_find_stream_info(input_ctx, NULL);
    if (ret < 0) {
        fprintf(stderr, "[Stream %d] Error obteniendo info del stream\n", config->id);
        avformat_close_input(&input_ctx);
        close(sockfd);
        config->stats.active = 0;
        return NULL;
    }
    
    printf("[Stream %d] Stream abierto correctamente. Formato: %s\n", 
           config->id, input_ctx->iformat->name);
    
    // Crear paquete
    packet = av_packet_alloc();
    if (!packet) {
        fprintf(stderr, "[Stream %d] Error allocando paquete\n", config->id);
        avformat_close_input(&input_ctx);
        close(sockfd);
        config->stats.active = 0;
        return NULL;
    }
    
    // Loop principal de lectura y envío
    while (config->running && program_running) {
        ret = av_read_frame(input_ctx, packet);
        
        if (ret < 0) {
            if (ret == AVERROR_EOF) {
                printf("[Stream %d] Fin del stream\n", config->id);
                break;
            } else if (ret == AVERROR(EAGAIN)) {
                usleep(1000);
                continue;
            } else {
                char errbuf[128];
                av_strerror(ret, errbuf, sizeof(errbuf));
                fprintf(stderr, "[Stream %d] Error leyendo frame: %s\n", config->id, errbuf);
                pthread_mutex_lock(&stats_mutex);
                config->stats.errors++;
                pthread_mutex_unlock(&stats_mutex);
                usleep(10000);
                continue;
            }
        }
        
        // Enviar datos directamente (sin transcodificación)
        // Para TS streams, podemos enviar directamente
        // Para otros formatos, necesitamos extraer los datos útiles
        
        if (packet->size > 0) {
            // Acumular en buffer para enviar en paquetes UDP óptimos
            int remaining = packet->size;
            int offset = 0;
            
            while (remaining > 0) {
                int to_copy = (remaining < (UDP_PACKET_SIZE - buffer_pos)) ? 
                              remaining : (UDP_PACKET_SIZE - buffer_pos);
                
                memcpy(buffer + buffer_pos, packet->data + offset, to_copy);
                buffer_pos += to_copy;
                offset += to_copy;
                remaining -= to_copy;
                
                // Enviar cuando el buffer esté lleno o sea el último paquete
                if (buffer_pos >= UDP_PACKET_SIZE || remaining == 0) {
                    ssize_t sent = sendto(sockfd, buffer, buffer_pos, 0,
                                         (struct sockaddr*)&dest_addr, sizeof(dest_addr));
                    
                    if (sent < 0) {
                        pthread_mutex_lock(&stats_mutex);
                        config->stats.errors++;
                        pthread_mutex_unlock(&stats_mutex);
                    } else {
                        pthread_mutex_lock(&stats_mutex);
                        config->stats.packets_sent++;
                        config->stats.bytes_sent += sent;
                        pthread_mutex_unlock(&stats_mutex);
                    }
                    
                    buffer_pos = 0;
                }
            }
        }
        
        av_packet_unref(packet);
    }
    
    // Cleanup
    av_packet_free(&packet);
    avformat_close_input(&input_ctx);
    close(sockfd);
    
    pthread_mutex_lock(&stats_mutex);
    config->stats.active = 0;
    pthread_mutex_unlock(&stats_mutex);
    
    printf("[Stream %d] Thread finalizado\n", config->id);
    return NULL;
}

// Cargar configuración desde archivo
int load_config(const char* filename) {
    FILE* fp = fopen(filename, "r");
    if (!fp) {
        fprintf(stderr, "Error abriendo archivo de configuración: %s\n", filename);
        return -1;
    }
    
    char line[1024];
    stream_count = 0;
    
    while (fgets(line, sizeof(line), fp) && stream_count < MAX_STREAMS) {
        // Saltar comentarios y líneas vacías
        if (line[0] == '#' || line[0] == '\n') continue;
        
        char input[512], output_ip[64];
        int output_port;
        
        if (sscanf(line, "%s %s %d", input, output_ip, &output_port) == 3) {
            streams[stream_count].id = stream_count;
            strncpy(streams[stream_count].input_url, input, sizeof(streams[stream_count].input_url) - 1);
            strncpy(streams[stream_count].output_ip, output_ip, sizeof(streams[stream_count].output_ip) - 1);
            streams[stream_count].output_port = output_port;
            streams[stream_count].running = 1;
            
            memset(&streams[stream_count].stats, 0, sizeof(StreamStats));
            
            stream_count++;
        }
    }
    
    fclose(fp);
    printf("Configuración cargada: %d streams\n", stream_count);
    return stream_count;
}

// Manejador de señales
void signal_handler(int sig) {
    printf("\nSeñal recibida (%d). Deteniendo streams...\n", sig);
    program_running = 0;
    
    for (int i = 0; i < stream_count; i++) {
        streams[i].running = 0;
    }
}

int main(int argc, char* argv[]) {
    pthread_t stats_tid;
    
    printf("===========================================\n");
    printf("  Multi-Stream Relay para CATV-2\n");
    printf("  Optimizado para bajo consumo de CPU\n");
    printf("===========================================\n\n");
    
    if (argc < 2) {
        printf("Uso: %s <archivo_configuracion>\n\n", argv[0]);
        printf("Formato del archivo de configuración:\n");
        printf("# Comentarios comienzan con #\n");
        printf("INPUT_URL OUTPUT_IP OUTPUT_PORT\n\n");
        printf("Ejemplos:\n");
        printf("srt://source.com:9000 239.1.1.1 5000\n");
        printf("rtmp://server.com/live/stream1 239.1.1.2 5001\n");
        printf("http://server.com/stream.m3u8 239.1.1.3 5002\n");
        printf("udp://@:1234 239.1.1.4 5003\n\n");
        return 1;
    }
    
    // Registrar manejador de señales
    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);
    
    // Inicializar FFmpeg
    av_log_set_level(AV_LOG_WARNING);
    
    // Cargar configuración
    if (load_config(argv[1]) <= 0) {
        fprintf(stderr, "Error: No se pudieron cargar streams del archivo de configuración\n");
        return 1;
    }
    
    // Iniciar thread de estadísticas
    pthread_create(&stats_tid, NULL, stats_thread, NULL);
    
    // Iniciar threads de streaming
    for (int i = 0; i < stream_count; i++) {
        if (pthread_create(&streams[i].thread, NULL, stream_relay_thread, &streams[i]) != 0) {
            fprintf(stderr, "Error creando thread para stream %d\n", i);
            streams[i].running = 0;
        } else {
            usleep(100000); // 100ms entre inicio de streams
        }
    }
    
    printf("\nTodos los streams iniciados. Presiona Ctrl+C para detener.\n\n");
    
    // Esperar a que terminen los threads
    for (int i = 0; i < stream_count; i++) {
        if (streams[i].thread) {
            pthread_join(streams[i].thread, NULL);
        }
    }
    
    program_running = 0;
    pthread_join(stats_tid, NULL);
    
    // Mostrar estadísticas finales
    printf("\n========== ESTADÍSTICAS FINALES ==========\n");
    print_stats();
    
    printf("\nPrograma finalizado correctamente.\n");
    return 0;
}
