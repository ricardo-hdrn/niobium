// DesktopDisplay — window_manager implementation of NbDisplay.
//
// Uses popup windows: resize, show/hide, always-on-top, minimize.
// Only imported on desktop platforms (Linux, macOS, Windows).

import 'package:flutter/widgets.dart';
import 'package:window_manager/window_manager.dart';

import 'nb_display.dart';

class DesktopDisplay extends NbDisplay with WindowListener {
  @override
  Future<void> init() async {
    await windowManager.ensureInitialized();

    await windowManager.waitUntilReadyToShow(
      const WindowOptions(
        size: Size(580, 720),
        minimumSize: Size(420, 480),
        center: true,
        title: 'Niobium',
        titleBarStyle: TitleBarStyle.hidden,
        backgroundColor: Color(0x00000000),
      ),
      () async {
        await windowManager.setPreventClose(true);
        await windowManager.setBackgroundColor(const Color(0x00000000));
        // On Linux/GNOME, setMinimizable(false) sets GTK type hint to DIALOG,
        // which bypasses focus-stealing prevention when the window pops up.
        await windowManager.setMinimizable(false);
        await windowManager.setSkipTaskbar(true);
        await windowManager.hide();
      },
    );

    windowManager.addListener(this);
  }

  @override
  Future<void> showView({
    required Size size,
    NbViewMode mode = NbViewMode.popup,
  }) async {
    if (mode == NbViewMode.persistent) {
      await windowManager.setMinimizable(true);
      await windowManager.setSkipTaskbar(false);
    }

    await windowManager.setSize(size);
    await windowManager.center();
    await windowManager.show();
    await windowManager.setAlwaysOnTop(true);
    await windowManager.focus();
  }

  @override
  Future<void> hideView() async {
    await windowManager.setAlwaysOnTop(false);
    await windowManager.setMinimizable(false);
    await windowManager.setSkipTaskbar(true);
    await windowManager.hide();
  }

  @override
  Future<bool> get isVisible => windowManager.isVisible();

  @override
  Future<void> minimize() async {
    await windowManager.minimize();
  }

  @override
  Future<void> setShowInTaskbar(bool show) async {
    await windowManager.setSkipTaskbar(!show);
  }

  @override
  Future<bool> onCloseRequested() async {
    await windowManager.hide();
    return true; // handled — don't actually close
  }

  @override
  void dispose() {
    windowManager.removeListener(this);
  }

  @override
  Widget wrapDraggable({required Widget child}) => DragToMoveArea(child: child);

  @override
  bool get isDesktop => true;

  // WindowListener — forward close to our handler.
  @override
  void onWindowClose() {
    onCloseRequested();
  }
}
