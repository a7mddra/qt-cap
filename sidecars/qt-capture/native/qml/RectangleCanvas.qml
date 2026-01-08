// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

import QtQuick
import Qt5Compat.GraphicalEffects

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
        opacity: 0

        // Fade in animation (matches CaptureWindow original)
        NumberAnimation on opacity {
            from: 0; to: 1
            duration: 200
            running: true
            easing.type: Easing.OutQuad
        }
        
        onPaint: {
            var ctx = getContext("2d")
            ctx.reset()
            
            // Gradient dim overlay (matches original look)
            // Top is darker (gradient + base dim), bottom is lighter (base dim only)
            var grad = ctx.createLinearGradient(0, 0, 0, height);
            grad.addColorStop(0.0, Qt.rgba(0, 0, 0, 0.65));
            grad.addColorStop(1.0, Qt.rgba(0, 0, 0, 0.35));
            
            ctx.fillStyle = grad
            ctx.fillRect(0, 0, width, height)
            
            // Cut out the selection area
            if (root.isDrawing || root.hasSelection) {
                ctx.globalCompositeOperation = "destination-out"
                ctx.fillStyle = "white"
                
                // Use shared path logic to match border
                root.drawSelectionPath(ctx, root.selX, root.selY, root.selW, root.selH)
                
                ctx.fill()
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
    
    // Shared path drawing logic for consistent rounded/sharp corners
    function drawSelectionPath(ctx, x, y, w, h) {
        // Dynamic radius: Max 24, but shrink if rect is too small to avoid artifacts
        var r = Math.min(24, Math.min(w, h) / 2)
        
        // Determine which corner matches the current mouse (endPoint)
        // normalized relative to the rect
        var tl = r, tr = r, br = r, bl = r
        
        // Logic: The corner corresponding to endPoint gets 0 radius
        if (root.endPoint.x >= root.startPoint.x) {
            // Mouse is to the right
            if (root.endPoint.y >= root.startPoint.y) br = 0 // Bottom-Right
            else tr = 0 // Top-Right
        } else {
            // Mouse is to the left
            if (root.endPoint.y >= root.startPoint.y) bl = 0 // Bottom-Left
            else tl = 0 // Top-Left
        }
        
        ctx.beginPath()
        
        // Top edge
        ctx.moveTo(x + tl, y)
        ctx.lineTo(x + w - tr, y)
        if (tr > 0) ctx.quadraticCurveTo(x + w, y, x + w, y + tr)
        
        // Right edge
        ctx.lineTo(x + w, y + h - br)
        if (br > 0) ctx.quadraticCurveTo(x + w, y + h, x + w - br, y + h)
        
        // Bottom edge
        ctx.lineTo(x + bl, y + h)
        if (bl > 0) ctx.quadraticCurveTo(x, y + h, x, y + h - bl)
        
        // Left edge
        ctx.lineTo(x, y + tl)
        if (tl > 0) ctx.quadraticCurveTo(x, y, x + tl, y)
        
        ctx.closePath()
    }

    Canvas {
        id: selectionBorderCanvas
        anchors.fill: parent
        visible: root.isDrawing || root.hasSelection
        
        onPaint: {
            var ctx = getContext("2d")
            ctx.reset()
            
            ctx.lineWidth = 2
            ctx.strokeStyle = "white"
            
            root.drawSelectionPath(ctx, root.selX, root.selY, root.selW, root.selH)
            
            ctx.stroke()
        }
        
        Connections {
            target: root
            function onStartPointChanged() { selectionBorderCanvas.requestPaint() }
            function onEndPointChanged() { selectionBorderCanvas.requestPaint() }
            function onIsDrawingChanged() { selectionBorderCanvas.requestPaint() }
        }
    }
    

    
    // Mask to clip the glow from the inside (keeps the selection 100% native)
    Canvas {
        id: glowMask
        anchors.fill: parent
        visible: false // Used as mask only
        
        onPaint: {
            var ctx = getContext("2d")
            ctx.reset()
            
            // Opaque outside
            ctx.fillStyle = "black"
            ctx.fillRect(0, 0, width, height)
            
            // Cut out inside (make transparent)
            if (root.isDrawing || root.hasSelection) {
                ctx.globalCompositeOperation = "destination-out"
                ctx.fillStyle = "white"
                root.drawSelectionPath(ctx, root.selX, root.selY, root.selW, root.selH)
                ctx.fill()
            }
        }
        
        Connections {
            target: root
            function onStartPointChanged() { glowMask.requestPaint() }
            function onEndPointChanged() { glowMask.requestPaint() }
            function onIsDrawingChanged() { glowMask.requestPaint() }
        }
    }

    // Glow Container with Masking
    Item {
        id: glowWrapper
        anchors.fill: parent
        visible: selectionBorderCanvas.visible
        
        // Layer 1: Wide smoke (Boosted opacity)
        Glow {
            anchors.fill: selectionBorderCanvas
            source: selectionBorderCanvas
            radius: 64
            samples: 64
            color: Qt.rgba(1, 1, 1, 0.8) // Was 0.3, now 0.8 for visibility
            spread: 0.0
            transparentBorder: true
        }

        // Layer 2: Tight aura (Boosted opacity)
        Glow {
            anchors.fill: selectionBorderCanvas
            source: selectionBorderCanvas
            radius: 16
            samples: 32
            color: Qt.rgba(1, 1, 1, 0.9) // Was 0.4, now 0.9
            spread: 0.1
            transparentBorder: true
        }
        
        // Clip the inside of the glow so it doesn't fog up the selection
        layer.enabled: true
        layer.effect: OpacityMask {
            maskSource: glowMask
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
