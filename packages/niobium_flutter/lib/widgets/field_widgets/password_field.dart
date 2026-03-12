import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../utils/validation.dart';

/// Password field with visibility toggle. Text is visible by default.
class NbPasswordField extends StatefulWidget {
  final NbFormField field;
  final TextEditingController controller;
  final ValueChanged<String> onChanged;

  const NbPasswordField({
    super.key,
    required this.field,
    required this.controller,
    required this.onChanged,
  });

  @override
  State<NbPasswordField> createState() => _NbPasswordFieldState();
}

class _NbPasswordFieldState extends State<NbPasswordField> {
  bool _obscured = false;

  @override
  Widget build(BuildContext context) {
    return TextFormField(
      controller: widget.controller,
      obscureText: _obscured,
      decoration: InputDecoration(
        labelText: widget.field.label + (widget.field.required ? ' *' : ''),
        hintText: widget.field.description,
        suffixIcon: IconButton(
          icon: Icon(_obscured ? Icons.visibility_off : Icons.visibility),
          onPressed: () => setState(() => _obscured = !_obscured),
        ),
      ),
      validator: (value) => validateField(widget.field, value),
      onChanged: widget.onChanged,
    );
  }
}
