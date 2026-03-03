import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../utils/schema_parser.dart';

/// Dropdown field for enum types.
class NbEnumField extends StatelessWidget {
  final NbFormField field;
  final dynamic value;
  final ValueChanged<dynamic> onChanged;

  const NbEnumField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return DropdownButtonFormField<dynamic>(
      decoration: InputDecoration(
        labelText: field.label + (field.required ? ' *' : ''),
        hintText: field.description,
      ),
      value: value,
      items: field.enumValues!.map((v) {
        return DropdownMenuItem(
          value: v,
          child: Text(humanizeFieldName(v.toString())),
        );
      }).toList(),
      validator: (v) {
        if (field.required && v == null) return '${field.label} is required';
        return null;
      },
      onChanged: (v) => onChanged(v),
    );
  }
}
