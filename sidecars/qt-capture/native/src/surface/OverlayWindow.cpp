/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

#include "OverlayWindow.h"
#include "SquiggleCanvas.h"
#include "RectangleCanvas.h"
#include <QApplication>
#include <QCloseEvent>
#include <QDebug>
#include <QScreen>

#ifdef Q_OS_WIN
#include <windows.h>
#include <dwmapi.h>
#endif

OverlayWindow::OverlayWindow(int displayNum, const QImage &bgImage, const QRect &geo, QScreen *screen, CaptureMode mode, QWidget *parent)
    : QMainWindow(parent),
      m_displayNum(displayNum),
      m_mode(mode),
      m_canvas(nullptr)
{
    if (m_mode == CaptureMode::Rectangle)
    {
        m_canvas = new RectangleCanvas(bgImage, this);
    }
    else
    {
        m_canvas = new SquiggleCanvas(bgImage, this);
    }

    setCentralWidget(m_canvas);
    m_canvas->setFocus();

    setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool | Qt::Popup);
    setAttribute(Qt::WA_ShowWithoutActivating);
    setAttribute(Qt::WA_TranslucentBackground, false);

    // Associate with the target screen first
    if (screen)
    {
        setScreen(screen);
    }

    // Go fullscreen - let Qt handle the geometry
    // Don't call setGeometry before showFullScreen as it conflicts with HiDPI
    showFullScreen();

    // After fullscreen, ensure we use the screen's geometry
    if (screen)
    {
        setGeometry(screen->geometry());
    }
    else
    {
        setGeometry(geo);
    }

    setContentsMargins(0, 0, 0, 0);
    m_canvas->setContentsMargins(0, 0, 0, 0);

#ifdef Q_OS_WIN
    BOOL attrib = TRUE;
    DwmSetWindowAttribute(reinterpret_cast<HWND>(winId()), DWMWA_TRANSITIONS_FORCEDISABLED, &attrib, sizeof(attrib));
#endif
}

OverlayWindow::~OverlayWindow() {}

void OverlayWindow::closeEvent(QCloseEvent *event)
{
    QApplication::exit(1);
    QMainWindow::closeEvent(event);
}

#ifdef Q_OS_WIN
bool OverlayWindow::nativeEvent(const QByteArray &eventType, void *message, qintptr *result)
{
    MSG *msg = static_cast<MSG *>(message);
    if (msg->message == WM_DISPLAYCHANGE)
    {
        QApplication::exit(1);
        return true;
    }
    return QMainWindow::nativeEvent(eventType, message, result);
}
#endif