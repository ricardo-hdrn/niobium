// NbDisplay — platform display adapter.
//
// Abstracts how views are presented: popup windows on desktop,
// routes/sheets/overlays on mobile. Same API, different implementations.

import 'package:flutter/widgets.dart';

/// How a view should be presented.
enum NbViewMode {
  /// Standard popup (form, output, confirmation).
  popup,

  /// Persistent floating view (voice orb, pills).
  persistent,

  /// Ephemeral notification (toast).
  toast,
}

/// Abstract display surface — desktop windows or mobile navigation.
abstract class NbDisplay {
  /// Initialize the display system. Call once at app startup.
  Future<void> init();

  /// Show a view with the given size and mode.
  Future<void> showView({
    required Size size,
    NbViewMode mode = NbViewMode.popup,
  });

  /// Hide the current view.
  Future<void> hideView();

  /// Whether a view is currently visible.
  Future<bool> get isVisible;

  /// Minimize the current view (no-op on mobile).
  Future<void> minimize();

  /// Set whether the view appears in taskbar/app switcher.
  Future<void> setShowInTaskbar(bool show);

  /// Handle app close event. Returns true if handled (prevent default).
  Future<bool> onCloseRequested();

  /// Wrap a widget to make it draggable (title bar drag on desktop, no-op on mobile).
  Widget wrapDraggable({required Widget child}) => child;

  /// Whether the current platform is desktop.
  bool get isDesktop;

  /// Clean up resources.
  void dispose();
}
