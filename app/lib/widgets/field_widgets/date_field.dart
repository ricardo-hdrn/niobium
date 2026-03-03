// Date, DateTime, and Time picker fields.
//
// Triggered by format: "date", "date-time", or "time" on string-type fields.
// Uses Flutter's native showDatePicker/showTimePicker dialogs.

import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../theme/niobium_theme.dart';

class NbDateField extends StatefulWidget {
  final NbFormField field;
  final dynamic value;
  final ValueChanged<dynamic> onChanged;

  const NbDateField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  State<NbDateField> createState() => _NbDateFieldState();
}

class _NbDateFieldState extends State<NbDateField> {
  DateTime? _date;
  TimeOfDay? _time;

  bool get _isTimeOnly => widget.field.format == 'time';
  bool get _isDateTime => widget.field.format == 'date-time';

  @override
  void initState() {
    super.initState();
    _parseInitialValue();
  }

  void _parseInitialValue() {
    final v = widget.value;
    if (v == null || (v is String && v.isEmpty)) return;

    final str = v.toString();
    if (_isTimeOnly) {
      final parts = str.split(':');
      if (parts.length >= 2) {
        _time = TimeOfDay(
          hour: int.tryParse(parts[0]) ?? 0,
          minute: int.tryParse(parts[1]) ?? 0,
        );
      }
    } else {
      final parsed = DateTime.tryParse(str);
      if (parsed != null) {
        _date = parsed;
        if (_isDateTime) {
          _time = TimeOfDay(hour: parsed.hour, minute: parsed.minute);
        }
      }
    }
  }

  String _formatValue() {
    if (_isTimeOnly && _time != null) {
      return '${_time!.hour.toString().padLeft(2, '0')}:${_time!.minute.toString().padLeft(2, '0')}';
    }
    if (_date != null) {
      final dateStr =
          '${_date!.year.toString().padLeft(4, '0')}-${_date!.month.toString().padLeft(2, '0')}-${_date!.day.toString().padLeft(2, '0')}';
      if (_isDateTime && _time != null) {
        return '${dateStr}T${_time!.hour.toString().padLeft(2, '0')}:${_time!.minute.toString().padLeft(2, '0')}:00';
      }
      return dateStr;
    }
    return '';
  }

  String get _displayText {
    if (_isTimeOnly && _time != null) {
      return _time!.format(context);
    }
    if (_date != null) {
      final dateStr =
          '${_date!.day.toString().padLeft(2, '0')}/${_date!.month.toString().padLeft(2, '0')}/${_date!.year}';
      if (_isDateTime && _time != null) {
        return '$dateStr ${_time!.format(context)}';
      }
      return dateStr;
    }
    return '';
  }

  Future<void> _pickDate() async {
    final now = DateTime.now();
    final picked = await showDatePicker(
      context: context,
      initialDate: _date ?? now,
      firstDate: DateTime(1900),
      lastDate: DateTime(2100),
    );
    if (picked == null) return;

    setState(() => _date = picked);

    if (_isDateTime) {
      await _pickTime();
    } else {
      _emitValue();
    }
  }

  Future<void> _pickTime() async {
    final picked = await showTimePicker(
      context: context,
      initialTime: _time ?? TimeOfDay.now(),
    );
    if (picked == null) return;

    setState(() => _time = picked);
    _emitValue();
  }

  void _emitValue() {
    final formatted = _formatValue();
    if (formatted.isNotEmpty) {
      widget.onChanged(formatted);
    }
  }

  void _clear() {
    setState(() {
      _date = null;
      _time = null;
    });
    widget.onChanged(null);
  }

  @override
  Widget build(BuildContext context) {
    final label = widget.field.label + (widget.field.required ? ' *' : '');
    final hasValue =
        (_isTimeOnly && _time != null) || (!_isTimeOnly && _date != null);

    return FormField<String>(
      initialValue: widget.value?.toString(),
      validator: (v) {
        if (widget.field.required && !hasValue) {
          return '${widget.field.label} is required';
        }
        return null;
      },
      builder: (state) {
        return InkWell(
          onTap: _isTimeOnly ? _pickTime : _pickDate,
          borderRadius: BorderRadius.circular(NbRadius.sm),
          child: InputDecorator(
            decoration: InputDecoration(
              labelText: label,
              hintText: widget.field.description ?? _hintForFormat,
              errorText: state.errorText,
              suffixIcon: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (hasValue)
                    IconButton(
                      icon: const Icon(Icons.clear, size: 18),
                      onPressed: () {
                        _clear();
                        state.didChange(null);
                      },
                    ),
                  Padding(
                    padding: const EdgeInsets.only(right: 12),
                    child: Icon(_iconForFormat, size: 20),
                  ),
                ],
              ),
            ),
            child: Text(
              hasValue ? _displayText : '',
              style: hasValue
                  ? Theme.of(context).textTheme.bodyMedium
                  : Theme.of(context)
                      .textTheme
                      .bodyMedium
                      ?.copyWith(color: NbColors.textTertiary),
            ),
          ),
        );
      },
    );
  }

  IconData get _iconForFormat {
    if (_isTimeOnly) return Icons.access_time;
    if (_isDateTime) return Icons.event;
    return Icons.calendar_today;
  }

  String get _hintForFormat {
    if (_isTimeOnly) return 'Select time';
    if (_isDateTime) return 'Select date and time';
    return 'Select date';
  }
}
