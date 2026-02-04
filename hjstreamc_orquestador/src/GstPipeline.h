#pragma once
#include <gst/gst.h>
#include <string>
#include <memory>
#include <atomic>

// Renombrado a StreamPipeline para evitar conflicto con GstPipeline de GStreamer
class StreamPipeline {
public:
    struct Config {
        std::string name;
        std::string rtsp_url;
        std::string multicast_ip;
        int port;
        std::string interface;  // Ahora se lee del archivo .txt
        int latency_ms = 100;
        int buffer_size = 10;
    };

    explicit StreamPipeline(const Config& cfg);
    ~StreamPipeline();

    // No permitir copia
    StreamPipeline(const StreamPipeline&) = delete;
    StreamPipeline& operator=(const StreamPipeline&) = delete;

    bool start();
    void stop();
    bool isRunning() const { return running_.load(); }
    std::string getStatus() const;
    const Config& getConfig() const { return config_; }

private:
    static GstBusSyncReply busCallback(GstBus* bus, GstMessage* msg, gpointer data);
    void handleBusMessage(GstMessage* msg);
    std::string buildPipelineDescription() const;

    Config config_;
    GstElement* pipeline_ = nullptr;  // Este es el GstPipeline de GStreamer
    GstBus* bus_ = nullptr;
    std::atomic<bool> running_{false};
};
