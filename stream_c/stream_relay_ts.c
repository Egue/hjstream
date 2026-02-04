#include <stdio.h>
<stdlib.h>
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
#define BUFFER_SIZE 188 * 7  // 7 paquetes TS
#define UDP_PACKET_SIZE 1316
#define STATS_INTERVAL 10

typedef struct {
    char name[256];
    unsigned long packets_sent;
    unsigned long bytes_sent;
    unsigned long errors;
    time_t start_time;
    int active;
    double bitrate;
    int reconnect_count;
} StreamStats;

typedef struct {
    int id;
    char input_url[512];
    char output_ip[64];
    int output_port;
    int running;
    int remux_to_ts;  // 1 para forzar remux a MPEGTS
    pthread_t thread;
    StreamStats stats;
} StreamConfig;

static StreamConfig streams[MAX_STREAMS];
static int stream_count = 0;
static int program_running = 1;
static pthread_mutex_t stats_mutex = PTHREAD_MUTEX_INITIALIZER;

void print_stats() {
    pthread_mutex_lock(&stats_mutex);
    
    printf("\n========== ESTADÍSTICAS DE STREAMS ==========\n");
    time_t now = time(NULL);
    printf("Tiempo: %s", ctime(&now));
    
    for (int i = 0; i < stream_count; i++) {
        if (streams[i].stats.active) {
            time_t elapsed = time(NULL) - streams[i].stats.start_time;
            double mbps = elapsed > 0 ? (streams[i].stats.bytes_sent * 8.0) / (elapsed * 1000000.0) : 0;
            
            printf("\n[Stream %d] %s\n", i, streams[i].stats.name);
            printf("  Input:  %s\n", streams[i].input_url);
            printf("  Output: %s:%d (UDP)\n", streams[i].output_ip, streams[i].output_port);
            printf("  Paquetes: %lu | Bytes: %lu MB | Errores: %lu\n", 
                   streams[i].stats.packets_sent, 
                   streams[i].stats.bytes_sent / (1024 * 1024),
                   streams[i].stats.errors);
            printf("  Bitrate: %.2f Mbps | Uptime: %ld seg | Reconexiones: %d\n", 
                   mbps, elapsed, streams[i].stats.reconnect_count);
        }
    }
    
    printf("============================================\n\n");
    pthread_mutex_unlock(&stats_mutex);
}

void* stats_thread(void* arg) {
    while (program_running) {
        sleep(STATS_INTERVAL);
        print_stats();
    }
    return NULL;
}

// Versión mejorada con remuxing a MPEGTS
void* stream_relay_remux_thread(void* arg) {
    StreamConfig* config = (StreamConfig*)arg;
    AVFormatContext* input_ctx = NULL;
    AVFormatContext* output_ctx = NULL;
    AVPacket* packet = NULL;
    AVDictionary* opts = NULL;
    int ret;
    int sockfd;
    struct sockaddr_in dest_addr;
    char output_url[256];
    int64_t last_pts[128] = {0}; // Para tracking de PTS por stream
    
    pthread_mutex_lock(&stats_mutex);
    config->stats.active = 1;
    config->stats.start_time = time(NULL);
    snprintf(config->stats.name, sizeof(config->stats.name), "Stream_%d", config->id);
    pthread_mutex_unlock(&stats_mutex);
    
    printf("[Stream %d] Iniciando con remux a MPEGTS: %s -> %s:%d\n", 
           config->id, config->input_url, config->output_ip, config->output_port);
    
reconnect:
    if (!config->running || !program_running) goto cleanup;
    
    // Crear socket UDP
    sockfd = socket(AF_INET, SOCK_DGRAM, 0);
    if (sockfd < 0) {
        fprintf(stderr, "[Stream %d] Error creando socket: %s\n", config->id, strerror(errno));
        pthread_mutex_lock(&stats_mutex);
        config->stats.errors++;
        pthread_mutex_unlock(&stats_mutex);
        sleep(5);
        goto reconnect;
    }
    
    // Configurar socket
    int sendbuf = 4 * 1024 * 1024; // 4MB buffer
    setsockopt(sockfd, SOL_SOCKET, SO_SNDBUF, &sendbuf, sizeof(sendbuf));
    
    memset(&dest_addr, 0, sizeof(dest_addr));
    dest_addr.sin_family = AF_INET;
    dest_addr.sin_port = htons(config->output_port);
    inet_pton(AF_INET, config->output_ip, &dest_addr.sin_addr);
    
    // Configurar opciones de input
    av_dict_set(&opts, "timeout", "10000000", 0);
    av_dict_set(&opts, "buffer_size", "2097152", 0);
    av_dict_set(&opts, "rtbufsize", "200M", 0);
    av_dict_set(&opts, "analyzeduration", "3000000", 0);
    av_dict_set(&opts, "probesize", "2000000", 0);
    av_dict_set(&opts, "fflags", "nobuffer", 0);
    av_dict_set(&opts, "flags", "low_delay", 0);
    
    // Abrir input
    ret = avformat_open_input(&input_ctx, config->input_url, NULL, &opts);
    av_dict_free(&opts);
    
    if (ret < 0) {
        char errbuf[128];
        av_strerror(ret, errbuf, sizeof(errbuf));
        fprintf(stderr, "[Stream %d] Error abriendo input: %s\n", config->id, errbuf);
        close(sockfd);
        pthread_mutex_lock(&stats_mutex);
        config->stats.errors++;
        config->stats.reconnect_count++;
        pthread_mutex_unlock(&stats_mutex);
        sleep(5);
        goto reconnect;
    }
    
    ret = avformat_find_stream_info(input_ctx, NULL);
    if (ret < 0) {
        fprintf(stderr, "[Stream %d] Error obteniendo stream info\n", config->id);
        avformat_close_input(&input_ctx);
        close(sockfd);
        pthread_mutex_lock(&stats_mutex);
        config->stats.errors++;
        pthread_mutex_unlock(&stats_mutex);
        sleep(5);
        goto reconnect;
    }
    
    printf("[Stream %d] Input abierto: %s (%d streams)\n", 
           config->id, input_ctx->iformat->name, input_ctx->nb_streams);
    
    // Crear output context para UDP MPEGTS
    snprintf(output_url, sizeof(output_url), "udp://%s:%d?pkt_size=1316&buffer_size=65535",
             config->output_ip, config->output_port);
    
    ret = avformat_alloc_output_context2(&output_ctx, NULL, "mpegts", output_url);
    if (ret < 0 || !output_ctx) {
        fprintf(stderr, "[Stream %d] Error creando output context\n", config->id);
        avformat_close_input(&input_ctx);
        close(sockfd);
        pthread_mutex_lock(&stats_mutex);
        config->stats.errors++;
        pthread_mutex_unlock(&stats_mutex);
        sleep(5);
        goto reconnect;
    }
    
    // Copiar streams del input al output
    for (unsigned int i = 0; i < input_ctx->nb_streams; i++) {
        AVStream* in_stream = input_ctx->streams[i];
        AVStream* out_stream = avformat_new_stream(output_ctx, NULL);
        
        if (!out_stream) {
            fprintf(stderr, "[Stream %d] Error creando output stream\n", config->id);
            goto cleanup_contexts;
        }
        
        ret = avcodec_parameters_copy(out_stream->codecpar, in_stream->codecpar);
        if (ret < 0) {
            fprintf(stderr, "[Stream %d] Error copiando parámetros de codec\n", config->id);
            goto cleanup_contexts;
        }
        
        out_stream->codecpar->codec_tag = 0;
        out_stream->time_base = in_stream->time_base;
    }
    
    // Configurar opciones de MPEGTS output
    AVDictionary* output_opts = NULL;
    av_dict_set(&output_opts, "mpegts_flags", "initial_discontinuity", 0);
    av_dict_set_int(output_ctx->priv_data, "muxrate", 0, 0);
    
    // Abrir output
    if (!(output_ctx->oformat->flags & AVFMT_NOFILE)) {
        ret = avio_open2(&output_ctx->pb, output_url, AVIO_FLAG_WRITE, NULL, &output_opts);
        av_dict_free(&output_opts);
        
        if (ret < 0) {
            char errbuf[128];
            av_strerror(ret, errbuf, sizeof(errbuf));
            fprintf(stderr, "[Stream %d] Error abriendo output: %s\n", config->id, errbuf);
            goto cleanup_contexts;
        }
    }
    
    // Escribir header
    ret = avformat_write_header(output_ctx, NULL);
    if (ret < 0) {
        fprintf(stderr, "[Stream %d] Error escribiendo header\n", config->id);
        goto cleanup_contexts;
    }
    
    printf("[Stream %d] Output MPEGTS configurado correctamente\n", config->id);
    
    // Allocar packet
    packet = av_packet_alloc();
    if (!packet) {
        fprintf(stderr, "[Stream %d] Error allocando packet\n", config->id);
        goto cleanup_contexts;
    }
    
    // Loop principal
    while (config->running && program_running) {
        ret = av_read_frame(input_ctx, packet);
        
        if (ret < 0) {
            if (ret == AVERROR_EOF) {
                printf("[Stream %d] Fin del stream, reconectando...\n", config->id);
                av_packet_unref(packet);
                break;
            } else if (ret == AVERROR(EAGAIN)) {
                usleep(1000);
                continue;
            } else {
                char errbuf[128];
                av_strerror(ret, errbuf, sizeof(errbuf));
                fprintf(stderr, "[Stream %d] Error leyendo: %s\n", config->id, errbuf);
                pthread_mutex_lock(&stats_mutex);
                config->stats.errors++;
                pthread_mutex_unlock(&stats_mutex);
                av_packet_unref(packet);
                break;
            }
        }
        
        // Rescalar timestamps
        AVStream* in_stream = input_ctx->streams[packet->stream_index];
        AVStream* out_stream = output_ctx->streams[packet->stream_index];
        
        packet->pts = av_rescale_q_rnd(packet->pts, in_stream->time_base, 
                                       out_stream->time_base, 
                                       AV_ROUND_NEAR_INF | AV_ROUND_PASS_MINMAX);
        packet->dts = av_rescale_q_rnd(packet->dts, in_stream->time_base, 
                                       out_stream->time_base, 
                                       AV_ROUND_NEAR_INF | AV_ROUND_PASS_MINMAX);
        packet->duration = av_rescale_q(packet->duration, in_stream->time_base, 
                                        out_stream->time_base);
        packet->pos = -1;
        
        // Escribir packet
        ret = av_interleaved_write_frame(output_ctx, packet);
        
        if (ret < 0) {
            pthread_mutex_lock(&stats_mutex);
            config->stats.errors++;
            pthread_mutex_unlock(&stats_mutex);
        } else {
            pthread_mutex_lock(&stats_mutex);
            config->stats.packets_sent++;
            config->stats.bytes_sent += packet->size;
            pthread_mutex_unlock(&stats_mutex);
        }
        
        av_packet_unref(packet);
    }
    
    // Escribir trailer
    if (output_ctx) {
        av_write_trailer(output_ctx);
    }
    
cleanup_contexts:
    if (packet) av_packet_free(&packet);
    if (output_ctx) {
        if (output_ctx->pb) avio_closep(&output_ctx->pb);
        avformat_free_context(output_ctx);
    }
    if (input_ctx) avformat_close_input(&input_ctx);
    close(sockfd);
    
    if (config->running && program_running) {
        printf("[Stream %d] Reconectando en 5 segundos...\n", config->id);
        sleep(5);
        goto reconnect;
    }
    
cleanup:
    pthread_mutex_lock(&stats_mutex);
    config->stats.active = 0;
    pthread_mutex_unlock(&stats_mutex);
    
    printf("[Stream %d] Thread finalizado\n", config->id);
    return NULL;
}

int load_config(const char* filename) {
    FILE* fp = fopen(filename, "r");
    if (!fp) {
        fprintf(stderr, "Error abriendo configuración: %s\n", filename);
        return -1;
    }
    
    char line[1024];
    stream_count = 0;
    
    while (fgets(line, sizeof(line), fp) && stream_count < MAX_STREAMS) {
        if (line[0] == '#' || line[0] == '\n') continue;
        
        char input[512], output_ip[64];
        int output_port;
        
        if (sscanf(line, "%s %s %d", input, output_ip, &output_port) == 3) {
            streams[stream_count].id = stream_count;
            strncpy(streams[stream_count].input_url, input, sizeof(streams[stream_count].input_url) - 1);
            strncpy(streams[stream_count].output_ip, output_ip, sizeof(streams[stream_count].output_ip) - 1);
            streams[stream_count].output_port = output_port;
            streams[stream_count].running = 1;
            streams[stream_count].remux_to_ts = 1; // Siempre remux a TS
            
            memset(&streams[stream_count].stats, 0, sizeof(StreamStats));
            
            stream_count++;
        }
    }
    
    fclose(fp);
    printf("Configuración cargada: %d streams\n", stream_count);
    return stream_count;
}

void signal_handler(int sig) {
    printf("\nSeñal recibida (%d). Deteniendo...\n", sig);
    program_running = 0;
    
    for (int i = 0; i < stream_count; i++) {
        streams[i].running = 0;
    }
}

int main(int argc, char* argv[]) {
    pthread_t stats_tid;
    
    printf("===========================================\n");
    printf("  Multi-hjstreamc MPEGTS para CATV-2\n");
    printf("  Optimizado - Bajo CPU - Auto-reconexión\n");
    printf("===========================================\n\n");
    
    if (argc < 2) {
        printf("Uso: %s <config.txt>\n", argv[0]);
        return 1;
    }
    
    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);
    signal(SIGPIPE, SIG_IGN);
    
    av_log_set_level(AV_LOG_ERROR);
    
    if (load_config(argv[1]) <= 0) {
        fprintf(stderr, "Error cargando configuración\n");
        return 1;
    }
    
    pthread_create(&stats_tid, NULL, stats_thread, NULL);
    
    for (int i = 0; i < stream_count; i++) {
        if (pthread_create(&streams[i].thread, NULL, stream_relay_remux_thread, &streams[i]) != 0) {
            fprintf(stderr, "Error creando thread %d\n", i);
            streams[i].running = 0;
        } else {
            usleep(200000); // 200ms entre streams
        }
    }
    
    printf("\n%d streams activos. Ctrl+C para detener.\n\n", stream_count);
    
    for (int i = 0; i < stream_count; i++) {
        if (streams[i].thread) {
            pthread_join(streams[i].thread, NULL);
        }
    }
    
    program_running = 0;
    pthread_join(stats_tid, NULL);
    
    print_stats();
    printf("\nPrograma finalizado.\n");
    return 0;
}
