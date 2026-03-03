import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../theme/niobium_theme.dart';

/// Slider field for numeric types (format: slider).
class NbSliderField extends StatefulWidget {
  final NbFormField field;
  final dynamic value;
  final ValueChanged<dynamic> onChanged;

  const NbSliderField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  State<NbSliderField> createState() => _NbSliderFieldState();
}

class _NbSliderFieldState extends State<NbSliderField> {
  late double _value;

  double get _min => (widget.field.minimum ?? 0).toDouble();
  double get _max => (widget.field.maximum ?? 100).toDouble();

  int? get _divisions {
    final step = widget.field.multipleOf;
    if (step == null || step == 0) return null;
    return ((_max - _min) / step.toDouble()).round();
  }

  @override
  void initState() {
    super.initState();
    _value = (widget.value as num?)?.toDouble() ?? _min;
  }

  @override
  Widget build(BuildContext context) {
    final isInteger = widget.field.type == 'integer';
    final displayValue = isInteger ? _value.round().toString() : _value.toStringAsFixed(1);

    return InputDecorator(
      decoration: InputDecoration(
        labelText: widget.field.label + (widget.field.required ? ' *' : ''),
        hintText: widget.field.description,
        border: InputBorder.none,
        enabledBorder: InputBorder.none,
        focusedBorder: InputBorder.none,
      ),
      child: Row(
        children: [
          Expanded(
            child: Slider(
              value: _value,
              min: _min,
              max: _max,
              divisions: _divisions,
              label: displayValue,
              onChanged: (v) {
                setState(() => _value = v);
                widget.onChanged(isInteger ? v.round() : v);
              },
            ),
          ),
          SizedBox(
            width: 48,
            child: Text(
              displayValue,
              textAlign: TextAlign.end,
              style: const TextStyle(
                color: NbColors.textPrimary,
                fontSize: 13,
                fontWeight: FontWeight.w500,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
