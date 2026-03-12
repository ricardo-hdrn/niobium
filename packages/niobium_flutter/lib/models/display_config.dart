/// Agent-controllable display parameters for Niobium windows.
///
/// Parsed from the FFI JSON payload — all fields are optional with sensible defaults.
/// Rust resolves mode strings ("wide", "compact") to concrete values before sending.
class NbDisplayConfig {
  final double? width;
  final double? height;
  final String density; // "compact" | "normal" | "comfortable"
  final bool animate;
  final String? accent; // resolved hex "#RRGGBB" or null (use theme default)

  const NbDisplayConfig({
    this.width,
    this.height,
    this.density = 'normal',
    this.animate = true,
    this.accent,
  });

  factory NbDisplayConfig.fromJson(Map<String, dynamic> json) {
    return NbDisplayConfig(
      width: (json['width'] as num?)?.toDouble(),
      height: (json['height'] as num?)?.toDouble(),
      density: (json['density'] as String?) ?? 'normal',
      animate: (json['animate'] as bool?) ?? true,
      accent: json['accent'] as String?,
    );
  }

  /// Vertical spacing between form fields.
  double get fieldSpacing => switch (density) {
        'compact' => 8,
        'comfortable' => 24,
        _ => 16, // "normal"
      };

  /// Horizontal body padding.
  double get bodyPaddingH => switch (density) {
        'compact' => 16,
        'comfortable' => 32,
        _ => 24, // "normal"
      };

  /// Vertical body padding.
  double get bodyPaddingV => switch (density) {
        'compact' => 8,
        'comfortable' => 24,
        _ => 16, // "normal"
      };

  static const defaultConfig = NbDisplayConfig();
}
