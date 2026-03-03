// Simple dropdown field for remote options (≤ 30 items).
//
// Fetches all options from endpoint, renders as a standard dropdown.
// This is the "plain" variant — no search, no pagination.

import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../utils/remote_data.dart';
import '../../utils/schema_parser.dart';

/// A dropdown whose options are loaded from a remote URL at render time.
class NbRemoteEnumField extends StatefulWidget {
  final NbFormField field;
  final dynamic value;
  final ValueChanged<dynamic> onChanged;

  const NbRemoteEnumField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  State<NbRemoteEnumField> createState() => _NbRemoteEnumFieldState();
}

class _NbRemoteEnumFieldState extends State<NbRemoteEnumField> {
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
          hintText: widget.field.description,
        ),
        child: const Row(
          children: [
            SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2)),
            SizedBox(width: 12),
            Text('Loading options...', style: TextStyle(color: Colors.grey)),
          ],
        ),
      );
    }

    if (_error != null) {
      return InputDecorator(
        decoration: InputDecoration(
          labelText: label,
          hintText: widget.field.description,
          errorText: _error,
        ),
        child: Row(
          children: [
            const Icon(Icons.error_outline, color: Colors.red, size: 18),
            const SizedBox(width: 8),
            const Expanded(child: Text('Failed to load options')),
            IconButton(
              icon: const Icon(Icons.refresh, size: 18),
              onPressed: () {
                setState(() { _loading = true; _error = null; });
                _fetchOptions();
              },
            ),
          ],
        ),
      );
    }

    return DropdownButtonFormField<dynamic>(
      decoration: InputDecoration(
        labelText: label,
        hintText: widget.field.description ?? '${_options!.length} options loaded',
      ),
      value: widget.value,
      items: _options!.map((opt) {
        return DropdownMenuItem(
          value: opt.value,
          child: Text(humanizeFieldName(opt.label)),
        );
      }).toList(),
      validator: (v) {
        if (widget.field.required && v == null) {
          return '${widget.field.label} is required';
        }
        return null;
      },
      onChanged: (v) => widget.onChanged(v),
      isExpanded: true,
    );
  }
}
