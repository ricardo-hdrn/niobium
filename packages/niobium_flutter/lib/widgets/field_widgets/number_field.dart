import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../../models/form_schema.dart';
import '../../utils/validation.dart';

/// Numeric input field widget. Handles number and integer types.
class NbNumberField extends StatelessWidget {
  final NbFormField field;
  final TextEditingController controller;
  final ValueChanged<String> onChanged;

  const NbNumberField({
    super.key,
    required this.field,
    required this.controller,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    return TextFormField(
      controller: controller,
      decoration: InputDecoration(
        labelText: field.label + (field.required ? ' *' : ''),
        hintText: field.description ?? _rangeHint(),
        // border inherited from theme
      ),
      keyboardType: TextInputType.number,
      inputFormatters: [
        if (field.type == 'integer')
          FilteringTextInputFormatter.digitsOnly
        else
          FilteringTextInputFormatter.allow(RegExp(r'^-?\d*\.?\d*')),
      ],
      validator: (value) => validateField(field, value),
      onChanged: onChanged,
    );
  }

  String? _rangeHint() {
    if (field.minimum != null && field.maximum != null) {
      return '${field.minimum} – ${field.maximum}';
    }
    if (field.minimum != null) return 'Min: ${field.minimum}';
    if (field.maximum != null) return 'Max: ${field.maximum}';
    return null;
  }
}
