// Client-side validation for form fields.
//
// Ported from intelligence/services/form_generator.py _extract_validation.

import '../models/form_schema.dart';

/// Validate a form field value. Returns null if valid, error message if not.
String? validateField(NbFormField field, dynamic value) {
  final strValue = value?.toString() ?? '';

  // Required check
  if (field.required && (value == null || strValue.isEmpty)) {
    return '${field.label} is required';
  }

  // Skip further validation for empty optional fields
  if (value == null || strValue.isEmpty) return null;

  // String validations
  if (field.type == 'string') {
    if (field.minLength != null && strValue.length < field.minLength!) {
      return 'Minimum length is ${field.minLength}';
    }
    if (field.maxLength != null && strValue.length > field.maxLength!) {
      return 'Maximum length is ${field.maxLength}';
    }
    if (field.pattern != null) {
      final regex = RegExp(field.pattern!);
      if (!regex.hasMatch(strValue)) {
        return 'Invalid format';
      }
    }
    // Format-specific validation
    if (field.format == 'email' && !_isValidEmail(strValue)) {
      return 'Invalid email address';
    }
    if ((field.format == 'uri' || field.format == 'url') &&
        !_isValidUrl(strValue)) {
      return 'Invalid URL';
    }
  }

  // Number validations
  if (field.type == 'number' || field.type == 'integer') {
    final num? parsed =
        field.type == 'integer' ? int.tryParse(strValue) : num.tryParse(strValue);
    if (parsed == null) {
      return 'Invalid number';
    }
    if (field.minimum != null && parsed < field.minimum!) {
      return 'Minimum value is ${field.minimum}';
    }
    if (field.maximum != null && parsed > field.maximum!) {
      return 'Maximum value is ${field.maximum}';
    }
  }

  return null;
}

bool _isValidEmail(String value) {
  return RegExp(r'^[^@\s]+@[^@\s]+\.[^@\s]+$').hasMatch(value);
}

bool _isValidUrl(String value) {
  return Uri.tryParse(value)?.hasScheme ?? false;
}
