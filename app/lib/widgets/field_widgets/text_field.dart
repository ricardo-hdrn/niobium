import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../utils/validation.dart';

/// Text input field widget. Handles string type including formats:
/// email, uri/url, password, multiline (textarea).
class NbTextField extends StatelessWidget {
  final NbFormField field;
  final TextEditingController controller;
  final ValueChanged<String> onChanged;

  const NbTextField({
    super.key,
    required this.field,
    required this.controller,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final isMultiline =
        field.format == 'textarea' || (field.maxLength != null && field.maxLength! > 200);

    return TextFormField(
      controller: controller,
      maxLines: isMultiline ? 4 : 1,
      decoration: InputDecoration(
        labelText: field.label + (field.required ? ' *' : ''),
        hintText: field.description,
        // border inherited from theme
        suffixIcon: _formatIcon(),
      ),
      keyboardType: _keyboardType(),
      validator: (value) => validateField(field, value),
      onChanged: onChanged,
    );
  }

  TextInputType? _keyboardType() {
    return switch (field.format) {
      'email' => TextInputType.emailAddress,
      'uri' || 'url' => TextInputType.url,
      _ => null,
    };
  }

  Icon? _formatIcon() {
    return switch (field.format) {
      'email' => const Icon(Icons.email_outlined),
      'uri' || 'url' => const Icon(Icons.link),
      _ => null,
    };
  }
}
