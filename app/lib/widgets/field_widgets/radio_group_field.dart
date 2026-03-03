// Radio button group for small option sets (≤ 7 items).
//
// Selected by component intelligence when estimated_count is low.
// Fetches options from remote endpoint, then renders as radio buttons.

import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../utils/remote_data.dart';
import '../../utils/schema_parser.dart';

class NbRadioGroupField extends StatefulWidget {
  final NbFormField field;
  final dynamic value;
  final ValueChanged<dynamic> onChanged;

  const NbRadioGroupField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  State<NbRadioGroupField> createState() => _NbRadioGroupFieldState();
}

class _NbRadioGroupFieldState extends State<NbRadioGroupField> {
  List<RemoteOption>? _options;
  String? _error;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _fetchOptions();
  }

  Future<void> _fetchOptions() async {
    try {
      final result = await fetchRemoteOptions(widget.field.dataSource!);
      setState(() {
        _options = result.options;
        _loading = false;
      });
    } catch (e) {
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final label = widget.field.label + (widget.field.required ? ' *' : '');

    if (_loading) {
      return InputDecorator(
        decoration: InputDecoration(
          labelText: label,
        ),
        child: const Row(children: [
          SizedBox(
              width: 16,
              height: 16,
              child: CircularProgressIndicator(strokeWidth: 2)),
          SizedBox(width: 12),
          Text('Loading...', style: TextStyle(color: Colors.grey)),
        ]),
      );
    }

    if (_error != null) {
      return InputDecorator(
        decoration: InputDecoration(
          labelText: label,
          errorText: _error,
        ),
        child: Row(children: [
          const Icon(Icons.error_outline, color: Colors.red, size: 18),
          const SizedBox(width: 8),
          const Expanded(child: Text('Failed to load options')),
          IconButton(
            icon: const Icon(Icons.refresh, size: 18),
            onPressed: () {
              setState(() {
                _loading = true;
                _error = null;
              });
              _fetchOptions();
            },
          ),
        ]),
      );
    }

    return FormField<dynamic>(
      initialValue: widget.value,
      validator: (v) {
        if (widget.field.required && v == null) {
          return '${widget.field.label} is required';
        }
        return null;
      },
      builder: (state) {
        return InputDecorator(
          decoration: InputDecoration(
            labelText: label,
            hintText: widget.field.description,
            errorText: state.errorText,
          ),
          child: Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: _options!.map((opt) {
                final isSelected = widget.value == opt.value;
                return Material(
                  color: isSelected
                      ? Theme.of(context)
                          .colorScheme
                          .primaryContainer
                          .withValues(alpha: 0.3)
                      : Colors.transparent,
                  borderRadius: BorderRadius.circular(8),
                  child: InkWell(
                    borderRadius: BorderRadius.circular(8),
                    onTap: () {
                      widget.onChanged(opt.value);
                      state.didChange(opt.value);
                    },
                    child: Padding(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 4, vertical: 2),
                      child: Row(
                        children: [
                          Radio<dynamic>(
                            value: opt.value,
                            // ignore: deprecated_member_use
                            groupValue: widget.value,
                            // ignore: deprecated_member_use
                            onChanged: (v) {
                              widget.onChanged(v);
                              state.didChange(v);
                            },
                            materialTapTargetSize:
                                MaterialTapTargetSize.shrinkWrap,
                            visualDensity: VisualDensity.compact,
                          ),
                          const SizedBox(width: 4),
                          Expanded(
                            child: Text(
                              humanizeFieldName(opt.label),
                              style: Theme.of(context).textTheme.bodyMedium,
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                );
              }).toList(),
            ),
          ),
        );
      },
    );
  }
}
