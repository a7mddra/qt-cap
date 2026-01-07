// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

import QtQuick
import Qt5Compat.GraphicalEffects

/**
 * Freehand drawing canvas with GPU-accelerated white glow.
 * 
 * This is the "Circle to Search" style squiggle selection mode.
 * Uses Qt5Compat.GraphicalEffects Glow for smooth, GPU-rendered glow.
 * 
 * Aesthetic: Clean white stroke with soft gray/white glow. No colors.
 */
Item {
    id: root
    anchors.fill: parent
    focus: true
    
    // Controller reference (set by CaptureWindow loader)
    property var controller
    
    // Drawing state
    property var points: []
    property bool isDrawing: false
    property point currentMouse: Qt.point(0, 0)
    
    // Smoothing factor (matches original m_smoothingFactor = 0.2)
    readonly property real smoothingFactor: 0.2
    readonly property real brushSize: 7
    
    // Cursor circle indicator (visible during drawing)
    Rectangle {
        id: cursorCircle
        width: 56
        height: 56
        radius: 28
        color: Qt.rgba(1, 1, 1, 0.12)
        visible: root.isDrawing
        x: root.currentMouse.x - 28
        y: root.currentMouse.y - 28
        
        // Subtle pulse animation
        SequentialAnimation on scale {
            running: root.isDrawing
            loops: Animation.Infinite
            NumberAnimation { to: 1.05; duration: 600; easing.type: Easing.InOutSine }
            NumberAnimation { to: 1.0; duration: 600; easing.type: Easing.InOutSine }
        }
    }
    
    // The drawing canvas (source for glow effect)
    Canvas {
        id: canvas
        anchors.fill: parent
        visible: false  // Hidden, rendered via glow effect
        
        onPaint: {
            var ctx = getContext("2d")
            ctx.reset()
            
            if (root.points.length < 2) return
            
            // White stroke, rounded caps for smooth look
            ctx.strokeStyle = "white"
            ctx.lineWidth = root.brushSize
            ctx.lineCap = "round"
            ctx.lineJoin = "round"
            
            ctx.beginPath()
            ctx.moveTo(root.points[0].x, root.points[0].y)
            
            // Smooth quadratic curves through control points
            for (var i = 1; i < root.points.length - 1; i++) {
                var xMid = (root.points[i].x + root.points[i + 1].x) / 2
                var yMid = (root.points[i].y + root.points[i + 1].y) / 2
                ctx.quadraticCurveTo(root.points[i].x, root.points[i].y, xMid, yMid)
            }
            
            // Connect to final point
            if (root.points.length > 1) {
                var last = root.points[root.points.length - 1]
                ctx.lineTo(last.x, last.y)
            }
            
            ctx.stroke()
        }
    }
    
    // GPU-accelerated glow effect (Qt5Compat)
    Glow {
        anchors.fill: canvas
        source: canvas
        radius: 12
        samples: 25
        color: Qt.rgba(1, 1, 1, 0.5)  // Soft white glow
        spread: 0.2
        cached: false
    }
    
    // Render the actual stroke on top of glow
    ShaderEffectSource {
        anchors.fill: canvas
        sourceItem: canvas
        live: true
    }
    
    // Cross cursor when not drawing
    Item {
        id: crosshair
        visible: !root.isDrawing && root.points.length === 0
        x: mouseArea.mouseX
        y: mouseArea.mouseY
        
        Rectangle {
            width: 1
            height: 20
            color: Qt.rgba(1, 1, 1, 0.7)
            anchors.centerIn: parent
        }
        Rectangle {
            width: 20
            height: 1
            color: Qt.rgba(1, 1, 1, 0.7)
            anchors.centerIn: parent
        }
    }
    
    // Mouse interaction
    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.CrossCursor
        
        onPressed: function(mouse) {
            // Clear any previous drawing
            root.points = []
            root.isDrawing = true
            root.currentMouse = Qt.point(mouse.x, mouse.y)
            root.points.push(root.currentMouse)
            canvas.requestPaint()
        }
        
        onPositionChanged: function(mouse) {
            root.currentMouse = Qt.point(mouse.x, mouse.y)
            
            if (root.isDrawing && root.points.length > 0) {
                // Apply smoothing (matches original algorithm)
                var prev = root.points[root.points.length - 1]
                var smoothed = Qt.point(
                    prev.x * (1 - root.smoothingFactor) + mouse.x * root.smoothingFactor,
                    prev.y * (1 - root.smoothingFactor) + mouse.y * root.smoothingFactor
                )
                root.points.push(smoothed)
                canvas.requestPaint()
            }
        }
        
        onReleased: function(mouse) {
            if (root.isDrawing) {
                root.isDrawing = false
                
                // Convert points array to QVariantList for C++
                var pointsList = []
                for (var i = 0; i < root.points.length; i++) {
                    pointsList.push(root.points[i])
                }
                
                root.controller.finishSquiggleCapture(pointsList)
            }
        }
    }
    
    // Keyboard handling
    Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Escape || event.key === Qt.Key_Q) {
            root.controller.cancel()
            event.accepted = true
        }
    }
}
