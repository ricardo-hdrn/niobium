// MobileDisplay — mobile implementation of NbDisplay.
//
// Uses Navigator routes, bottom sheets, and overlays instead of windows.
// Full-screen app — no window management needed.

import 'package:flutter/widgets.dart';

import 'nb_display.dart';

class MobileDisplay extends NbDisplay {
  bool _visible = false;

  @override
  Future<void> init() async {
    // No window initialization needed on mobile.
  }

  @override
  Future<void> showView({
    required Size size,
    NbViewMode mode = NbViewMode.popup,
  }) async {
    // On mobile, size is ignored — views are full-screen or sheet-sized.
    // The actual navigation (push route, show sheet) is handled by the
    // app shell, not the display adapter. This just tracks visibility state.
    _visible = true;
  }

  @override
  Future<void> hideView() async {
    _visible = false;
  }

  @override
  Future<bool> get isVisible async => _visible;

  @override
  Future<void> minimize() async {
    // No-op on mobile — there's no minimize concept.
  }

  @override
  Future<void> setShowInTaskbar(bool show) async {
    // No-op on mobile — always in app switcher.
  }

  @override
  Future<bool> onCloseRequested() async {
    return false; // Let the system handle back navigation.
  }

  @override
  bool get isDesktop => false;

  @override
  void dispose() {}
}
