// Multi-select field for array types with enum or remote options.
//
// Static enum arrays (items.enum): checkbox group for ≤ 7, dialog for more.
// Remote arrays (items.x-source): fetches options, then shows checkboxes or dialog.
// Value is always stored as List<dynamic>.

import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../theme/niobium_theme.dart';
import '../../utils/remote_data.dart';
import '../../utils/schema_parser.dart';

class NbMultiSelectField extends StatefulWidget {
  final NbFormField field;
  final dynamic value;
  final ValueChanged<dynamic> onChanged;

  const NbMultiSelectField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  State<NbMultiSelectField> createState() => _NbMultiSelectFieldState();
}

class _NbMultiSelectFieldState extends State<NbMultiSelectField> {
  List<_SelectOption> _options = [];
  bool _loading = false;
  String? _error;

  List<dynamic> get _selected {
    final v = widget.value;
    if (v is List) return List<dynamic>.from(v);
    return [];
  }

  bool get _isRemote => widget.field.items?.dataSource != null;
  bool get _useInlineCheckboxes => _options.length <= 7;

  @override
  void initState() {
    super.initState();
    _loadOptions();
  }

  void _loadOptions() {
    if (_isRemote) {
      _fetchRemoteOptions();
    } else {
      // Static enum from items schema
      final enumValues = widget.field.items?.enumValues ?? [];
      _options = enumValues
          .map((v) => _SelectOption(
              label: humanizeFieldName(v.toString()), value: v))
          .toList();
    }
  }

  Future<void> _fetchRemoteOptions() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final result =
          await fetchRemoteOptions(widget.field.items!.dataSource!);
      setState(() {
        _options = result.options
            .map((o) => _SelectOption(label: o.label, value: o.value))
            .toList();
        _loading = false;
      });
    } catch (e) {
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  void _toggleValue(dynamic value) {
    final current = List<dynamic>.from(_selected);
    if (current.contains(value)) {
      current.remove(value);
    } else {
      current.add(value);
    }
    widget.onChanged(current);
  }

  @override
  Widget build(BuildContext context) {
    final label = widget.field.label + (widget.field.required ? ' *' : '');

    if (_loading) {
      return InputDecorator(
        decoration: InputDecoration(labelText: label),
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
        decoration: InputDecoration(labelText: label, errorText: _error),
        child: Row(children: [
          const Icon(Icons.error_outline, color: Colors.red, size: 18),
          const SizedBox(width: 8),
          const Expanded(child: Text('Failed to load options')),
          IconButton(
            icon: const Icon(Icons.refresh, size: 18),
            onPressed: _fetchRemoteOptions,
          ),
        ]),
      );
    }

    if (_useInlineCheckboxes) {
      return _buildCheckboxGroup(label);
    }
    return _buildDialogPicker(label);
  }

  Widget _buildCheckboxGroup(String label) {
    return FormField<List<dynamic>>(
      initialValue: _selected,
      validator: (v) {
        if (widget.field.required && (v == null || v.isEmpty)) {
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
              children: _options.map((opt) {
                final isSelected = _selected.contains(opt.value);
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
                      _toggleValue(opt.value);
                      state.didChange(_selected);
                    },
                    child: Padding(
                      padding: const EdgeInsets.symmetric(
                          horizontal: 4, vertical: 2),
                      child: Row(
                        children: [
                          Checkbox(
                            value: isSelected,
                            onChanged: (_) {
                              _toggleValue(opt.value);
                              state.didChange(_selected);
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

  Widget _buildDialogPicker(String label) {
    final selectedLabels = _options
        .where((o) => _selected.contains(o.value))
        .map((o) => o.label)
        .toList();

    return FormField<List<dynamic>>(
      initialValue: _selected,
      validator: (v) {
        if (widget.field.required && (v == null || v.isEmpty)) {
          return '${widget.field.label} is required';
        }
        return null;
      },
      builder: (state) {
        return InkWell(
          onTap: () => _showSelectionDialog(state),
          borderRadius: BorderRadius.circular(NbRadius.sm),
          child: InputDecorator(
            decoration: InputDecoration(
              labelText: label,
              hintText: widget.field.description ?? 'Select items',
              errorText: state.errorText,
              suffixIcon: const Padding(
                padding: EdgeInsets.only(right: 12),
                child: Icon(Icons.checklist, size: 20),
              ),
            ),
            child: selectedLabels.isEmpty
                ? const SizedBox.shrink()
                : Wrap(
                    spacing: 6,
                    runSpacing: 4,
                    children: selectedLabels.map((label) {
                      return Chip(
                        label: Text(label, style: const TextStyle(fontSize: 12)),
                        deleteIcon: const Icon(Icons.close, size: 14),
                        onDeleted: () {
                          final opt =
                              _options.firstWhere((o) => o.label == label);
                          _toggleValue(opt.value);
                          state.didChange(_selected);
                        },
                        materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                        visualDensity: VisualDensity.compact,
                        side: const BorderSide(color: NbColors.glassBorder),
                        backgroundColor: NbColors.surfaceElevated,
                      );
                    }).toList(),
                  ),
          ),
        );
      },
    );
  }

  Future<void> _showSelectionDialog(FormFieldState<List<dynamic>> state) async {
    final result = await showDialog<List<dynamic>>(
      context: context,
      builder: (ctx) => _MultiSelectDialog(
        title: widget.field.label,
        options: _options,
        selected: _selected,
      ),
    );

    if (result != null) {
      widget.onChanged(result);
      state.didChange(result);
    }
  }
}

class _SelectOption {
  final String label;
  final dynamic value;
  _SelectOption({required this.label, required this.value});
}

/// Dialog that shows all options with checkboxes.
class _MultiSelectDialog extends StatefulWidget {
  final String title;
  final List<_SelectOption> options;
  final List<dynamic> selected;

  const _MultiSelectDialog({
    required this.title,
    required this.options,
    required this.selected,
  });

  @override
  State<_MultiSelectDialog> createState() => _MultiSelectDialogState();
}

class _MultiSelectDialogState extends State<_MultiSelectDialog> {
  late List<dynamic> _selected;
  String _filter = '';

  @override
  void initState() {
    super.initState();
    _selected = List<dynamic>.from(widget.selected);
  }

  List<_SelectOption> get _filteredOptions {
    if (_filter.isEmpty) return widget.options;
    final lower = _filter.toLowerCase();
    return widget.options
        .where((o) => o.label.toLowerCase().contains(lower))
        .toList();
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 420, maxHeight: 500),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 16, 20, 0),
              child: Row(
                children: [
                  Text(widget.title,
                      style: Theme.of(context).textTheme.titleMedium),
                  const Spacer(),
                  Text('${_selected.length} selected',
                      style: Theme.of(context).textTheme.bodySmall),
                ],
              ),
            ),
            if (widget.options.length > 10)
              Padding(
                padding: const EdgeInsets.fromLTRB(20, 12, 20, 0),
                child: TextField(
                  decoration: const InputDecoration(
                    hintText: 'Filter...',
                    prefixIcon: Icon(Icons.search, size: 18),
                    isDense: true,
                    contentPadding:
                        EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                  ),
                  onChanged: (v) => setState(() => _filter = v),
                ),
              ),
            const SizedBox(height: 8),
            Flexible(
              child: ListView.builder(
                shrinkWrap: true,
                itemCount: _filteredOptions.length,
                itemBuilder: (ctx, i) {
                  final opt = _filteredOptions[i];
                  final isSelected = _selected.contains(opt.value);
                  return CheckboxListTile(
                    title: Text(humanizeFieldName(opt.label)),
                    value: isSelected,
                    dense: true,
                    onChanged: (_) {
                      setState(() {
                        if (isSelected) {
                          _selected.remove(opt.value);
                        } else {
                          _selected.add(opt.value);
                        }
                      });
                    },
                  );
                },
              ),
            ),
            const Divider(),
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 8, 20, 12),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(
                    onPressed: () => Navigator.pop(context),
                    child: const Text('Cancel'),
                  ),
                  const SizedBox(width: 8),
                  FilledButton(
                    onPressed: () => Navigator.pop(context, _selected),
                    child: const Text('Done'),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
