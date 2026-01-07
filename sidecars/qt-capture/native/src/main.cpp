/**
 * @license
 * Copyright 2026 a7mddra
 * SPDX-License-Identifier: Apache-2.0
 */

#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQmlComponent>
#include <QQuickWindow>
#include <QCommandLineParser>
#include <QDebug>
#include <QScreen>
#include <vector>

#include "config.h"
#include "surface/CaptureMode.h"
#include "shutter/ScreenGrabber.h"
#include "qml/CaptureController.h"

#ifdef Q_OS_WIN
#include <windows.h>
#include <dwmapi.h>
#endif

#ifdef Q_OS_MAC
#include <objc/runtime.h>
#include <objc/message.h>
#endif

extern "C" ScreenGrabber *createWindowsEngine(QObject *parent);
extern "C" ScreenGrabber *createUnixEngine(QObject *parent);

/**
 * Apply platform-specific window hacks for instant appearance.
 * These bypass OS window animations (macOS zoom, Windows fade).
 */
static void applyPlatformWindowHacks(QQuickWindow *window)
{
#ifdef Q_OS_WIN
    // Disable Windows DWM transition animations
    HWND hwnd = reinterpret_cast<HWND>(window->winId());
    BOOL attrib = TRUE;
    DwmSetWindowAttribute(hwnd, DWMWA_TRANSITIONS_FORCEDISABLED, &attrib, sizeof(attrib));
#endif

#ifdef Q_OS_MAC
    // Disable macOS window animations via Objective-C runtime
    // This matches the original OverlayWindow.mm behavior
    WId nativeId = window->winId();
    id nsView = reinterpret_cast<id>(nativeId);
    if (nsView)
    {
        // Get NSWindow from NSView
        id nsWindow = ((id(*)(id, SEL))objc_msgSend)(nsView, sel_registerName("window"));
        if (nsWindow)
        {
            // [nsWindow setAnimationBehavior:NSWindowAnimationBehaviorNone]
            ((void (*)(id, SEL, long))objc_msgSend)(nsWindow, sel_registerName("setAnimationBehavior:"), 2);
            // [nsWindow setHasShadow:NO]
            ((void (*)(id, SEL, BOOL))objc_msgSend)(nsWindow, sel_registerName("setHasShadow:"), NO);
            // [nsWindow setLevel:NSFloatingWindowLevel]
            ((void (*)(id, SEL, long))objc_msgSend)(nsWindow, sel_registerName("setLevel:"), 5);
        }
    }
#endif
}

int main(int argc, char *argv[])
{
    // High DPI setup (same as original)
    QCoreApplication::setAttribute(Qt::AA_EnableHighDpiScaling);
    QCoreApplication::setAttribute(Qt::AA_UseHighDpiPixmaps);
#if QT_VERSION >= QT_VERSION_CHECK(5, 14, 0)
    QGuiApplication::setHighDpiScaleFactorRoundingPolicy(Qt::HighDpiScaleFactorRoundingPolicy::PassThrough);
#endif

#ifdef Q_OS_WIN
    // Windows DPI awareness setup (unchanged from original)
    HMODULE user32 = LoadLibraryW(L"user32.dll");
    if (user32)
    {
        using SetProcessDpiAwarenessContextFn = BOOL(WINAPI *)(HANDLE);
        auto fn = reinterpret_cast<SetProcessDpiAwarenessContextFn>(GetProcAddress(user32, "SetProcessDpiAwarenessContext"));
        if (fn)
        {
            fn(reinterpret_cast<HANDLE>(-4));
        }
        else
        {
            HMODULE shcore = LoadLibraryW(L"Shcore.dll");
            if (shcore)
            {
                using SetProcessDpiAwarenessFn = HRESULT(WINAPI *)(int);
                auto fn2 = reinterpret_cast<SetProcessDpiAwarenessFn>(GetProcAddress(shcore, "SetProcessDpiAwareness"));
                if (fn2)
                {
                    constexpr int PROCESS_PER_MONITOR_DPI_AWARE = 2;
                    fn2(PROCESS_PER_MONITOR_DPI_AWARE);
                }
                FreeLibrary(shcore);
            }
            else
            {
                using SetProcessDPIAwareFn = BOOL(WINAPI *)();
                auto fn3 = reinterpret_cast<SetProcessDPIAwareFn>(GetProcAddress(user32, "SetProcessDPIAware"));
                if (fn3)
                    fn3();
            }
        }
        FreeLibrary(user32);
    }
#endif

#ifdef Q_OS_LINUX
    qputenv("QT_QPA_PLATFORM", "xcb");
#endif

    QGuiApplication app(argc, argv);

    app.setApplicationName(APP_NAME);
    app.setOrganizationName(ORG_NAME);
    app.setApplicationVersion(APP_VERSION);
    app.setQuitOnLastWindowClosed(true);

    // CLI parsing (unchanged from original)
    QCommandLineParser parser;
    parser.setApplicationDescription("Screen capture tool with selection modes");
    parser.addHelpOption();
    parser.addVersionOption();

    QCommandLineOption freeshapeOption(
        QStringList() << "f" << "freeshape",
        "Use freeshape (squiggle) selection mode (default)");
    parser.addOption(freeshapeOption);

    QCommandLineOption rectangleOption(
        QStringList() << "r" << "rectangle",
        "Use rectangle selection mode");
    parser.addOption(rectangleOption);

    parser.process(app);

    QString captureMode = "freeshape";
    if (parser.isSet(rectangleOption))
    {
        captureMode = "rectangle";
        qDebug() << "Capture mode: Rectangle";
    }
    else
    {
        qDebug() << "Capture mode: Freeshape";
    }

    // Screen capture (unchanged - uses existing ScreenGrabber backend)
    ScreenGrabber *engine = nullptr;
#ifdef Q_OS_WIN
    engine = createWindowsEngine(&app);
#else
    engine = createUnixEngine(&app);
#endif

    if (!engine)
    {
        qCritical() << "FATAL: Failed to initialize Capture Engine.";
        return 1;
    }

    std::vector<CapturedFrame> frames = engine->captureAll();

    if (frames.empty())
    {
        qCritical() << "FATAL: No screens captured.";
        return 1;
    }

    // Get Qt screens for positioning
    QList<QScreen *> qtScreens = app.screens();

    // QML engine setup
    QQmlApplicationEngine qmlEngine;

    // Store controllers and windows to prevent garbage collection
    std::vector<CaptureController *> controllers;
    std::vector<QQuickWindow *> windows;

    // Create one QML window per captured display
    for (const auto &frame : frames)
    {
        qDebug() << "Display" << frame.index
                 << "|" << frame.name
                 << "|" << frame.geometry
                 << "| DPR:" << frame.devicePixelRatio;

        // Find the matching Qt screen
        QScreen *targetScreen = nullptr;
        for (QScreen *s : qtScreens)
        {
            if (s->name() == frame.name)
            {
                targetScreen = s;
                break;
            }
        }
        if (!targetScreen)
        {
            for (QScreen *s : qtScreens)
            {
                if (s->geometry() == frame.geometry)
                {
                    targetScreen = s;
                    break;
                }
            }
        }

        // Create controller for this display
        auto *controller = new CaptureController(&app);
        controller->setDisplayIndex(frame.index);
        controller->setCaptureMode(captureMode);
        controller->setBackgroundImage(frame.image, frame.devicePixelRatio);
        controllers.push_back(controller);

        // Load QML component
        QQmlComponent component(&qmlEngine, QUrl("qrc:/CaptureQml/qml/CaptureWindow.qml"));
        
        if (component.isError())
        {
            qCritical() << "QML load error:" << component.errors();
            return 1;
        }

        // Create window with controller property
        QVariantMap properties;
        properties["controller"] = QVariant::fromValue(controller);

        QObject *obj = component.createWithInitialProperties(properties);
        QQuickWindow *window = qobject_cast<QQuickWindow *>(obj);

        if (!window)
        {
            qCritical() << "Failed to create QML window for display" << frame.index;
            return 1;
        }

        windows.push_back(window);

        // Position window on correct screen
        if (targetScreen)
        {
            window->setScreen(targetScreen);
            window->setGeometry(targetScreen->geometry());
        }
        else
        {
            window->setGeometry(frame.geometry);
        }

        // Apply platform-specific hacks for instant appearance
        applyPlatformWindowHacks(window);

        // Show fullscreen
        window->showFullScreen();
    }

    return app.exec();
}
