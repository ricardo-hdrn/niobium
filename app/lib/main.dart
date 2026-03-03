// Niobium Flutter Desktop App
//
// Rust MCP server runs in-process via flutter_rust_bridge (FFI).
// Single process — no HTTP server needed.

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';
import 'src/rust/api.dart' as rust_api;
import 'src/rust/frb_generated.dart';
import 'models/display_config.dart';
import 'widgets/dynamic_form.dart';
import 'widgets/confirmation_dialog.dart';
import 'widgets/output_display.dart';
import 'theme/niobium_theme.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await windowManager.ensureInitialized();

  windowManager.waitUntilReadyToShow(
    const WindowOptions(
      size: Size(580, 720),
      minimumSize: Size(420, 480),
      center: true,
      title: 'Niobium',
      titleBarStyle: TitleBarStyle.hidden,
      backgroundColor: Colors.transparent,
    ),
    () async {
      await windowManager.setPreventClose(true);
      await windowManager.setBackgroundColor(Colors.transparent);
      // On Linux/GNOME, setMinimizable(false) sets GTK type hint to DIALOG,
      // which bypasses focus-stealing prevention when the window pops up.
      await windowManager.setMinimizable(false);
      await windowManager.setSkipTaskbar(true);
      await windowManager.hide();
    },
  );

  await RustLib.init();
  runApp(const NiobiumApp());
}

class NiobiumApp extends StatefulWidget {
  const NiobiumApp({super.key});

  @override
  State<NiobiumApp> createState() => _NiobiumAppState();
}

class _NiobiumAppState extends State<NiobiumApp> with WindowListener {
  final _scaffoldMessengerKey = GlobalKey<ScaffoldMessengerState>();

  // Current UI to display — null means idle (hidden)
  Widget? _currentView;

  @override
  void initState() {
    super.initState();
    windowManager.addListener(this);
    _startFfiServer();
  }

  @override
  void onWindowClose() async {
    await windowManager.hide();
  }

  // ── FFI mode (single-process) ───────────────────────────────────────

  Future<void> _startFfiServer() async {
    // Initialize Rust logging
    await rust_api.initLogging();

    // Start MCP server in background with Dart callbacks for UI
    rust_api.startMcpServer(
      showForm: _handleShowFormFfi,
      showConfirm: _handleShowConfirmFfi,
      showToast: _handleShowToastFfi,
      showOutput: _handleShowOutputFfi,
    ).then((_) {
      // MCP server exited (stdin closed) — shut down the app
      exit(0);
    }).catchError((e) {
      stderr.writeln('MCP server error: $e');
      exit(1);
    });
  }

  /// FFI callback: agent requested a form.
  /// Receives JSON payload, returns JSON result or null if cancelled.
  Future<String?> _handleShowFormFfi(String payload) async {
    final json = jsonDecode(payload) as Map<String, dynamic>;
    final schema = json['schema'] as Map<String, dynamic>;
    final title = (json['title'] as String?) ?? 'Form';
    final prefill = json['prefill'] as Map<String, dynamic>?;
    final display = NbDisplayConfig.fromJson(json);

    final result = await _handleShowForm(schema, title, prefill, display: display);
    if (result == null) return null;
    return jsonEncode(result);
  }

  /// FFI callback: agent requested confirmation.
  /// Receives JSON payload, returns bool.
  Future<bool> _handleShowConfirmFfi(String payload) async {
    final json = jsonDecode(payload) as Map<String, dynamic>;
    final message = json['message'] as String;
    final title = (json['title'] as String?) ?? 'Confirm';
    final display = NbDisplayConfig.fromJson(json);
    return _handleShowConfirmation(message, title, display: display);
  }

  /// FFI callback: pipeline emitted a toast notification.
  /// Receives JSON payload, fire-and-forget.
  Future<void> _handleShowToastFfi(String payload) async {
    final json = jsonDecode(payload) as Map<String, dynamic>;
    final message = json['message'] as String;
    final severity = (json['severity'] as String?) ?? 'info';
    _handleShowToast(message, severity);
  }

  /// FFI callback: agent requested rich output display.
  /// Receives JSON payload, returns bool when dismissed.
  Future<bool> _handleShowOutputFfi(String payload) async {
    final json = jsonDecode(payload) as Map<String, dynamic>;
    final content = json['content'] as String;
    final outputType = (json['output_type'] as String?) ?? 'text';
    final title = (json['title'] as String?) ?? 'Output';
    final display = NbDisplayConfig.fromJson(json);
    return _handleShowOutput(content, outputType, title, display: display);
  }

  // ── Request handlers ─────────────────────────────────────────────────

  Future<void> _showWindow() async {
    await windowManager.show();
    await windowManager.setAlwaysOnTop(true);
    await windowManager.focus();
  }

  Future<void> _hideWindow() async {
    // Drop always-on-top only when hiding, so the form stays on top while visible.
    await windowManager.setAlwaysOnTop(false);
    await windowManager.hide();
  }

  Future<Map<String, dynamic>?> _handleShowForm(
    Map<String, dynamic> schema,
    String title,
    Map<String, dynamic>? prefill, {
    NbDisplayConfig display = NbDisplayConfig.defaultConfig,
  }) async {
    final completer = Completer<Map<String, dynamic>?>();

    final formWidget = DynamicForm(
      schema: schema,
      title: title,
      prefill: prefill,
      completer: completer,
      display: display,
    );

    setState(() {
      _currentView = display.accent != null
          ? Theme(data: applyAccent(buildNiobiumTheme(), display.accent), child: formWidget)
          : formWidget;
    });

    final w = display.width ?? 580;
    final h = display.height ?? 720;
    await windowManager.setSize(Size(w, h));
    await windowManager.center();
    await _showWindow();
    final result = await completer.future;

    setState(() => _currentView = null);
    await _hideWindow();
    await windowManager.setSize(const Size(580, 720));

    return result;
  }

  Future<bool> _handleShowConfirmation(String message, String title, {
    NbDisplayConfig display = NbDisplayConfig.defaultConfig,
  }) async {
    final completer = Completer<bool>();

    final dialogWidget = ConfirmationDialog(
      message: message,
      title: title,
      completer: completer,
    );

    setState(() {
      _currentView = display.accent != null
          ? Theme(data: applyAccent(buildNiobiumTheme(), display.accent), child: dialogWidget)
          : dialogWidget;
    });

    final w = display.width ?? 580;
    final h = display.height ?? 720;
    await windowManager.setSize(Size(w, h));
    await windowManager.center();
    await _showWindow();
    final result = await completer.future;

    setState(() => _currentView = null);
    await _hideWindow();
    await windowManager.setSize(const Size(580, 720));

    return result;
  }

  void _handleShowToast(String message, String severity) async {
    final color = switch (severity) {
      'success' => NbColors.success,
      'warning' => NbColors.warning,
      'error' => NbColors.error,
      _ => NbColors.accent,
    };

    final icon = switch (severity) {
      'success' => Icons.check_circle_outline,
      'warning' => Icons.warning_amber,
      'error' => Icons.error_outline,
      _ => Icons.info_outline,
    };

    // If window is already visible (during a form/output), use SnackBar overlay
    final wasVisible = await windowManager.isVisible();
    if (wasVisible) {
      _scaffoldMessengerKey.currentState?.showSnackBar(
        SnackBar(
          content: Row(
            children: [
              Icon(icon, color: color, size: 18),
              const SizedBox(width: 10),
              Expanded(
                child: Text(message,
                    style: const TextStyle(color: NbColors.textPrimary)),
              ),
            ],
          ),
          backgroundColor: NbColors.surfaceElevated,
          behavior: SnackBarBehavior.floating,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(NbRadius.sm),
            side: BorderSide(color: color.withValues(alpha: 0.3)),
          ),
          duration: const Duration(seconds: 4),
        ),
      );
      return;
    }

    // Window is hidden — show a small toast popup
    const toastDuration = Duration(seconds: 3);

    setState(() {
      _currentView = _ToastPopup(
        message: message,
        icon: icon,
        color: color,
      );
    });

    await windowManager.setSize(const Size(400, 72));
    await _showWindow();

    await Future.delayed(toastDuration);

    // Dismiss toast and restore window
    if (_currentView is _ToastPopup) {
      setState(() => _currentView = null);
      await _hideWindow();
      await windowManager.setSize(const Size(580, 720));
      await windowManager.center();
    }
  }

  Future<bool> _handleShowOutput(
      String content, String outputType, String title, {
      NbDisplayConfig display = NbDisplayConfig.defaultConfig,
  }) async {
    final completer = Completer<bool>();

    final outputWidget = OutputDisplay(
      content: content,
      outputType: outputType,
      title: title,
      completer: completer,
    );

    setState(() {
      _currentView = display.accent != null
          ? Theme(data: applyAccent(buildNiobiumTheme(), display.accent), child: outputWidget)
          : outputWidget;
    });

    final w = display.width ?? 580;
    final h = display.height ?? 720;
    await windowManager.setSize(Size(w, h));
    await windowManager.center();
    await _showWindow();
    final result = await completer.future;

    setState(() => _currentView = null);
    await _hideWindow();
    await windowManager.setSize(const Size(580, 720));

    return result;
  }

  @override
  void dispose() {
    windowManager.removeListener(this);
    super.dispose();
  }

  // ── UI ───────────────────────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Niobium',
      debugShowCheckedModeBanner: false,
      scaffoldMessengerKey: _scaffoldMessengerKey,
      theme: buildNiobiumTheme(),
      home: Container(
        decoration: const BoxDecoration(
          gradient: NbColors.bgGradient,
          borderRadius: BorderRadius.all(Radius.circular(10)),
        ),
        clipBehavior: Clip.antiAlias,
        child: AnimatedSwitcher(
          duration: const Duration(milliseconds: 200),
          switchInCurve: Curves.easeOutCubic,
          switchOutCurve: Curves.easeInCubic,
          child: _currentView ??
              const Scaffold(
                key: ValueKey('idle'),
                backgroundColor: Colors.transparent,
                body: SizedBox.shrink(),
              ),
        ),
      ),
    );
  }
}

/// Compact toast widget with slide-in animation.
class _ToastPopup extends StatefulWidget {
  final String message;
  final IconData icon;
  final Color color;

  const _ToastPopup({
    required this.message,
    required this.icon,
    required this.color,
  });

  @override
  State<_ToastPopup> createState() => _ToastPopupState();
}

class _ToastPopupState extends State<_ToastPopup>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<Offset> _slideAnimation;
  late final Animation<double> _fadeAnimation;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      duration: const Duration(milliseconds: 250),
      vsync: this,
    );
    _slideAnimation = Tween<Offset>(
      begin: const Offset(0.3, 0),
      end: Offset.zero,
    ).animate(CurvedAnimation(parent: _controller, curve: Curves.easeOutCubic));
    _fadeAnimation = CurvedAnimation(parent: _controller, curve: Curves.easeOut);
    _controller.forward();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Center(
        child: SlideTransition(
          position: _slideAnimation,
          child: FadeTransition(
            opacity: _fadeAnimation,
            child: Padding(
              padding:
                  const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
              child: Row(
                children: [
                  Icon(widget.icon, color: widget.color, size: 20),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text(
                      widget.message,
                      style: const TextStyle(
                        color: NbColors.textPrimary,
                        fontSize: 13,
                      ),
                      overflow: TextOverflow.ellipsis,
                      maxLines: 2,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
