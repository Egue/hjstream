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

    if (avformat_open_input(&in, cfg->input_url, NULL, &opts) < 0) {
        av_dict_free(&opts);
        sleep(3);
        goto reconnect;
    }
    av_dict_free(&opts);

    if (avformat_find_stream_info(in, NULL) < 0)
        goto cleanup;

    avformat_alloc_output_context2(&out, NULL, "mpegts", NULL);
    if (!out) goto cleanup;

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

    if (avio_open(&out->pb,
        av_asprintf("udp://%s:%d?pkt_size=1316",
            cfg->output_ip, cfg->output_port),
        AVIO_FLAG_WRITE) < 0)
        goto cleanup;

    if (avformat_write_header(out, NULL) < 0)
        goto cleanup;

    /* ===== Bitstream filter H264 ===== */
    if (video_index >= 0) {
        const AVBitStreamFilter *filter =
            av_bsf_get_by_name("h264_mp4toannexb");
        av_bsf_alloc(filter, &bsf);
        avcodec_parameters_copy(
            bsf->par_in, in->streams[video_index]->codecpar);
        bsf->time_base_in = in->streams[video_index]->time_base;
        av_bsf_init(bsf);
    }

    pkt = av_packet_alloc();

    while (program_running && cfg->running) {
        if (av_read_frame(in, pkt) < 0)
            break;

        if (pkt->stream_index == video_index) {
            if (!got_keyframe) {
                if (!(pkt->flags & AV_PKT_FLAG_KEY)) {
                    av_packet_unref(pkt);
                    continue;
                }
                got_keyframe = 1;
            }

            av_bsf_send_packet(bsf, pkt);
            while (av_bsf_receive_packet(bsf, pkt) == 0) {
                av_interleaved_write_frame(out, pkt);
                pthread_mutex_lock(&stats_mutex);
                cfg->stats.packets_sent++;
                cfg->stats.bytes_sent += pkt->size;
                pthread_mutex_unlock(&stats_mutex);
                av_packet_unref(pkt);
            }
            continue;
        }

        av_interleaved_write_frame(out, pkt);
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
    if (!fp) return -1;

    char in[512], ip[64];
    int port;
    while (fscanf(fp, "%s %s %d", in, ip, &port) == 3) {
        streams[stream_count].id = stream_count;
        strcpy(streams[stream_count].input_url, in);
        strcpy(streams[stream_count].output_ip, ip);
        streams[stream_count].output_port = port;
        streams[stream_count].running = 1;
        memset(&streams[stream_count].stats, 0, sizeof(StreamStats));
        stream_count++;
    }
    fclose(fp);
    return stream_count;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        printf("Uso: %s config.txt\n", argv[0]);
        return 1;
    }

    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);

    av_log_set_level(AV_LOG_ERROR);

    if (load_config(argv[1]) <= 0)
        return 1;

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
