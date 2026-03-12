import 'package:flutter/material.dart';
import '../../models/form_schema.dart';

/// Checkbox field for boolean types.
class NbBooleanField extends StatelessWidget {
  final NbFormField field;
  final bool value;
  final ValueChanged<bool?> onChanged;

  const NbBooleanField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return CheckboxListTile(
      title: Text(field.label),
      subtitle: field.description != null ? Text(field.description!) : null,
      value: value,
      onChanged: onChanged,
      controlAffinity: ListTileControlAffinity.leading,
      contentPadding: EdgeInsets.zero,
    );
  }
}
