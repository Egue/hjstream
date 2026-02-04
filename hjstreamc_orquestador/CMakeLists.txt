cmake_minimum_required(VERSION 3.10)
project(multicast-streamer VERSION 1.0.0)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -O3 -Wall -Wextra")
set(CMAKE_CXX_FLAGS_DEBUG "-g -O0")

# Buscar dependencias
find_package(PkgConfig REQUIRED)
pkg_check_modules(GSTREAMER REQUIRED 
    gstreamer-1.0>=1.14
    gstreamer-rtsp-1.0>=1.14
)

# Incluir directorios
include_directories(
    ${CMAKE_SOURCE_DIR}/src
    ${GSTREAMER_INCLUDE_DIRS}
)

link_directories(${GSTREAMER_LIBRARY_DIRS})

# Archivos fuente
set(SOURCES
    src/main.cpp
    src/ChannelManager.cpp
    src/GstPipeline.cpp
)

# Ejecutable principal
add_executable(multicast-streamer ${SOURCES})

target_link_libraries(multicast-streamer
    ${GSTREAMER_LIBRARIES}
    pthread
)

# Instalación
install(TARGETS multicast-streamer 
    RUNTIME DESTINATION bin
)

install(FILES config/canales.txt.example
    DESTINATION etc/multicast-streamer
)

install(FILES systemd/multicast-streamer.service
    DESTINATION lib/systemd/system
)

install(FILES scripts/setup.sh
    DESTINATION share/multicast-streamer
    PERMISSIONS OWNER_READ OWNER_WRITE OWNER_EXECUTE GROUP_READ GROUP_EXECUTE WORLD_READ WORLD_EXECUTE
)

# Generar paquete
set(CPACK_GENERATOR "TGZ")
set(CPACK_PACKAGE_NAME "multicast-streamer")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Broadcast-grade multicast streaming server")
include(CPack)
