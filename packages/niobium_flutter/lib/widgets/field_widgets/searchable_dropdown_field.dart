// Searchable dropdown for medium option sets (≤ 200 items).
//
// Fetches all options at once, then filters client-side as user types.
// Uses a custom overlay approach for proper width-matching.

import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../utils/remote_data.dart';
import '../../utils/schema_parser.dart';

class NbSearchableDropdownField extends StatefulWidget {
  final NbFormField field;
  final dynamic value;
  final ValueChanged<dynamic> onChanged;

  const NbSearchableDropdownField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  State<NbSearchableDropdownField> createState() =>
      _NbSearchableDropdownFieldState();
}

class _NbSearchableDropdownFieldState extends State<NbSearchableDropdownField> {
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

  String? _displayLabel() {
    if (widget.value == null || _options == null) return null;
    final match = _options!.where((o) => o.value == widget.value);
    return match.isNotEmpty ? match.first.label : null;
  }

  void _openSearch() async {
    final result = await showDialog<RemoteOption>(
      context: context,
      builder: (ctx) => _InlineSearchDialog(
        label: widget.field.label,
        options: _options!,
        description: widget.field.description,
      ),
    );
    if (result != null) {
      widget.onChanged(result.value);
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

    final currentLabel = _displayLabel();

    return FormField<dynamic>(
      initialValue: widget.value,
      validator: (v) {
        if (widget.field.required && widget.value == null) {
          return '${widget.field.label} is required';
        }
        return null;
      },
      builder: (state) {
        return InkWell(
          onTap: _openSearch,
          borderRadius: BorderRadius.circular(4),
          child: InputDecorator(
            decoration: InputDecoration(
              labelText: label,
              hintText: widget.field.description ??
                  'Tap to search ${_options!.length} options...',
              errorText: state.errorText,
              suffixIcon: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (widget.value != null)
                    IconButton(
                      icon: const Icon(Icons.clear, size: 18),
                      onPressed: () {
                        widget.onChanged(null);
                        state.didChange(null);
                      },
                    ),
                  const Padding(
                    padding: EdgeInsets.only(right: 12),
                    child: Icon(Icons.search, size: 20),
                  ),
                ],
              ),
            ),
            child: Text(
              currentLabel != null ? humanizeFieldName(currentLabel) : '',
              style: currentLabel == null
                  ? TextStyle(color: Theme.of(context).hintColor)
                  : null,
            ),
          ),
        );
      },
    );
  }
}

/// Inline search dialog — renders as a centered card with a search field
/// and filtered list. Simpler and more reliable than Autocomplete overlay.
class _InlineSearchDialog extends StatefulWidget {
  final String label;
  final List<RemoteOption> options;
  final String? description;

  const _InlineSearchDialog({
    required this.label,
    required this.options,
    this.description,
  });

  @override
  State<_InlineSearchDialog> createState() => _InlineSearchDialogState();
}

class _InlineSearchDialogState extends State<_InlineSearchDialog> {
  final _controller = TextEditingController();
  List<RemoteOption> _filtered = [];

  @override
  void initState() {
    super.initState();
    _filtered = widget.options;
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _onFilter(String query) {
    setState(() {
      if (query.isEmpty) {
        _filtered = widget.options;
      } else {
        final q = query.toLowerCase();
        _filtered =
            widget.options.where((o) => o.label.toLowerCase().contains(q)).toList();
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      insetPadding: const EdgeInsets.all(24),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 500, maxHeight: 480),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                'Select ${widget.label}',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _controller,
                autofocus: true,
                decoration: InputDecoration(
                  hintText: 'Filter ${widget.options.length} options...',
                  prefixIcon: const Icon(Icons.search),
                  isDense: true,
                  suffixIcon: _controller.text.isNotEmpty
                      ? IconButton(
                          icon: const Icon(Icons.clear, size: 18),
                          onPressed: () {
                            _controller.clear();
                            _onFilter('');
                          },
                        )
                      : null,
                ),
                onChanged: _onFilter,
              ),
              const SizedBox(height: 8),
              Expanded(
                child: _filtered.isEmpty
                    ? const Center(
                        child: Padding(
                          padding: EdgeInsets.all(24),
                          child: Text('No matches',
                              style: TextStyle(color: Colors.grey)),
                        ),
                      )
                    : ListView.separated(
                        itemCount: _filtered.length,
                        separatorBuilder: (_, __) => const Divider(height: 1),
                        itemBuilder: (context, index) {
                          final opt = _filtered[index];
                          return ListTile(
                            title: Text(humanizeFieldName(opt.label)),
                            dense: true,
                            onTap: () => Navigator.of(context).pop(opt),
                          );
                        },
                      ),
              ),
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerRight,
                child: TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text('Cancel'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
