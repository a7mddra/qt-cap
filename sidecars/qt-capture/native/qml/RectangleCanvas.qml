// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

import QtQuick

/**
 * Rectangle selection canvas.
 * 
 * Provides a click-and-drag rectangle selection with:
 * - Dimmed overlay outside selection
 * - White border with corner handles
 * - Dimension label below selection
 * - Crosshair cursor when idle
 * 
 * Aesthetic: Clean monochrome, matches the squiggle canvas style.
 */
Item {
    id: root
    anchors.fill: parent
    focus: true
    
    // Controller reference (set by CaptureWindow loader)
    property var controller
    
    // Selection state
    property point startPoint: Qt.point(0, 0)
    property point endPoint: Qt.point(0, 0)
    property bool isDrawing: false
    property bool hasSelection: false
    
    // Computed normalized rectangle
    readonly property real selX: Math.min(startPoint.x, endPoint.x)
    readonly property real selY: Math.min(startPoint.y, endPoint.y)
    readonly property real selW: Math.abs(endPoint.x - startPoint.x)
    readonly property real selH: Math.abs(endPoint.y - startPoint.y)
    
    // Dim overlay with cutout for selection
    Canvas {
        id: dimCanvas
        anchors.fill: parent
        
        onPaint: {
            var ctx = getContext("2d")
            ctx.reset()
            
            // Full dim overlay
            ctx.fillStyle = Qt.rgba(0, 0, 0, 0.35)
            ctx.fillRect(0, 0, width, height)
            
            // Cut out the selection area
            if (root.isDrawing || root.hasSelection) {
                ctx.globalCompositeOperation = "destination-out"
                ctx.fillStyle = "white"
                ctx.fillRect(root.selX, root.selY, root.selW, root.selH)
            }
        }
    }
    
    // Repaint dim canvas when selection changes
    Connections {
        target: root
        function onStartPointChanged() { dimCanvas.requestPaint() }
        function onEndPointChanged() { dimCanvas.requestPaint() }
        function onIsDrawingChanged() { dimCanvas.requestPaint() }
    }
    
    // Selection rectangle border
    Rectangle {
        id: selectionBorder
        visible: root.isDrawing || root.hasSelection
        x: root.selX
        y: root.selY
        width: root.selW
        height: root.selH
        color: "transparent"
        border.color: "white"
        border.width: 2
        
        // Corner handles
        Repeater {
            model: [
                { px: 0, py: 0 },
                { px: 1, py: 0 },
                { px: 0, py: 1 },
                { px: 1, py: 1 }
            ]
            
            Rectangle {
                width: 8
                height: 8
                radius: 4
                color: "white"
                x: modelData.px * selectionBorder.width - 4
                y: modelData.py * selectionBorder.height - 4
            }
        }
    }
    
    // Dimension label
    Rectangle {
        id: dimLabel
        visible: (root.isDrawing || root.hasSelection) && root.selW > 10 && root.selH > 10
        
        x: root.selX + root.selW / 2 - width / 2
        y: root.selY + root.selH + 12
        
        width: dimText.width + 16
        height: dimText.height + 8
        radius: 4
        color: Qt.rgba(0, 0, 0, 0.7)
        
        Text {
            id: dimText
            anchors.centerIn: parent
            text: Math.round(root.selW) + " × " + Math.round(root.selH)
            color: "white"
            font.pixelSize: 11
            font.bold: true
        }
    }
    
    // Mouse interaction
    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.CrossCursor
        
        onPressed: function(mouse) {
            // Start new selection
            root.startPoint = Qt.point(mouse.x, mouse.y)
            root.endPoint = root.startPoint
            root.isDrawing = true
            root.hasSelection = false
        }
        
        onPositionChanged: function(mouse) {
            if (root.isDrawing) {
                root.endPoint = Qt.point(mouse.x, mouse.y)
            }
        }
        
        onReleased: function(mouse) {
            if (root.isDrawing) {
                root.endPoint = Qt.point(mouse.x, mouse.y)
                root.isDrawing = false
                root.hasSelection = true
                
                // Send to controller
                root.controller.finishRectCapture(root.startPoint, root.endPoint)
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
