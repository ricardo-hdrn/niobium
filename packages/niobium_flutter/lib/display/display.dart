// Display — barrel export + factory.

import 'dart:io';

import 'nb_display.dart';
import 'desktop_display.dart';
import 'mobile_display.dart';

export 'nb_display.dart';
export 'desktop_display.dart';
export 'mobile_display.dart';

/// Create the appropriate display adapter for the current platform.
NbDisplay createDisplay() {
  if (Platform.isAndroid || Platform.isIOS) {
    return MobileDisplay();
  }
  return DesktopDisplay();
}
