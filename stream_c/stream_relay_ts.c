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
#include <stdarg.h>

#include <libavformat/avformat.h>
#include <libavcodec/avcodec.h>
#include <libavcodec/bsf.h>
#include <libavutil/time.h>
#include <libavutil/opt.h>

#define MAX_STREAMS 100
#define STATS_INTERVAL 10

typedef struct {
    unsigned long packets_sent;
    unsigned long bytes_sent;
    unsigned long errors;
    time_t start_time;
    int active;
    int reconnects;
} StreamStats;

typedef struct {
    int id;
    char input_url[512];
    char output_ip[64];
    int output_port;
    int running;
    pthread_t thread;
    StreamStats stats;
} StreamConfig;

static StreamConfig streams[MAX_STREAMS];
static int stream_count = 0;
static int program_running = 1;
static pthread_mutex_t stats_mutex = PTHREAD_MUTEX_INITIALIZER;
static int verbose = 0;

static const char* av_errstr_buf(int err, char *buf, size_t bufsz) {
    if (av_strerror(err, buf, bufsz) < 0) {
        snprintf(buf, bufsz, "Unknown error %d", err);
    }
    return buf;
}

static void log_info(const char *fmt, ...) {
    if (!verbose) return;
    va_list ap; va_start(ap, fmt); vfprintf(stderr, fmt, ap); va_end(ap);
}
static void log_warn(const char *fmt, ...) { va_list ap; va_start(ap, fmt); vfprintf(stderr, fmt, ap); va_end(ap); }
static void log_error(const char *fmt, ...) { va_list ap; va_start(ap, fmt); vfprintf(stderr, fmt, ap); va_end(ap); }

/* ================= STATS ================= */

void print_stats() {
    pthread_mutex_lock(&stats_mutex);
    printf("\n========= ESTADÍSTICAS =========\n");
    for (int i = 0; i < stream_count; i++) {
        if (!streams[i].stats.active) continue;
        time_t up = time(NULL) - streams[i].stats.start_time;
        double mbps = (up > 0) ?
            (streams[i].stats.bytes_sent * 8.0) / (up * 1000000.0) : 0;

        printf("[Stream %d] %.2f Mbps | Pkts: %lu | Err: %lu | Reconn: %d\n",
            i, mbps,
            streams[i].stats.packets_sent,
            streams[i].stats.errors,
            streams[i].stats.reconnects);
    }
    printf("===============================\n");
    pthread_mutex_unlock(&stats_mutex);
}

void* stats_thread(void* arg) {
    while (program_running) {
        sleep(STATS_INTERVAL);
        print_stats();
    }
    return NULL;
}

/* ================= STREAM THREAD ================= */

void* stream_thread(void* arg) {
    StreamConfig* cfg = (StreamConfig*)arg;
    AVFormatContext *in = NULL, *out = NULL;
    AVPacket *pkt = NULL;
    AVBSFContext *bsf = NULL;
    int video_index = -1;
    int got_keyframe = 0;

reconnect:
    if (!cfg->running || !program_running) goto end;

    pthread_mutex_lock(&stats_mutex);
    cfg->stats.active = 1;
    cfg->stats.start_time = time(NULL);
    cfg->stats.reconnects++;
    pthread_mutex_unlock(&stats_mutex);

    AVDictionary *opts = NULL;
    av_dict_set(&opts, "latency", "200000", 0);
    av_dict_set(&opts, "rcvbuf", "20000000", 0);
    av_dict_set(&opts, "peerlatency", "200000", 0);

    int ret = avformat_open_input(&in, cfg->input_url, NULL, &opts);
    if (ret < 0) {
        char errbuf[128]; av_strerror(ret, errbuf, sizeof(errbuf));
        log_error("[Stream %d] Error abriendo input '%s': %s\n", cfg->id, cfg->input_url, errbuf);
        av_dict_free(&opts);
        sleep(3);
        goto reconnect;
    }
    av_dict_free(&opts);

    ret = avformat_find_stream_info(in, NULL);
    if (ret < 0) { char errbuf[128]; av_strerror(ret, errbuf, sizeof(errbuf)); log_error("[Stream %d] Error leyendo metadata del stream: %s\n", cfg->id, errbuf); goto cleanup; }

    ret = avformat_alloc_output_context2(&out, NULL, "mpegts", NULL);
    if (ret < 0 || !out) { log_error("[Stream %d] Error creando contexto de salida\n", cfg->id); goto cleanup; }

    for (unsigned i = 0; i < in->nb_streams; i++) {
        AVStream *is = in->streams[i];
        AVStream *os = avformat_new_stream(out, NULL);
        avcodec_parameters_copy(os->codecpar, is->codecpar);
        os->time_base = is->time_base;

        if (is->codecpar->codec_type == AVMEDIA_TYPE_VIDEO &&
            is->codecpar->codec_id == AV_CODEC_ID_H264) {
            video_index = i;
        }
    }

    char out_url[256];

    snprintf(out_url, sizeof(out_url),
            "udp://%s:%d?pkt_size=1316",
            cfg->output_ip, cfg->output_port);

    struct in_addr outaddr;
    if (inet_pton(AF_INET, cfg->output_ip, &outaddr) != 1) {
        log_error("[Stream %d] IP de salida invalida: %s\n", cfg->id, cfg->output_ip);
        goto cleanup;
    }
    unsigned char first_octet = (ntohl(outaddr.s_addr) >> 24) & 0xFF;
    if (!(first_octet >= 224 && first_octet <= 239)) {
        log_warn("[Stream %d] Output IP %s no parece ser multicast (224.0.0.0/4). Esto puede estar bien si usas unicast.\n", cfg->id, cfg->output_ip);
    }

    ret = avio_open(&out->pb, out_url, AVIO_FLAG_WRITE);
    if (ret < 0) { char errbuf[128]; av_strerror(ret, errbuf, sizeof(errbuf)); log_error("[Stream %d] Error abriendo salida UDP '%s': %s\n", cfg->id, out_url, errbuf); goto cleanup; }

    ret = avformat_write_header(out, NULL);
    if (ret < 0) { char errbuf[128]; av_strerror(ret, errbuf, sizeof(errbuf)); log_error("[Stream %d] Error escribiendo cabecera de salida: %s\n", cfg->id, errbuf); goto cleanup; }

    /* ===== Bitstream filter H264 ===== */
    if (video_index >= 0) {
        const AVBitStreamFilter *filter = av_bsf_get_by_name("h264_mp4toannexb");
        if (!filter) {
            log_warn("[Stream %d] Bitstream filter h264_mp4toannexb no disponible\n", cfg->id);
        } else {
            ret = av_bsf_alloc(filter, &bsf);
            if (ret < 0) { char errbuf[128]; av_strerror(ret, errbuf, sizeof(errbuf)); log_error("[Stream %d] av_bsf_alloc failed: %s\n", cfg->id, errbuf); goto cleanup; }
            ret = avcodec_parameters_copy(bsf->par_in, in->streams[video_index]->codecpar);
            if (ret < 0) { log_warn("[Stream %d] avcodec_parameters_copy to bsf failed\n", cfg->id); }
            bsf->time_base_in = in->streams[video_index]->time_base;
            ret = av_bsf_init(bsf);
            if (ret < 0) { char errbuf[128]; av_strerror(ret, errbuf, sizeof(errbuf)); log_error("[Stream %d] av_bsf_init failed: %s\n", cfg->id, errbuf); goto cleanup; }
        }
    }

    pkt = av_packet_alloc();

    while (program_running && cfg->running) {
        int r = av_read_frame(in, pkt);
        if (r < 0) {
            if (r == AVERROR_EOF) {
                log_info("[Stream %d] EOF\n", cfg->id);
                break;
            }
            char errbuf[128]; av_strerror(r, errbuf, sizeof(errbuf));
            log_error("[Stream %d] Error leyendo frame: %s\n", cfg->id, errbuf);
            pthread_mutex_lock(&stats_mutex); cfg->stats.errors++; pthread_mutex_unlock(&stats_mutex);
            usleep(10000);
            break;
        }

        if (pkt->stream_index == video_index) {
            if (!got_keyframe) {
                if (!(pkt->flags & AV_PKT_FLAG_KEY)) {
                    av_packet_unref(pkt);
                    continue;
                }
                got_keyframe = 1;
            }

            r = av_bsf_send_packet(bsf, pkt);
            if (r < 0) { char errbuf[128]; av_strerror(r, errbuf, sizeof(errbuf)); log_error("[Stream %d] av_bsf_send_packet failed: %s\n", cfg->id, errbuf); av_packet_unref(pkt); continue; }
            while ((r = av_bsf_receive_packet(bsf, pkt)) == 0) {
                int w = av_interleaved_write_frame(out, pkt);
                if (w < 0) { char errbuf[128]; av_strerror(w, errbuf, sizeof(errbuf)); log_error("[Stream %d] Error escribiendo frame despues de bsf: %s\n", cfg->id, errbuf); pthread_mutex_lock(&stats_mutex); cfg->stats.errors++; pthread_mutex_unlock(&stats_mutex); av_packet_unref(pkt); goto cleanup; }
                pthread_mutex_lock(&stats_mutex); cfg->stats.packets_sent++; cfg->stats.bytes_sent += pkt->size; pthread_mutex_unlock(&stats_mutex);
                av_packet_unref(pkt);
            }
            if (r != AVERROR(EAGAIN) && r != AVERROR_EOF && r < 0) { char errbuf[128]; av_strerror(r, errbuf, sizeof(errbuf)); log_error("[Stream %d] av_bsf_receive_packet failed: %s\n", cfg->id, errbuf); }
            continue;
        }

        int w = av_interleaved_write_frame(out, pkt);
        if (w < 0) { char errbuf[128]; av_strerror(w, errbuf, sizeof(errbuf)); log_error("[Stream %d] Error escribiendo frame: %s\n", cfg->id, errbuf); pthread_mutex_lock(&stats_mutex); cfg->stats.errors++; pthread_mutex_unlock(&stats_mutex); av_packet_unref(pkt); goto cleanup; }
        av_packet_unref(pkt);
    }

cleanup:
    if (pkt) av_packet_free(&pkt);
    if (bsf) av_bsf_free(&bsf);
    if (out) {
        av_write_trailer(out);
        if (out->pb) avio_closep(&out->pb);
        avformat_free_context(out);
    }
    if (in) avformat_close_input(&in);

    sleep(3);
    goto reconnect;

end:
    pthread_mutex_lock(&stats_mutex);
    cfg->stats.active = 0;
    pthread_mutex_unlock(&stats_mutex);
    return NULL;
}

/* ================= MAIN ================= */

void signal_handler(int s) {
    program_running = 0;
    for (int i = 0; i < stream_count; i++)
        streams[i].running = 0;
}

int load_config(const char *f) {
    FILE *fp = fopen(f, "r");
    if (!fp) {
        fprintf(stderr, "Error abriendo archivo de configuración: %s (%s)\n", f, strerror(errno));
        return -1;
    }

    char line[1024];
    stream_count = 0;

    while (fgets(line, sizeof(line), fp) && stream_count < MAX_STREAMS) {
        // Trim leading whitespace
        char *p = line;
        while (*p == ' ' || *p == '\t') p++;
        // Skip comments and empty lines
        if (*p == '#' || *p == '\n' || *p == '\0') continue;

        // Trim trailing whitespace/newlines
        char *end = p + strlen(p) - 1;
        while (end > p && (*end == '\n' || *end == '\r' || *end == ' ' || *end == '\t')) { *end = '\0'; end--; }

        char in[512], ip[64];
        int port;
        if (sscanf(p, "%511s %63s %d", in, ip, &port) == 3) {
            // Validate IP and port
            struct in_addr addr;
            if (inet_pton(AF_INET, ip, &addr) != 1) {
                fprintf(stderr, "Skipping invalid IP in config: %s\n", p);
                continue;
            }
            if (port <= 0 || port > 65535) {
                fprintf(stderr, "Skipping invalid port in config: %s\n", p);
                continue;
            }

            streams[stream_count].id = stream_count;
            strncpy(streams[stream_count].input_url, in, sizeof(streams[stream_count].input_url)-1);
            streams[stream_count].input_url[sizeof(streams[stream_count].input_url)-1] = '\0';
            strncpy(streams[stream_count].output_ip, ip, sizeof(streams[stream_count].output_ip)-1);
            streams[stream_count].output_ip[sizeof(streams[stream_count].output_ip)-1] = '\0';
            streams[stream_count].output_port = port;
            streams[stream_count].running = 1;
            memset(&streams[stream_count].stats, 0, sizeof(StreamStats));
            stream_count++;
        } else {
            fprintf(stderr, "Skipping invalid config line: %s\n", p);
        }
    }

    fclose(fp);

    if (stream_count == 0) {
        fprintf(stderr, "No streams configurados en %s\n", f);
    } else {
        fprintf(stderr, "Configuración cargada: %d streams\n", stream_count);
    }

    return stream_count;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        printf("Uso: %s [-v] config.txt\n", argv[0]);
        return 1;
    }

    int argi = 1;
    if (strcmp(argv[argi], "-v") == 0 || strcmp(argv[argi], "--verbose") == 0) {
        verbose = 1;
        argi++;
        if (argi >= argc) { fprintf(stderr, "FATAL: falta archivo de configuración\n"); return 1; }
    }

    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);

    av_log_set_level(verbose ? AV_LOG_INFO : AV_LOG_ERROR);

    int cfg_res = load_config(argv[argi]);
    if (cfg_res < 0) {
        fprintf(stderr, "FATAL: no se pudo abrir archivo de configuración: %s\n", argv[1]);
        return 2;
    }
    if (cfg_res == 0) {
        fprintf(stderr, "FATAL: no hay streams válidos en el archivo de configuración: %s\n", argv[1]);
        return 3;
    }

    pthread_t st;
    pthread_create(&st, NULL, stats_thread, NULL);

    for (int i = 0; i < stream_count; i++)
        pthread_create(&streams[i].thread, NULL,
                       stream_thread, &streams[i]);

    for (int i = 0; i < stream_count; i++)
        pthread_join(streams[i].thread, NULL);

    program_running = 0;
    pthread_join(st, NULL);
    return 0;
}
