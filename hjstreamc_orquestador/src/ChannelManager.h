#pragma once
#include "GstPipeline.h"
#include <vector>
#include <memory>
#include <thread>
#include <atomic>
#include <chrono>

class ChannelManager {
public:
    ChannelManager(const std::string& config_file, int restart_delay_sec = 5);
    ~ChannelManager();

    // No permitir copia
    ChannelManager(const ChannelManager&) = delete;
    ChannelManager& operator=(const ChannelManager&) = delete;

    bool loadChannels();
    void startAll();
    void stopAll();
    void monitorLoop();
    void printStatus() const;

private:
    void restartFailedPipelines();
    void parseConfigLine(const std::string& line);

    std::string config_file_;
    int restart_delay_sec_;
    std::vector<std::unique_ptr<StreamPipeline>> pipelines_;
    std::atomic<bool> running_{false};
    std::thread monitor_thread_;
};
