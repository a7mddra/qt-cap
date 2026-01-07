cmake_minimum_required(VERSION 3.16)

if(APPLE)
    project(CAPTURE LANGUAGES CXX OBJCXX)
else()
    project(CAPTURE LANGUAGES CXX)
endif()

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

# Qt Quick requires these policies for qt_add_qml_module
cmake_policy(SET CMP0071 NEW)
if(POLICY CMP0169)
    cmake_policy(SET CMP0169 OLD)
endif()

# Core Qt packages (keep Widgets for now during incremental migration)
find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets)
# Qt Quick packages for QML UI (Qt5Compat provides GraphicalEffects)
find_package(Qt6 REQUIRED COMPONENTS Quick)
find_package(Qt6 REQUIRED COMPONENTS Core5Compat)
if(UNIX AND NOT APPLE)
    find_package(Qt6 REQUIRED COMPONENTS DBus)
endif()

configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/config.h.in"
    "${CMAKE_CURRENT_BINARY_DIR}/generated/config.h"
    @ONLY
)
include_directories("${CMAKE_CURRENT_BINARY_DIR}/generated")

# C++ sources (backend - UNTOUCHED)
set(SOURCES 
    src/main.cpp 
    src/shutter/ScreenGrabber.h
    src/surface/CaptureMode.h
    # Legacy widget canvases (kept during migration, will be removed later)
    src/surface/SquiggleCanvas.cpp 
    src/surface/SquiggleCanvas.h 
    src/surface/RectangleCanvas.cpp
    src/surface/RectangleCanvas.h
    src/surface/OverlayWindow.h
    # New QML bridge
    src/qml/CaptureController.cpp
    src/qml/CaptureController.h
)

if(WIN32)
    list(APPEND SOURCES src/platforms/Shutter_Win.cpp src/surface/OverlayWindow.cpp)
    set(PLATFORM_LIBS dwmapi gdiplus user32 gdi32 Shcore)
elseif(APPLE)
    list(APPEND SOURCES src/platforms/Shutter_Mac.mm src/surface/OverlayWindow.mm)
    
    find_library(FOUNDATION_LIB Foundation)
    find_library(COREGRAPHICS_LIB CoreGraphics)
    find_library(COCOA_LIB Cocoa)
    find_library(APPKIT_LIB AppKit)
    
    if(NOT FOUNDATION_LIB OR NOT COREGRAPHICS_LIB OR NOT COCOA_LIB OR NOT APPKIT_LIB)
        message(FATAL_ERROR "Required macOS frameworks not found")
    endif()
    set(PLATFORM_LIBS ${FOUNDATION_LIB} ${COREGRAPHICS_LIB} ${COCOA_LIB} ${APPKIT_LIB})
elseif(UNIX AND NOT APPLE)
    list(APPEND SOURCES src/platforms/Shutter_Linux.cpp src/surface/OverlayWindow.cpp)
    set(PLATFORM_LIBS Qt6::DBus)
endif()

# Create executable with QML module
qt_add_executable(capture WIN32 MACOSX_BUNDLE ${SOURCES})

# Add QML module for the capture UI
qt_add_qml_module(capture
    URI CaptureQml
    VERSION 1.0
    QML_FILES
        qml/CaptureWindow.qml
        qml/SquiggleCanvas.qml
        qml/RectangleCanvas.qml
)

target_include_directories(capture PRIVATE 
    src/shutter
    src/surface
    src/platforms
    src/interface
    src/qml
)

target_link_libraries(capture PRIVATE 
    Qt6::Core Qt6::Gui Qt6::Widgets 
    Qt6::Quick Qt6::Core5Compat
    ${PLATFORM_LIBS}
)

if(UNIX AND NOT APPLE)
    set_target_properties(capture PROPERTIES
        OUTPUT_NAME "capture-bin"
        INSTALL_RPATH "$ORIGIN/../libs"
        BUILD_WITH_INSTALL_RPATH TRUE
    )
endif()

if(APPLE)
    set_target_properties(capture PROPERTIES
        MACOSX_BUNDLE TRUE
        MACOSX_BUNDLE_GUI_IDENTIFIER "com.capture.app"
        MACOSX_BUNDLE_BUNDLE_NAME "Capture"
        MACOSX_BUNDLE_INFO_STRING "Capture Screen Capture"
        MACOSX_BUNDLE_INFO_PLIST "${CMAKE_CURRENT_SOURCE_DIR}/Info.plist.in"
    )
endif()
