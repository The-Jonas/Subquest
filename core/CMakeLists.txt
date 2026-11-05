cmake_minimum_required(VERSION 3.24)
project(subquest_core VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

option(SUBQUEST_BUILD_TESTS "Compila os testes do núcleo" ON)

# --- Partes portáveis (compilam em qualquer SO; é o que os testes usam) ---
add_library(subquest_portable STATIC
    src/vision/change_detector.cpp
    src/vision/image_ops.cpp
)
target_include_directories(subquest_portable PUBLIC src)

# --- DLL do núcleo (Windows) ---
add_library(subquest_core SHARED
    src/core_api.cpp
    src/pipeline/pipeline.cpp
    src/pipeline/preview_stream.cpp
    src/capture/wgc_capture.cpp
    src/capture/window_list.cpp
    src/ocr/win_ocr_engine.cpp
    src/overlay/overlay_window.cpp
)
target_include_directories(subquest_core
    PUBLIC  include
    PRIVATE src
)
target_compile_definitions(subquest_core PRIVATE SUBQUEST_CORE_BUILD)
target_link_libraries(subquest_core PRIVATE subquest_portable)

if(WIN32)
    target_compile_definitions(subquest_core PRIVATE UNICODE _UNICODE WIN32_LEAN_AND_MEAN NOMINMAX)
    target_link_libraries(subquest_core PRIVATE dwmapi d3d11 dxgi windowsapp)
endif()

if(MSVC)
    # /utf-8: fontes e literais em UTF-8 (comentários em português)
    # /permissive-: exigido pelo C++/WinRT
    target_compile_options(subquest_portable PRIVATE /W4 /utf-8)
    target_compile_options(subquest_core PRIVATE /W4 /utf-8 /permissive-)
else()
    target_compile_options(subquest_portable PRIVATE -Wall -Wextra)
    target_compile_options(subquest_core PRIVATE -Wall -Wextra -fvisibility=hidden)
endif()

# Copia a DLL para onde o app Tauri procura em modo dev
set(SUBQUEST_APP_BIN "${CMAKE_CURRENT_SOURCE_DIR}/../app/src-tauri/bin")
add_custom_command(TARGET subquest_core POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E make_directory "${SUBQUEST_APP_BIN}"
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_FILE:subquest_core> "${SUBQUEST_APP_BIN}"
)

if(SUBQUEST_BUILD_TESTS)
    enable_testing()
    add_executable(change_detector_test tests/change_detector_test.cpp)
    target_link_libraries(change_detector_test PRIVATE subquest_portable)
    add_test(NAME change_detector COMMAND change_detector_test)

    add_executable(image_ops_test tests/image_ops_test.cpp)
    target_link_libraries(image_ops_test PRIVATE subquest_portable)
    add_test(NAME image_ops COMMAND image_ops_test)
endif()
