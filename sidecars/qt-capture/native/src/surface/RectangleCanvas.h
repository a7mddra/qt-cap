/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once

#ifndef RECTANGLECANVAS_H
#define RECTANGLECANVAS_H

#include <QWidget>
#include <QImage>
#include <QPropertyAnimation>

class RectangleCanvas : public QWidget
{
    Q_OBJECT
    Q_PROPERTY(qreal gradientOpacity READ gradientOpacity WRITE setGradientOpacity)

public:
    explicit RectangleCanvas(const QImage &background, QWidget *parent = nullptr);

protected:
    void showEvent(QShowEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void paintEvent(QPaintEvent *event) override;

private:
    qreal gradientOpacity() const;
    void setGradientOpacity(qreal opacity);
    void clearSelection();
    void cropAndFinish();

    QImage m_background;

    QPointF m_startPoint;
    QPointF m_endPoint;
    QPointF m_currentMousePos;
    bool m_isDrawing = false;
    bool m_hasSelection = false;

    qreal m_gradientOpacity = 0.0;
    QPropertyAnimation *m_animation;
};

#endif // RECTANGLECANVAS_H
