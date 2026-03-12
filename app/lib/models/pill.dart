// Pill model — generic feed item from any source plugin.
//
// Received as JSON from Rust via FFI callback.
// Each pill becomes a card in the PillsView.

class Pill {
  /// Source plugin identifier (e.g. "hub", "voice", "watcher").
  final String source;

  /// Human-readable summary.
  final String summary;

  /// When this pill was created (ISO 8601).
  final DateTime createdAt;

  /// Output type hint for rendering ("decision", "form", "markdown", "table", etc.).
  final String? outputType;

  /// Decision options (when outputType = "decision").
  final List<String>? options;

  /// Rich content (form schema, table data, markdown text, etc.).
  final dynamic content;

  /// URL to sink the user's response to (remote routing).
  final String? responseUrl;

  /// Source-specific metadata (IDs, refs, etc.).
  final Map<String, dynamic>? meta;

  /// Mutable — set after user responds.
  String? response;

  Pill({
    required this.source,
    required this.summary,
    required this.createdAt,
    this.outputType,
    this.options,
    this.content,
    this.responseUrl,
    this.meta,
    this.response,
  });

  factory Pill.fromJson(Map<String, dynamic> json) {
    // Flat JSON from Rust's Pill struct (serde serialized)
    return Pill(
      source: json['source'] as String? ?? 'unknown',
      summary: json['summary'] as String? ?? '',
      createdAt: _parseCreatedAt(json['created_at']),
      outputType: json['output_type'] as String?,
      options: (json['options'] as List<dynamic>?)?.cast<String>(),
      content: json['content'],
      responseUrl: json['response_url'] as String?,
      meta: json['meta'] as Map<String, dynamic>?,
    );
  }

  bool get isDecision => outputType == 'decision' && options != null;
  bool get hasRemoteSink => responseUrl != null;
  bool get isAnswered => response != null;
  bool get isTappable => outputType != null;

  // Convenience accessors for common meta fields
  String? get sourceKind => meta?['source_kind'] as String?;
  String? get sourceId => meta?['source_id'] as String?;
  String? get subjectId => meta?['subject_id'] as String?;
  String? get newState => meta?['new_state'] as String?;
  String? get newStatus => meta?['new_status'] as String?;

  /// Event type hint from meta (hub-specific: update_event, actionable_state, etc.).
  /// Falls back to source name if not present.
  String get eventType => meta?['event_type'] as String? ?? source;

  static DateTime _parseCreatedAt(dynamic value) {
    if (value is String && value.isNotEmpty) {
      return DateTime.tryParse(value) ?? DateTime.now();
    }
    return DateTime.now();
  }
}
