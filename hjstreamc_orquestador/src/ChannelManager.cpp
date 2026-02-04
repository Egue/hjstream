#include "ChannelManager.h"
#include <fstream>
#include <sstream>
#include <iostream>
#include <thread>
#include <algorithm>

ChannelManager::ChannelManager(const std::string& config_file, int restart_delay_sec)
    : config_file_(config_file), restart_delay_sec_(restart_delay_sec) {}

ChannelManager::~ChannelManager() {
    stopAll();
}

void ChannelManager::parseConfigLine(const std::string& line) {
    std::istringstream iss(line);
    std::string name, path, ip, port_str, interface;
    
    if (!std::getline(iss, name, ',') ||
        !std::getline(iss, path, ',') ||
        !std::getline(iss, ip, ',') ||
        !std::getline(iss, port_str, ',') ||
        !std::getline(iss, interface)) {
        std::cerr << "Formato inválido en línea: " << line << std::endl;
        std::cerr << "Formato esperado: nombre,/ruta_rtsp,ip_multicast,puerto,interfaz" << std::endl;
        return;
    }

    // Limpiar whitespace
    auto trim = [](std::string& s) {
        s.erase(s.begin(), std::find_if(s.begin(), s.end(), [](unsigned char ch) {
            return !std::isspace(ch);
        }));
        s.erase(std::find_if(s.rbegin(), s.rend(), [](unsigned char ch) {
            return !std::isspace(ch);
        }).base(), s.end());
    };
    
    trim(name);
    trim(path);
    trim(ip);
    trim(port_str);
    trim(interface);

    StreamPipeline::Config cfg;
    cfg.name = name;
    cfg.rtsp_url = "rtsp://127.0.0.1:8554" + path;
    cfg.multicast_ip = ip;
    cfg.interface = interface;
    
    try {
        cfg.port = std::stoi(port_str);
    } catch (const std::exception& e) {
        std::cerr << "Puerto inválido '" << port_str << "' para canal " << name << std::endl;
        return;
    }

    pipelines_.push_back(std::make_unique<StreamPipeline>(cfg));
}

bool ChannelManager::loadChannels() {
    std::ifstream file(config_file_);
    if (!file.is_open()) {
        std::cerr << "✗ Error abriendo archivo de configuración: " << config_file_ << std::endl;
        return false;
    }

    std::string line;
    int line_num = 0;
    
    while (std::getline(file, line)) {
        line_num++;
        
        // Ignorar líneas vacías y comentarios
        if (line.empty() || line[0] == '#') continue;
        
        parseConfigLine(line);
    }

    std::cout << "✓ Cargados " << pipelines_.size() << " canales desde " 
              << config_file_ << std::endl;
    
    return !pipelines_.empty();
}

void ChannelManager::startAll() {
    if (pipelines_.empty()) {
        std::cerr << "✗ No hay canales para iniciar" << std::endl;
        return;
    }

    std::cout << "\n=== Iniciando " << pipelines_.size() << " pipelines ===" << std::endl;
    
    int started = 0;
    for (auto& pipeline : pipelines_) {
        if (pipeline->start()) {
            started++;
        }
        // Delay pequeño entre inicios para evitar picos de CPU/red
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
    }

    std::cout << "\n✓ " << started << "/" << pipelines_.size() 
              << " pipelines iniciados correctamente" << std::endl;

    // Iniciar thread de monitoreo
    running_.store(true);
    monitor_thread_ = std::thread(&ChannelManager::monitorLoop, this);
}

void ChannelManager::stopAll() {
    std::cout << "\n=== Deteniendo pipelines ===" << std::endl;
    
    running_.store(false);
    
    if (monitor_thread_.joinable()) {
        monitor_thread_.join();
    }

    for (auto& pipeline : pipelines_) {
        pipeline->stop();
    }
    
    std::cout << "✓ Todos los pipelines detenidos" << std::endl;
}

void ChannelManager::monitorLoop() {
    std::cout << "✓ Monitor de canales iniciado (intervalo: " 
              << restart_delay_sec_ << "s)" << std::endl;
    
    while (running_.load()) {
        std::this_thread::sleep_for(std::chrono::seconds(restart_delay_sec_));
        
        if (running_.load()) {
            restartFailedPipelines();
        }
    }
}

void ChannelManager::restartFailedPipelines() {
    int failed_count = 0;
    
    for (auto& pipeline : pipelines_) {
        if (!pipeline->isRunning()) {
            failed_count++;
            const auto& cfg = pipeline->getConfig();
            
            std::cout << "\n⚠ Detectado pipeline fallido: " << cfg.name << std::endl;
            std::cout << "  Reiniciando en 2 segundos..." << std::endl;
            
            pipeline->stop();
            std::this_thread::sleep_for(std::chrono::seconds(2));
            
            if (pipeline->start()) {
                std::cout << "  ✓ Pipeline reiniciado exitosamente" << std::endl;
            } else {
                std::cout << "  ✗ Error reiniciando pipeline" << std::endl;
            }
        }
    }
    
    // Solo reportar si hubo fallos
    if (failed_count > 0) {
        std::cout << "\nSe reiniciaron " << failed_count << " pipeline(s)" << std::endl;
    }
}

void ChannelManager::printStatus() const {
    std::cout << "\n=== Estado de Canales ===" << std::endl;
    
    int running = 0;
    for (const auto& pipeline : pipelines_) {
        const auto& cfg = pipeline->getConfig();
        std::string status = pipeline->getStatus();
        
        std::cout << "  " << cfg.name << ": " << status 
                  << " (" << cfg.multicast_ip << ":" << cfg.port << ")" << std::endl;
        
        if (status == "PLAYING") running++;
    }
    
    std::cout << "\nTotal: " << running << "/" << pipelines_.size() 
              << " pipelines activos" << std::endl;
}
