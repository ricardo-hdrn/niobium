// Models for JSON Schema → form field conversion.
//
// These are lightweight representations of JSON Schema properties
// used to drive the dynamic form widget.

/// The widget type Niobium selects for a remote data field.
///
/// Normally determined automatically by [selectComponent] based on
/// [FieldDataSource.estimatedCount]. The agent can force a specific
/// component via `x-source.component`, but that should be the exception.
enum SelectComponent {
  radio,              // ≤ 7 items — inline radio buttons
  dropdown,           // ≤ 30 items — simple dropdown
  searchableDropdown, // ≤ 200 items — dropdown with client-side filter
  autocomplete,       // ≤ 2000 items — server-side search-as-you-type
  modalSearch,        // > 2000 items — full modal with filters + pagination
}

/// Pick the best select-one widget for the given estimated item count.
///
/// This is the deterministic intelligence inside Niobium — the agent
/// provides contextual hints, Niobium picks the best UX.
SelectComponent selectComponent(int? estimatedCount, {SelectComponent? forced}) {
  if (forced != null) return forced;
  if (estimatedCount == null) return SelectComponent.dropdown; // safe default
  if (estimatedCount <= 5) return SelectComponent.radio;
  if (estimatedCount <= 25) return SelectComponent.dropdown;
  if (estimatedCount <= 200) return SelectComponent.searchableDropdown;
  if (estimatedCount <= 2000) return SelectComponent.autocomplete;
  return SelectComponent.modalSearch;
}

/// Parse a component name string into [SelectComponent].
SelectComponent? _parseComponent(String? name) {
  if (name == null) return null;
  return switch (name) {
    'radio' => SelectComponent.radio,
    'dropdown' => SelectComponent.dropdown,
    'searchable_dropdown' || 'searchableDropdown' => SelectComponent.searchableDropdown,
    'autocomplete' => SelectComponent.autocomplete,
    'modal_search' || 'modalSearch' => SelectComponent.modalSearch,
    _ => null,
  };
}

/// Definition of a filter that the modal search can expose.
class SourceFilter {
  final String param;   // query parameter name
  final String label;   // display label
  final String type;    // "enum", "text", "boolean"
  final List<dynamic>? values; // for enum type

  SourceFilter({
    required this.param,
    required this.label,
    this.type = 'text',
    this.values,
  });

  factory SourceFilter.fromJson(Map<String, dynamic> json) {
    return SourceFilter(
      param: json['param'] as String,
      label: json['label'] as String? ?? json['param'] as String,
      type: json['type'] as String? ?? 'text',
      values: json['values'] as List<dynamic>?,
    );
  }
}

/// Configuration for fetching dropdown options from a remote endpoint.
///
/// Declared in JSON Schema as:
/// ```json
/// {
///   "type": "string",
///   "x-source": {
///     "url": "https://api.example.com/users",
///     "path": "data.items",
///     "label": "name",
///     "value": "id",
///     "headers": {"Authorization": "Bearer ..."},
///     "estimated_count": 5000,
///     "search_param": "q",
///     "page_param": "_page",
///     "page_size": 20,
///     "filters": [
///       {"param": "role", "label": "Role", "type": "enum", "values": ["admin","user"]},
///       {"param": "name", "label": "Name", "type": "text"}
///     ],
///     "component": "modal_search"
///   }
/// }
/// ```
class FieldDataSource {
  final String url;
  final String? path;
  final String label;
  final String? value;
  final Map<String, String>? headers;

  // Agent hints for scale-aware component selection
  final int? estimatedCount;
  final String? searchParam;   // query param for server-side search (e.g. "q")
  final String? pageParam;     // query param for pagination (e.g. "_page")
  final int? pageSize;
  final List<SourceFilter>? filters;

  // Agent can force a component — exception, not the norm
  final SelectComponent? component;

  FieldDataSource({
    required this.url,
    this.path,
    required this.label,
    this.value,
    this.headers,
    this.estimatedCount,
    this.searchParam,
    this.pageParam,
    this.pageSize,
    this.filters,
    this.component,
  });

  /// The resolved component type, applying deterministic intelligence.
  SelectComponent get resolvedComponent =>
      selectComponent(estimatedCount, forced: component);

  factory FieldDataSource.fromJson(Map<String, dynamic> json) {
    return FieldDataSource(
      url: json['url'] as String,
      path: json['path'] as String?,
      label: json['label'] as String,
      value: json['value'] as String?,
      headers: (json['headers'] as Map<String, dynamic>?)?.cast<String, String>(),
      estimatedCount: json['estimated_count'] as int?,
      searchParam: json['search_param'] as String?,
      pageParam: json['page_param'] as String?,
      pageSize: json['page_size'] as int?,
      filters: (json['filters'] as List<dynamic>?)
          ?.map((f) => SourceFilter.fromJson(f as Map<String, dynamic>))
          .toList(),
      component: _parseComponent(json['component'] as String?),
    );
  }
}

class NbFormField {
  final String name;
  final String type; // string, number, integer, boolean, array, object
  final String label;
  final bool required;
  final String? description;
  final dynamic defaultValue;
  final List<dynamic>? enumValues;
  final String? format; // date, email, uri, password, etc.
  final String? pattern;
  final int? minLength;
  final int? maxLength;
  final num? minimum;
  final num? maximum;
  final num? multipleOf; // step size for sliders
  final Map<String, NbFormField>? properties; // for nested objects
  final NbFormField? items; // for array items
  final FieldDataSource? dataSource; // remote options via x-source
  final List<String>? fileTypes; // file extension filters from x-file-types

  NbFormField({
    required this.name,
    required this.type,
    required this.label,
    this.required = false,
    this.description,
    this.defaultValue,
    this.enumValues,
    this.format,
    this.pattern,
    this.minLength,
    this.maxLength,
    this.minimum,
    this.maximum,
    this.multipleOf,
    this.properties,
    this.items,
    this.dataSource,
    this.fileTypes,
  });
}

/// A complete form request from the MCP server.
class FormRequest {
  final Map<String, dynamic> schema;
  final String title;
  final Map<String, dynamic>? prefill;

  FormRequest({
    required this.schema,
    required this.title,
    this.prefill,
  });

  factory FormRequest.fromJson(Map<String, dynamic> json) {
    return FormRequest(
      schema: json['schema'] as Map<String, dynamic>,
      title: (json['title'] as String?) ?? 'Form',
      prefill: json['prefill'] as Map<String, dynamic>?,
    );
  }
}

/// Result of form submission.
class FormResponse {
  final Map<String, dynamic> data;
  final bool cancelled;

  FormResponse({required this.data, this.cancelled = false});

  Map<String, dynamic> toJson() => {
        'data': data,
        'cancelled': cancelled,
      };
}

/// A confirmation dialog request.
class ConfirmationRequest {
  final String message;
  final String title;

  ConfirmationRequest({required this.message, required this.title});

  factory ConfirmationRequest.fromJson(Map<String, dynamic> json) {
    return ConfirmationRequest(
      message: json['message'] as String,
      title: (json['title'] as String?) ?? 'Confirm',
    );
  }
}
