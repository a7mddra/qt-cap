// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

import QtQuick
import QtQuick.Window

/**
 * Main capture overlay window.
 * 
 * This window displays fullscreen over a single monitor, showing the frozen
 * screenshot as background with either squiggle or rectangle selection mode.
 * 
 * Critical window flags ensure instant appearance without OS animations:
 * - Qt.FramelessWindowHint: No title bar or borders
 * - Qt.WindowStaysOnTopHint: Always on top
 * - Qt.BypassWindowManagerHint: Bypass WM completely (critical for GNOME top bar)
 * - Qt.Tool: Skips taskbar/dock, treated as utility window
 */
Window {
    id: root
    
    // Window flags for true fullscreen overlay (bypasses GNOME panel)
    // BypassWindowManagerHint is critical - it tells Qt to ignore the WM entirely
    flags: Qt.FramelessWindowHint 
           | Qt.WindowStaysOnTopHint 
           | Qt.BypassWindowManagerHint
           | Qt.Tool
    
    // Explicit geometry to cover entire screen (don't rely on FullScreen visibility)
    x: Screen.virtualX
    y: Screen.virtualY
    width: Screen.width
    height: Screen.height
    
    visibility: Window.Windowed  // Use Windowed + explicit geometry, not FullScreen
    color: "transparent"
    
    // Properties set from C++ before showing
    required property var controller
    
    // Frozen screenshot background
    Image {
        id: background
        anchors.fill: parent
        source: root.controller.backgroundSource
        fillMode: Image.PreserveAspectCrop
        cache: false
    }
    
    // Animated dim overlay (top-to-bottom gradient, matches original)
    Rectangle {
        id: dimOverlay
        anchors.fill: parent
        opacity: 0
        visible: root.controller.captureMode !== "rectangle"
        
        gradient: Gradient {
            GradientStop { position: 0.0; color: Qt.rgba(0, 0, 0, 0.5) }
            GradientStop { position: 1.0; color: "transparent" }
        }
        
        // Fade in animation (200ms, matches original m_animation duration)
        NumberAnimation on opacity {
            from: 0; to: 1
            duration: 200
            running: true
            easing.type: Easing.OutQuad
        }
    }
    
    // Canvas loader - switches between draw modes
    Loader {
        id: canvasLoader
        anchors.fill: parent
        focus: true
        
        source: root.controller.captureMode === "rectangle" 
            ? "RectangleCanvas.qml" 
            : "SquiggleCanvas.qml"
        
        onLoaded: {
            // Pass controller to loaded canvas
            if (item) {
                item.controller = root.controller
                item.forceActiveFocus()
            }
        }
    }
    
    // Global keyboard shortcuts
    Shortcut {
        sequence: "Escape"
        onActivated: root.controller.cancel()
    }
    
    Shortcut {
        sequence: "Q"
        onActivated: root.controller.cancel()
    }
    
    // Ensure window is focused when shown
    Component.onCompleted: {
        root.requestActivate()
    }
}
