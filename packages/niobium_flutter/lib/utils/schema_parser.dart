// Parses JSON Schema into NbFormField objects for the dynamic form widget.
//
// Ported from intelligence/services/form_generator.py.

import '../models/form_schema.dart';

/// Convert a JSON Schema object to a list of NbFormFields.
List<NbFormField> parseSchema(Map<String, dynamic> schema) {
  final properties =
      schema['properties'] as Map<String, dynamic>? ?? {};
  final required =
      (schema['required'] as List<dynamic>?)?.cast<String>() ?? [];

  return properties.entries.map((entry) {
    return _buildField(
      entry.key,
      _resolveRef(entry.value as Map<String, dynamic>, schema),
      entry.key.contains(RegExp(r'^(' + required.join('|') + r')$')),
    );
  }).toList();
}

/// Build a single NbFormField from a JSON Schema property.
NbFormField _buildField(
  String fieldName,
  Map<String, dynamic> fieldSchema,
  bool isRequired,
) {
  final type = fieldSchema['type'] as String? ?? 'string';
  final format = fieldSchema['format'] as String?;

  // Nested object fields
  Map<String, NbFormField>? nestedProperties;
  if (type == 'object' && fieldSchema['properties'] != null) {
    final props = fieldSchema['properties'] as Map<String, dynamic>;
    final nestedRequired =
        (fieldSchema['required'] as List<dynamic>?)?.cast<String>() ?? [];
    nestedProperties = props.map((k, v) => MapEntry(
          k,
          _buildField(k, v as Map<String, dynamic>, nestedRequired.contains(k)),
        ));
  }

  // Array item schema
  NbFormField? itemsField;
  if (type == 'array' && fieldSchema['items'] != null) {
    final itemSchema = fieldSchema['items'] as Map<String, dynamic>;
    itemsField = _buildField('item', itemSchema, false);
  }

  // Parse x-source for remote data binding
  FieldDataSource? dataSource;
  final xSource = fieldSchema['x-source'];
  if (xSource is Map<String, dynamic>) {
    dataSource = FieldDataSource.fromJson(xSource);
  }

  // Parse x-file-types for file/directory picker fields
  List<String>? fileTypes;
  final xFileTypes = fieldSchema['x-file-types'];
  if (xFileTypes is List) {
    fileTypes = xFileTypes.cast<String>();
  }

  return NbFormField(
    name: fieldName,
    type: type,
    label: fieldSchema['title'] as String? ?? humanizeFieldName(fieldName),
    required: isRequired,
    description: fieldSchema['description'] as String?,
    defaultValue: fieldSchema['default'],
    enumValues: fieldSchema['enum'] as List<dynamic>?,
    format: format,
    pattern: fieldSchema['pattern'] as String?,
    minLength: fieldSchema['minLength'] as int?,
    maxLength: fieldSchema['maxLength'] as int?,
    minimum: fieldSchema['minimum'] as num?,
    maximum: fieldSchema['maximum'] as num?,
    multipleOf: fieldSchema['multipleOf'] as num?,
    properties: nestedProperties,
    items: itemsField,
    dataSource: dataSource,
    fileTypes: fileTypes,
  );
}

/// Resolve $ref references within a schema.
///
/// Ported from FormGenerator._resolve_ref in form_generator.py.
Map<String, dynamic> _resolveRef(
  Map<String, dynamic> schema,
  Map<String, dynamic> rootSchema, {
  Set<String>? visited,
}) {
  if (!schema.containsKey('\$ref')) return schema;

  visited ??= {};
  final refPath = schema['\$ref'] as String;

  // Prevent circular references
  if (visited.contains(refPath)) {
    return {'type': 'object', 'description': 'Circular reference: $refPath'};
  }
  visited.add(refPath);

  if (!refPath.startsWith('#/')) return schema;

  // Navigate the ref path
  final parts = refPath.substring(2).split('/');
  dynamic resolved = rootSchema;
  for (final part in parts) {
    if (resolved is Map<String, dynamic> && resolved.containsKey(part)) {
      resolved = resolved[part];
    } else {
      return {'type': 'object', 'description': 'Unresolved: $refPath'};
    }
  }

  if (resolved is! Map<String, dynamic>) return schema;

  // Merge resolved with any non-$ref properties
  final merged = Map<String, dynamic>.from(resolved);
  for (final entry in schema.entries) {
    if (entry.key != '\$ref') merged[entry.key] = entry.value;
  }

  return merged;
}

/// Convert field_name to "Field Name".
///
/// Ported from FormGenerator._humanize_field_name in form_generator.py.
String humanizeFieldName(String fieldName) {
  // Replace underscores and hyphens with spaces
  var name = fieldName.replaceAll('_', ' ').replaceAll('-', ' ');

  // Handle camelCase
  name = name.replaceAllMapped(
    RegExp(r'([a-z])([A-Z])'),
    (m) => '${m.group(1)} ${m.group(2)}',
  );

  // Title case
  return name
      .split(' ')
      .map((word) =>
          word.isEmpty ? '' : '${word[0].toUpperCase()}${word.substring(1)}')
      .join(' ');
}
