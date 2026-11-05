cmake_minimum_required(VERSION 3.24)
project(subquest_core VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

option(SUBQUEST_BUILD_TESTS "Build the core tests" ON)

# --- Portable parts (build on any OS; this is what the tests use) ---
add_library(subquest_portable STATIC
    src/vision/change_detector.cpp
    src/vision/image_ops.cpp
    src/vision/palette.cpp
)
target_include_directories(subquest_portable PUBLIC src)

# --- Core DLL (Windows) ---
add_library(subquest_core SHARED
    src/core_api.cpp
    src/pipeline/pipeline.cpp
    src/pipeline/preview_stream.cpp
    src/capture/wgc_capture.cpp
    src/capture/window_list.cpp
    src/ocr/win_ocr_engine.cpp
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
    # /utf-8: sources and literals are UTF-8
    # /permissive-: required by C++/WinRT
    target_compile_options(subquest_portable PRIVATE /W4 /utf-8)
    target_compile_options(subquest_core PRIVATE /W4 /utf-8 /permissive-)
else()
    target_compile_options(subquest_portable PRIVATE -Wall -Wextra)
    target_compile_options(subquest_core PRIVATE -Wall -Wextra -fvisibility=hidden)
endif()

# Copies the DLL to where the Tauri app looks for it in dev mode
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

    add_executable(palette_test tests/palette_test.cpp)
    target_link_libraries(palette_test PRIVATE subquest_portable)
    add_test(NAME palette COMMAND palette_test)
endif()
