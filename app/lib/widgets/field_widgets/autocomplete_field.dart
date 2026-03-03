// Autocomplete field for large option sets (≤ 2000 items).
//
// Does NOT fetch all options upfront. Instead, sends search queries
// to the server via the search_param as the user types (debounced).
// Uses a dialog approach for reliable rendering.

import 'dart:async';
import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../utils/remote_data.dart';
import '../../utils/schema_parser.dart';

class NbAutocompleteField extends StatefulWidget {
  final NbFormField field;
  final dynamic value;
  final ValueChanged<dynamic> onChanged;

  const NbAutocompleteField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  State<NbAutocompleteField> createState() => _NbAutocompleteFieldState();
}

class _NbAutocompleteFieldState extends State<NbAutocompleteField> {
  String? _selectedLabel;

  void _openSearch() async {
    final result = await showDialog<RemoteOption>(
      context: context,
      builder: (ctx) => _ServerSearchDialog(
        field: widget.field,
        source: widget.field.dataSource!,
      ),
    );
    if (result != null) {
      setState(() {
        _selectedLabel = result.label;
      });
      widget.onChanged(result.value);
    }
  }

  @override
  Widget build(BuildContext context) {
    final label = widget.field.label + (widget.field.required ? ' *' : '');

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
                  'Tap to search...',
              errorText: state.errorText,
              suffixIcon: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (widget.value != null)
                    IconButton(
                      icon: const Icon(Icons.clear, size: 18),
                      onPressed: () {
                        setState(() { _selectedLabel = null; });
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
              _selectedLabel != null
                  ? humanizeFieldName(_selectedLabel!)
                  : '',
              style: _selectedLabel == null
                  ? TextStyle(color: Theme.of(context).hintColor)
                  : null,
            ),
          ),
        );
      },
    );
  }
}

/// Dialog with server-side search — types → debounce → fetch from endpoint.
class _ServerSearchDialog extends StatefulWidget {
  final NbFormField field;
  final FieldDataSource source;

  const _ServerSearchDialog({
    required this.field,
    required this.source,
  });

  @override
  State<_ServerSearchDialog> createState() => _ServerSearchDialogState();
}

class _ServerSearchDialogState extends State<_ServerSearchDialog> {
  final _controller = TextEditingController();
  List<RemoteOption> _results = [];
  bool _loading = false;
  String? _error;
  Timer? _debounce;

  @override
  void initState() {
    super.initState();
    // Load initial results
    _search('');
  }

  @override
  void dispose() {
    _controller.dispose();
    _debounce?.cancel();
    super.dispose();
  }

  void _onChanged(String query) {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 300), () {
      _search(query);
    });
  }

  Future<void> _search(String query) async {
    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      final result = await fetchRemoteOptions(
        widget.source,
        searchQuery: query.isNotEmpty ? query : null,
      );
      if (mounted) {
        setState(() {
          _results = result.options;
          _loading = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _error = e.toString();
          _loading = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      insetPadding: const EdgeInsets.all(24),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 500, maxHeight: 520),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                'Search ${widget.field.label}',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _controller,
                autofocus: true,
                decoration: InputDecoration(
                  hintText: 'Type to search...',
                  prefixIcon: const Icon(Icons.search),
                  isDense: true,
                  suffixIcon: _loading
                      ? const Padding(
                          padding: EdgeInsets.all(12),
                          child: SizedBox(
                            width: 18,
                            height: 18,
                            child:
                                CircularProgressIndicator(strokeWidth: 2),
                          ),
                        )
                      : _controller.text.isNotEmpty
                          ? IconButton(
                              icon: const Icon(Icons.clear, size: 18),
                              onPressed: () {
                                _controller.clear();
                                _search('');
                              },
                            )
                          : null,
                ),
                onChanged: _onChanged,
              ),
              const SizedBox(height: 8),
              Expanded(child: _buildResults()),
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

  Widget _buildResults() {
    if (_error != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.error_outline,
                color: Theme.of(context).colorScheme.error, size: 32),
            const SizedBox(height: 8),
            Text(_error!,
                style:
                    TextStyle(color: Theme.of(context).colorScheme.error)),
            const SizedBox(height: 8),
            TextButton.icon(
              onPressed: () => _search(_controller.text),
              icon: const Icon(Icons.refresh, size: 18),
              label: const Text('Retry'),
            ),
          ],
        ),
      );
    }

    if (_results.isEmpty && !_loading) {
      return const Center(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Text('No results found',
              style: TextStyle(color: Colors.grey)),
        ),
      );
    }

    return ListView.separated(
      itemCount: _results.length,
      separatorBuilder: (_, __) => const Divider(height: 1),
      itemBuilder: (context, index) {
        final opt = _results[index];
        return ListTile(
          title: Text(humanizeFieldName(opt.label)),
          dense: true,
          onTap: () => Navigator.of(context).pop(opt),
        );
      },
    );
  }
}
