#include "GstPipeline.h"
#include <iostream>
#include <sstream>

StreamPipeline::StreamPipeline(const Config& cfg) : config_(cfg) {}

StreamPipeline::~StreamPipeline() {
    stop();
}

std::string StreamPipeline::buildPipelineDescription() const {
    std::ostringstream pipeline_desc;
    
    // Pipeline que maneja video obligatorio + audio opcional
    // Si el audio no existe, solo pasa video
    pipeline_desc << "rtspsrc location=" << config_.rtsp_url
                  << " latency=" << config_.latency_ms
                  << " protocols=tcp name=src ";
    
    // Video H.264 (obligatorio)
    pipeline_desc << "src. ! application/x-rtp,media=video ! rtph264depay ! h264parse ! "
                  << "queue max-size-buffers=" << config_.buffer_size << " ! mux. ";
    
    // Audio AAC (opcional - GStreamer lo ignora si no existe)
    pipeline_desc << "src. ! application/x-rtp,media=audio ! rtpmp4adepay ! aacparse ! "
                  << "queue max-size-buffers=" << config_.buffer_size << " ! mux. ";
    
    // Mux y salida
    pipeline_desc << "mpegtsmux name=mux alignment=7 ! "
                  << "udpsink host=" << config_.multicast_ip
                  << " port=" << config_.port
                  << " auto-multicast=false multicast-iface=" << config_.interface
                  << " sync=false async=false";

    return pipeline_desc.str();
}

bool StreamPipeline::start() {
    if (running_.load()) {
        std::cerr << "[" << config_.name << "] Pipeline ya está corriendo" << std::endl;
        return true;
    }

    std::string pipeline_desc = buildPipelineDescription();
    
    GError* error = nullptr;
    pipeline_ = gst_parse_launch(pipeline_desc.c_str(), &error);
    
    if (error) {
        std::cerr << "[" << config_.name << "] Error creando pipeline: " 
                  << error->message << std::endl;
        g_error_free(error);
        return false;
    }

    // Configurar bus para monitoreo
    bus_ = gst_pipeline_get_bus(GST_PIPELINE(pipeline_));
    gst_bus_set_sync_handler(bus_, busCallback, this, nullptr);

    // Iniciar pipeline
    GstStateChangeReturn ret = gst_element_set_state(pipeline_, GST_STATE_PLAYING);
    if (ret == GST_STATE_CHANGE_FAILURE) {
        std::cerr << "[" << config_.name << "] Error iniciando pipeline" << std::endl;
        gst_object_unref(pipeline_);
        gst_object_unref(bus_);
        pipeline_ = nullptr;
        bus_ = nullptr;
        return false;
    }

    running_.store(true);
    std::cout << "[" << config_.name << "] ✓ Pipeline iniciado → " 
              << config_.multicast_ip << ":" << config_.port << std::endl;
    return true;
}

void StreamPipeline::stop() {
    if (!pipeline_) return;

    running_.store(false);
    
    gst_element_set_state(pipeline_, GST_STATE_NULL);
    gst_object_unref(pipeline_);
    pipeline_ = nullptr;
    
    if (bus_) {
        gst_object_unref(bus_);
        bus_ = nullptr;
    }

    std::cout << "[" << config_.name << "] Pipeline detenido" << std::endl;
}

GstBusSyncReply StreamPipeline::busCallback(GstBus* bus, GstMessage* msg, gpointer data) {
    auto* pipeline = static_cast<StreamPipeline*>(data);
    pipeline->handleBusMessage(msg);
    return GST_BUS_PASS;
}

void StreamPipeline::handleBusMessage(GstMessage* msg) {
    switch (GST_MESSAGE_TYPE(msg)) {
        case GST_MESSAGE_ERROR: {
            GError* err;
            gchar* debug;
            gst_message_parse_error(msg, &err, &debug);
            
            // Filtrar errores de audio ausente (normal si stream no tiene audio)
            std::string error_msg = err->message ? err->message : "";
            std::string debug_info = debug ? debug : "";
            
            bool is_audio_missing = (
                error_msg.find("not-linked") != std::string::npos ||
                error_msg.find("not linked") != std::string::npos ||
                error_msg.find("Could not link") != std::string::npos ||
                debug_info.find("rtpmp4adepay") != std::string::npos ||
                debug_info.find("aacparse") != std::string::npos
            );
            
            if (is_audio_missing) {
                // Solo video, no es un error
                std::cout << "[" << config_.name << "] ℹ Stream solo video (sin audio)" << std::endl;
            } else {
                // Error crítico
                std::cerr << "[" << config_.name << "] ✗ ERROR: " << error_msg << std::endl;
                if (!debug_info.empty()) {
                    std::cerr << "  Debug: " << debug_info << std::endl;
                }
                running_.store(false);
            }
            
            g_error_free(err);
            g_free(debug);
            break;
        }
        case GST_MESSAGE_WARNING: {
            GError* warn;
            gchar* debug;
            gst_message_parse_warning(msg, &warn, &debug);
            
            // Silenciar warnings de not-linked (audio opcional)
            std::string warn_msg = warn->message ? warn->message : "";
            if (warn_msg.find("not-linked") == std::string::npos) {
                std::cerr << "[" << config_.name << "] ⚠ WARNING: " << warn_msg << std::endl;
            }
            
            g_error_free(warn);
            g_free(debug);
            break;
        }
        case GST_MESSAGE_EOS:
            std::cout << "[" << config_.name << "] End of stream" << std::endl;
            running_.store(false);
            break;
        case GST_MESSAGE_STATE_CHANGED: {
            if (GST_MESSAGE_SRC(msg) == GST_OBJECT(pipeline_)) {
                GstState old_state, new_state;
                gst_message_parse_state_changed(msg, &old_state, &new_state, nullptr);
                if (new_state == GST_STATE_PLAYING && old_state != GST_STATE_PLAYING) {
                    std::cout << "[" << config_.name << "] → PLAYING" << std::endl;
                }
            }
            break;
        }
        default:
            break;
    }
}

std::string StreamPipeline::getStatus() const {
    if (!pipeline_) return "STOPPED";
    
    GstState state;
    gst_element_get_state(pipeline_, &state, nullptr, 0);
    
    switch (state) {
        case GST_STATE_PLAYING: return "PLAYING";
        case GST_STATE_PAUSED: return "PAUSED";
        case GST_STATE_READY: return "READY";
        case GST_STATE_NULL: return "NULL";
        default: return "UNKNOWN";
    }
}