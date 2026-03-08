import 'package:flutter/material.dart';
import '../../models/form_schema.dart';

/// Toggle switch field for boolean types (format: toggle).
class NbToggleField extends StatelessWidget {
  final NbFormField field;
  final bool value;
  final ValueChanged<bool?> onChanged;

  const NbToggleField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return SwitchListTile(
      title: Text(field.label),
      subtitle: field.description != null ? Text(field.description!) : null,
      value: value,
      onChanged: (v) => onChanged(v),
      contentPadding: EdgeInsets.zero,
    );
  }
}
