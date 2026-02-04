#pragma once
#include <gst/gst.h>
#include <string>
#include <memory>
#include <atomic>

class GstPipeline {
public:
    struct Config {
        std::string name;
        std::string rtsp_url;
        std::string multicast_ip;
        int port;
        std::string interface = "eno1";
        int latency_ms = 100;
        int buffer_size = 10;
    };

    explicit GstPipeline(const Config& cfg);
    ~GstPipeline();

    // No permitir copia
    GstPipeline(const GstPipeline&) = delete;
    GstPipeline& operator=(const GstPipeline&) = delete;

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
    GstElement* pipeline_ = nullptr;
    GstBus* bus_ = nullptr;
    std::atomic<bool> running_{false};
};
