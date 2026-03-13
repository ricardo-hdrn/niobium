// Niobium Design System — Glass morphism + dark theme.
//
// Semi-transparent panels, backdrop blur, teal accent, clean typography.
// Designed for a native desktop feel — Jarvis-inspired, not Material-default.

import 'dart:ui';
import 'package:flutter/material.dart';

// ── Color tokens ────────────────────────────────────────────────────────

class NbColors {
  NbColors._();

  // Background layers (darkest → lightest)
  static const Color bg = Color(0xFF0D1117);
  static const Color surface = Color(0xFF161B22);
  static const Color surfaceElevated = Color(0xFF1C2333);
  static const Color surfaceBright = Color(0xFF242D3D);

  // Glass overlay
  static const Color glass = Color(0x18FFFFFF);
  static const Color glassBorder = Color(0x20FFFFFF);
  static const Color glassHover = Color(0x10FFFFFF);

  // Accent — teal/cyan (Jarvis feel)
  static const Color accent = Color(0xFF00D4AA);
  static const Color accentDim = Color(0xFF00A88A);
  static const Color accentGlow = Color(0x3000D4AA);
  static const Color accentSurface = Color(0x1500D4AA);

  // Text
  static const Color textPrimary = Color(0xFFE6EDF3);
  static const Color textSecondary = Color(0xFF8B949E);
  static const Color textTertiary = Color(0xFF6E7681);
  static const Color textOnAccent = Color(0xFF0D1117);

  // Semantic
  static const Color error = Color(0xFFF85149);
  static const Color errorSurface = Color(0x15F85149);
  static const Color success = Color(0xFF3FB950);
  static const Color warning = Color(0xFFD29922);

  // Input
  static const Color inputBg = Color(0xFF0D1117);
  static const Color inputBorder = Color(0xFF30363D);
  static const Color inputBorderFocus = Color(0xFF00D4AA);
  static const Color inputFill = Color(0xFF161B22);

  // Gradient background
  static const LinearGradient bgGradient = LinearGradient(
    begin: Alignment.topCenter,
    end: Alignment.bottomCenter,
    colors: [Color(0xFF0D1117), Color(0xFF111922), Color(0xFF0D1117)],
  );

  // Glow effect for accent elements
  static const BoxShadow accentGlowShadow = BoxShadow(
    color: Color(0x2000D4AA),
    blurRadius: 12,
    spreadRadius: 2,
  );
}

// ── Spacing / Radius ────────────────────────────────────────────────────

class NbRadius {
  NbRadius._();
  static const double sm = 6;
  static const double md = 10;
  static const double lg = 14;
  static const double xl = 20;
}

class NbSpacing {
  NbSpacing._();
  static const double xs = 4;
  static const double sm = 8;
  static const double md = 16;
  static const double lg = 24;
  static const double xl = 32;
  static const double xxl = 48;
}

// ── Glass widgets ───────────────────────────────────────────────────────

/// A frosted glass panel with backdrop blur and subtle border.
class GlassPanel extends StatelessWidget {
  final Widget child;
  final EdgeInsetsGeometry? padding;
  final double blur;
  final BorderRadius? borderRadius;
  final bool elevated;

  const GlassPanel({
    super.key,
    required this.child,
    this.padding,
    this.blur = 12,
    this.borderRadius,
    this.elevated = false,
  });

  @override
  Widget build(BuildContext context) {
    final radius = borderRadius ?? BorderRadius.circular(NbRadius.lg);
    final effectiveBlur = elevated ? blur * 1.5 : blur;
    return ClipRRect(
      borderRadius: radius,
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: effectiveBlur, sigmaY: effectiveBlur),
        child: Container(
          padding: padding,
          decoration: BoxDecoration(
            color: elevated ? const Color(0x20FFFFFF) : NbColors.glass,
            borderRadius: radius,
            border: Border.all(
              color: elevated ? const Color(0x30FFFFFF) : NbColors.glassBorder,
              width: 0.5,
            ),
          ),
          child: child,
        ),
      ),
    );
  }
}

/// Draggable custom title bar for frameless window.
class NbTitleBar extends StatelessWidget {
  final String title;
  final List<Widget>? actions;
  final VoidCallback? onClose;

  const NbTitleBar({
    super.key,
    required this.title,
    this.actions,
    this.onClose,
  });

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.primary;
    final accentDim = Theme.of(context).colorScheme.secondary;
    return Container(
      height: 48,
      padding: const EdgeInsets.symmetric(horizontal: 16),
      decoration: const BoxDecoration(
        border: Border(
          bottom: BorderSide(color: NbColors.glassBorder, width: 0.5),
        ),
      ),
      child: Row(
        children: [
          // Niobium icon
          Container(
            width: 24,
            height: 24,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              gradient: LinearGradient(
                begin: Alignment.topLeft,
                end: Alignment.bottomRight,
                colors: [accent, accentDim],
              ),
              boxShadow: [
                BoxShadow(
                  color: accent.withValues(alpha: 0.19),
                  blurRadius: 8,
                  spreadRadius: 1,
                ),
              ],
            ),
            child: const Icon(Icons.hexagon_outlined,
                size: 14, color: NbColors.textOnAccent),
          ),
          const SizedBox(width: 12),
          Text(
            title,
            style: const TextStyle(
              color: NbColors.textPrimary,
              fontSize: 13,
              fontWeight: FontWeight.w500,
              letterSpacing: 0.3,
            ),
          ),
          const Spacer(),
          if (actions != null) ...actions!,
          if (onClose != null)
            _WindowButton(
              icon: Icons.close,
              onTap: onClose!,
              hoverColor: NbColors.error.withValues(alpha: 0.15),
            ),
        ],
      ),
    );
  }
}

class _WindowButton extends StatefulWidget {
  final IconData icon;
  final VoidCallback onTap;
  final Color? hoverColor;

  const _WindowButton({
    required this.icon,
    required this.onTap,
    this.hoverColor,
  });

  @override
  State<_WindowButton> createState() => _WindowButtonState();
}

class _WindowButtonState extends State<_WindowButton> {
  bool _hovering = false;

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      onEnter: (_) => setState(() => _hovering = true),
      onExit: (_) => setState(() => _hovering = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: Container(
          width: 32,
          height: 32,
          decoration: BoxDecoration(
            color: _hovering
                ? (widget.hoverColor ?? NbColors.glassHover)
                : Colors.transparent,
            borderRadius: BorderRadius.circular(6),
          ),
          child: Icon(
            widget.icon,
            size: 16,
            color:
                _hovering ? NbColors.textPrimary : NbColors.textSecondary,
          ),
        ),
      ),
    );
  }
}

// ── Accent override ─────────────────────────────────────────────────────

/// Parse a hex color string "#RRGGBB" to a Flutter Color.
Color _parseHex(String hex) {
  final clean = hex.replaceFirst('#', '');
  return Color(int.parse('FF$clean', radix: 16));
}

/// Apply a custom accent color to a base theme.
///
/// Derives dim, glow, and surface variants from the base color.
/// Returns the original theme if [hex] is null.
ThemeData applyAccent(ThemeData base, String? hex) {
  if (hex == null) return base;
  final accent = _parseHex(hex);
  final accentDim = Color.lerp(accent, Colors.black, 0.2)!;

  return base.copyWith(
    colorScheme: base.colorScheme.copyWith(
      primary: accent,
      secondary: accentDim,
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: base.filledButtonTheme.style?.copyWith(
        backgroundColor: WidgetStatePropertyAll(accent),
      ),
    ),
    inputDecorationTheme: base.inputDecorationTheme.copyWith(
      focusedBorder: OutlineInputBorder(
        borderRadius: const BorderRadius.all(Radius.circular(NbRadius.sm)),
        borderSide: BorderSide(color: accent, width: 1.5),
      ),
    ),
    radioTheme: RadioThemeData(
      fillColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.selected)) return accent;
        return NbColors.textTertiary;
      }),
    ),
    checkboxTheme: base.checkboxTheme.copyWith(
      fillColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.selected)) return accent;
        return Colors.transparent;
      }),
    ),
    progressIndicatorTheme: ProgressIndicatorThemeData(color: accent),
  );
}

// ── Theme builder ───────────────────────────────────────────────────────

ThemeData buildNiobiumTheme() {
  const inputBorder = OutlineInputBorder(
    borderRadius: BorderRadius.all(Radius.circular(NbRadius.sm)),
    borderSide: BorderSide(color: NbColors.inputBorder, width: 1),
  );
  const inputBorderFocus = OutlineInputBorder(
    borderRadius: BorderRadius.all(Radius.circular(NbRadius.sm)),
    borderSide: BorderSide(color: NbColors.inputBorderFocus, width: 1.5),
  );
  const inputBorderError = OutlineInputBorder(
    borderRadius: BorderRadius.all(Radius.circular(NbRadius.sm)),
    borderSide: BorderSide(color: NbColors.error, width: 1),
  );

  return ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    scaffoldBackgroundColor: Colors.transparent,
    canvasColor: NbColors.surface,
    cardColor: NbColors.surfaceElevated,

    colorScheme: const ColorScheme.dark(
      primary: NbColors.accent,
      onPrimary: NbColors.textOnAccent,
      secondary: NbColors.accentDim,
      surface: NbColors.surface,
      onSurface: NbColors.textPrimary,
      error: NbColors.error,
      outline: NbColors.inputBorder,
    ),

    // Typography
    textTheme: const TextTheme(
      headlineMedium: TextStyle(
        color: NbColors.textPrimary,
        fontSize: 22,
        fontWeight: FontWeight.w600,
        letterSpacing: -0.3,
      ),
      titleLarge: TextStyle(
        color: NbColors.textPrimary,
        fontSize: 18,
        fontWeight: FontWeight.w600,
        letterSpacing: -0.2,
      ),
      titleMedium: TextStyle(
        color: NbColors.textPrimary,
        fontSize: 15,
        fontWeight: FontWeight.w500,
      ),
      bodyLarge: TextStyle(
        color: NbColors.textPrimary,
        fontSize: 14,
      ),
      bodyMedium: TextStyle(
        color: NbColors.textPrimary,
        fontSize: 13,
      ),
      bodySmall: TextStyle(
        color: NbColors.textSecondary,
        fontSize: 12,
      ),
      labelLarge: TextStyle(
        color: NbColors.textPrimary,
        fontSize: 13,
        fontWeight: FontWeight.w500,
        letterSpacing: 0.2,
      ),
    ),

    // AppBar
    appBarTheme: const AppBarTheme(
      backgroundColor: Colors.transparent,
      elevation: 0,
      scrolledUnderElevation: 0,
      titleTextStyle: TextStyle(
        color: NbColors.textPrimary,
        fontSize: 15,
        fontWeight: FontWeight.w500,
      ),
      iconTheme: IconThemeData(color: NbColors.textSecondary, size: 20),
    ),

    // Input fields
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: NbColors.inputFill,
      contentPadding:
          const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
      border: inputBorder,
      enabledBorder: inputBorder,
      focusedBorder: inputBorderFocus,
      errorBorder: inputBorderError,
      focusedErrorBorder: inputBorderError,
      labelStyle: const TextStyle(
          color: NbColors.textSecondary, fontSize: 13),
      hintStyle: const TextStyle(
          color: NbColors.textTertiary, fontSize: 13),
      errorStyle: const TextStyle(color: NbColors.error, fontSize: 11),
      suffixIconColor: NbColors.textTertiary,
      prefixIconColor: NbColors.textTertiary,
    ),

    // Buttons
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: NbColors.accent,
        foregroundColor: NbColors.textOnAccent,
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(NbRadius.sm),
        ),
        textStyle: const TextStyle(
          fontSize: 13,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.3,
        ),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: NbColors.textPrimary,
        side: const BorderSide(color: NbColors.inputBorder),
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(NbRadius.sm),
        ),
        textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w500),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: NbColors.textSecondary,
        textStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w500),
      ),
    ),

    // Cards
    cardTheme: CardThemeData(
      color: NbColors.surfaceElevated,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(NbRadius.md),
        side: const BorderSide(color: NbColors.glassBorder, width: 0.5),
      ),
    ),

    // Dialogs
    dialogTheme: DialogThemeData(
      backgroundColor: NbColors.surface,
      surfaceTintColor: Colors.transparent,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(NbRadius.lg),
        side: const BorderSide(color: NbColors.glassBorder, width: 0.5),
      ),
    ),

    // Dividers
    dividerTheme: const DividerThemeData(
      color: NbColors.glassBorder,
      thickness: 0.5,
      space: 0,
    ),

    // ListTiles
    listTileTheme: const ListTileThemeData(
      textColor: NbColors.textPrimary,
      iconColor: NbColors.textSecondary,
      contentPadding: EdgeInsets.symmetric(horizontal: 16),
      dense: true,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.all(Radius.circular(NbRadius.sm)),
      ),
    ),

    // Dropdown
    dropdownMenuTheme: DropdownMenuThemeData(
      menuStyle: MenuStyle(
        backgroundColor: WidgetStatePropertyAll(NbColors.surface),
        surfaceTintColor: const WidgetStatePropertyAll(Colors.transparent),
        shape: WidgetStatePropertyAll(RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(NbRadius.md),
          side: const BorderSide(color: NbColors.glassBorder, width: 0.5),
        )),
      ),
    ),

    // Radio / Checkbox
    radioTheme: RadioThemeData(
      fillColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.selected)) return NbColors.accent;
        return NbColors.textTertiary;
      }),
    ),
    checkboxTheme: CheckboxThemeData(
      fillColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.selected)) return NbColors.accent;
        return Colors.transparent;
      }),
      side: const BorderSide(color: NbColors.textTertiary, width: 1.5),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(3),
      ),
    ),

    // Scrollbar
    scrollbarTheme: ScrollbarThemeData(
      thumbColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.hovered) ||
            states.contains(WidgetState.dragged)) {
          return NbColors.textTertiary.withValues(alpha: 0.5);
        }
        return NbColors.textTertiary.withValues(alpha: 0.2);
      }),
      trackColor: const WidgetStatePropertyAll(Colors.transparent),
      radius: const Radius.circular(4),
      thickness: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.hovered) ||
            states.contains(WidgetState.dragged)) {
          return 6;
        }
        return 4;
      }),
    ),

    // Progress indicator
    progressIndicatorTheme: const ProgressIndicatorThemeData(
      color: NbColors.accent,
    ),

    // Tooltip
    tooltipTheme: TooltipThemeData(
      decoration: BoxDecoration(
        color: NbColors.surfaceBright,
        borderRadius: BorderRadius.circular(NbRadius.sm),
        border: Border.all(color: NbColors.glassBorder, width: 0.5),
      ),
      textStyle: const TextStyle(color: NbColors.textPrimary, fontSize: 12),
    ),
  );
}
