#include "ChannelManager.h"
#include <gst/gst.h>
#include <signal.h>
#include <iostream>
#include <cstring>

static ChannelManager* g_manager = nullptr;

void signalHandler(int signum) {
    std::cout << "\n\n=== Señal recibida (" << signum << "), cerrando gracefully ===" << std::endl;
    if (g_manager) {
        g_manager->stopAll();
    }
    exit(signum);
}

void printUsage(const char* program_name) {
    std::cout << "Uso: " << program_name << " [OPCIONES]\n\n"
              << "Opciones:\n"
              << "  -c, --config FILE    Archivo de configuración (default: /etc/multicast-streamer/canales.txt)\n"
              << "  -r, --restart SEC    Intervalo de reintentos en segundos (default: 5)\n"
              << "  -h, --help           Mostrar esta ayuda\n"
              << "  -v, --version        Mostrar versión\n"
              << "\nFormato del archivo de configuración:\n"
              << "  # Comentario\n"
              << "  nombre,/ruta_rtsp,ip_multicast,puerto\n"
              << "\nEjemplo:\n"
              << "  Canal1,/stream1,239.1.1.1,5000\n"
              << "  Canal2,/stream2,239.1.1.2,5000\n"
              << std::endl;
}

void printVersion() {
    std::cout << "Multicast Streamer v1.0.0\n"
              << "Broadcast-grade streaming server\n"
              << "GStreamer version: " << gst_version_string() << std::endl;
}

int main(int argc, char* argv[]) {
    std::string config_file = "/etc/multicast-streamer/canales.txt";
    int restart_delay = 5;

    // Parsear argumentos
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            printUsage(argv[0]);
            return 0;
        } else if (strcmp(argv[i], "-v") == 0 || strcmp(argv[i], "--version") == 0) {
            printVersion();
            return 0;
        } else if (strcmp(argv[i], "-c") == 0 || strcmp(argv[i], "--config") == 0) {
            if (i + 1 < argc) {
                config_file = argv[++i];
            } else {
                std::cerr << "Error: -c requiere un argumento" << std::endl;
                return 1;
            }
        } else if (strcmp(argv[i], "-r") == 0 || strcmp(argv[i], "--restart") == 0) {
            if (i + 1 < argc) {
                restart_delay = std::atoi(argv[++i]);
                if (restart_delay <= 0) {
                    std::cerr << "Error: intervalo de reintentos debe ser > 0" << std::endl;
                    return 1;
                }
            } else {
                std::cerr << "Error: -r requiere un argumento" << std::endl;
                return 1;
            }
        } else {
            std::cerr << "Opción desconocida: " << argv[i] << std::endl;
            printUsage(argv[0]);
            return 1;
        }
    }

    // Banner
    std::cout << R"(
╔═══════════════════════════════════════════════════════════╗
║         MULTICAST STREAMER - Broadcast Edition            ║
║                    Version 1.0.0                          ║
╚═══════════════════════════════════════════════════════════╝
)" << std::endl;

    // Inicializar GStreamer
    gst_init(&argc, &argv);
    std::cout << "✓ GStreamer inicializado: " << gst_version_string() << std::endl;

    // Configurar señales
    signal(SIGINT, signalHandler);
    signal(SIGTERM, signalHandler);

    // Crear manager
    std::cout << "✓ Usando archivo de configuración: " << config_file << std::endl;
    ChannelManager manager(config_file, restart_delay);
    g_manager = &manager;

    if (!manager.loadChannels()) {
        std::cerr << "\n✗ Error: no se pudieron cargar los canales" << std::endl;
        std::cerr << "Verifica que el archivo " << config_file << " existe y tiene el formato correcto" << std::endl;
        return 1;
    }

    manager.startAll();

    std::cout << "\n╔═══════════════════════════════════════════════════════════╗" << std::endl;
    std::cout << "║  Streamer ACTIVO - Presiona Ctrl+C para detener          ║" << std::endl;
    std::cout << "╚═══════════════════════════════════════════════════════════╝\n" << std::endl;

    // Mantener vivo con GLib main loop
    GMainLoop* loop = g_main_loop_new(nullptr, FALSE);
    
    // Thread para mostrar estado periódicamente (cada 60 segundos)
    std::thread status_thread([&manager, &loop]() {
        while (g_main_loop_is_running(loop)) {
            std::this_thread::sleep_for(std::chrono::seconds(60));
            if (g_main_loop_is_running(loop)) {
                manager.printStatus();
            }
        }
    });

    g_main_loop_run(loop);

    // Cleanup
    g_main_loop_unref(loop);
    if (status_thread.joinable()) {
        status_thread.join();
    }

    return 0;
}
