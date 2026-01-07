/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

#include "RectangleCanvas.h"
#include <QApplication>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QDebug>
#include <QDir>
#include <iostream>

RectangleCanvas::RectangleCanvas(const QImage &background, QWidget *parent)
    : QWidget(parent),
      m_background(background),
      m_startPoint(0, 0),
      m_endPoint(0, 0),
      m_currentMousePos(0, 0),
      m_gradientOpacity(0.0),
      m_animation(nullptr)
{
    setMouseTracking(true);
    setCursor(Qt::CrossCursor);
    setContentsMargins(0, 0, 0, 0);

    qreal dpr = m_background.devicePixelRatio();
    if (dpr <= 0.0) dpr = 1.0;

    setFixedSize(qRound(m_background.width() / dpr), qRound(m_background.height() / dpr));

    m_animation = new QPropertyAnimation(this, "gradientOpacity");
    m_animation->setDuration(200);
    m_animation->setStartValue(0.0);
    m_animation->setEndValue(1.0);

    clearSelection();
}

void RectangleCanvas::showEvent(QShowEvent *event)
{
    QWidget::showEvent(event);
    m_animation->start();
}

qreal RectangleCanvas::gradientOpacity() const { return m_gradientOpacity; }

void RectangleCanvas::setGradientOpacity(qreal opacity)
{
    m_gradientOpacity = opacity;
    update();
}

void RectangleCanvas::mousePressEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton)
    {
        if (m_hasSelection)
            clearSelection();
        m_isDrawing = true;
        m_startPoint = event->pos();
        m_endPoint = event->pos();
        m_currentMousePos = event->pos();
        update();
    }
}

void RectangleCanvas::mouseMoveEvent(QMouseEvent *event)
{
    m_currentMousePos = event->pos();
    if (m_isDrawing)
    {
        m_endPoint = event->pos();
    }
    update();
}

void RectangleCanvas::mouseReleaseEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton && m_isDrawing)
    {
        m_endPoint = event->pos();
        m_isDrawing = false;
        m_hasSelection = true;
        cropAndFinish();
    }
}

void RectangleCanvas::keyPressEvent(QKeyEvent *event)
{
    if (event->key() == Qt::Key_Escape || event->key() == Qt::Key_Q)
    {
        QApplication::exit(1);
    }
}

void RectangleCanvas::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event);
    QPainter painter(this);

    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setRenderHint(QPainter::SmoothPixmapTransform, true);

    // Draw background
    painter.drawImage(rect(), m_background);

    // Draw gradient overlay
    QLinearGradient gradient(0, 0, 0, height());
    gradient.setColorAt(0.0, QColor(0, 0, 0, static_cast<int>(128 * m_gradientOpacity)));
    gradient.setColorAt(1.0, QColor(0, 0, 0, 0));
    painter.setCompositionMode(QPainter::CompositionMode_SourceOver);
    painter.fillRect(rect(), gradient);

    // Draw selection rectangle if drawing
    if (m_isDrawing || m_hasSelection)
    {
        QRectF selectionRect = QRectF(m_startPoint, m_endPoint).normalized();

        // Draw dark overlay outside selection
        QPainterPath overlayPath;
        overlayPath.addRect(rect());
        QPainterPath selectionPath;
        selectionPath.addRect(selectionRect);
        overlayPath = overlayPath.subtracted(selectionPath);
        painter.fillPath(overlayPath, QColor(0, 0, 0, 100));

        // Draw selection border
        QPen borderPen(Qt::white, 2, Qt::SolidLine);
        painter.setPen(borderPen);
        painter.setBrush(Qt::NoBrush);
        painter.drawRect(selectionRect);

        // Draw corner handles
        const qreal handleSize = 8.0;
        painter.setBrush(Qt::white);
        painter.setPen(Qt::NoPen);

        QPointF corners[4] = {
            selectionRect.topLeft(),
            selectionRect.topRight(),
            selectionRect.bottomLeft(),
            selectionRect.bottomRight()
        };

        for (const auto &corner : corners)
        {
            painter.drawEllipse(corner, handleSize / 2, handleSize / 2);
        }

        // Draw dimensions text
        int w = qRound(selectionRect.width());
        int h = qRound(selectionRect.height());
        QString dimText = QString("%1 × %2").arg(w).arg(h);

        QFont font = painter.font();
        font.setPointSize(11);
        font.setBold(true);
        painter.setFont(font);

        QFontMetrics fm(font);
        QRectF textRect = fm.boundingRect(dimText);
        textRect.moveCenter(QPointF(selectionRect.center().x(), selectionRect.bottom() + 20));

        // Draw text background
        QRectF bgRect = textRect.adjusted(-8, -4, 8, 4);
        painter.setBrush(QColor(0, 0, 0, 180));
        painter.setPen(Qt::NoPen);
        painter.drawRoundedRect(bgRect, 4, 4);

        // Draw text
        painter.setPen(Qt::white);
        painter.drawText(textRect, Qt::AlignCenter, dimText);
    }

    // Draw crosshair cursor when not drawing
    if (!m_isDrawing && !m_hasSelection)
    {
        const qreal crosshairSize = 20.0;
        QPen crosshairPen(Qt::white, 1, Qt::SolidLine);
        painter.setPen(crosshairPen);
        painter.drawLine(QPointF(m_currentMousePos.x() - crosshairSize, m_currentMousePos.y()),
                         QPointF(m_currentMousePos.x() + crosshairSize, m_currentMousePos.y()));
        painter.drawLine(QPointF(m_currentMousePos.x(), m_currentMousePos.y() - crosshairSize),
                         QPointF(m_currentMousePos.x(), m_currentMousePos.y() + crosshairSize));
    }
}

void RectangleCanvas::clearSelection()
{
    m_startPoint = QPointF(0, 0);
    m_endPoint = QPointF(0, 0);
    m_isDrawing = false;
    m_hasSelection = false;
    update();
}

void RectangleCanvas::cropAndFinish()
{
    QRectF selectionRect = QRectF(m_startPoint, m_endPoint).normalized();

    qreal dpr = m_background.devicePixelRatio();
    if (dpr <= 0.0) dpr = 1.0;

    int physX = qRound(selectionRect.x() * dpr);
    int physY = qRound(selectionRect.y() * dpr);
    int physW = qRound(selectionRect.width() * dpr);
    int physH = qRound(selectionRect.height() * dpr);

    if (physX < 0)
        physX = 0;
    if (physY < 0)
        physY = 0;
    if (physX + physW > m_background.width())
        physW = m_background.width() - physX;
    if (physY + physH > m_background.height())
        physH = m_background.height() - physY;

    if (physW <= 0 || physH <= 0)
    {
        QApplication::exit(1);
        return;
    }

    QImage cropped = m_background.copy(physX, physY, physW, physH);
    cropped.setDevicePixelRatio(1.0);

    QString finalPath = QDir::temp().filePath("spatial_capture.png");
    if (cropped.save(finalPath, "PNG", -1))
    {
        std::cout << finalPath.toStdString() << std::endl;
        QApplication::exit(0);
    }
    else
    {
        QApplication::exit(1);
    }
}
