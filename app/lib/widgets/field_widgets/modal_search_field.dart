// Modal search field for very large option sets (> 2000 items).
//
// Shows a tappable field that opens a full-screen dialog with:
// - Search bar (server-side via search_param)
// - Filters declared in x-source.filters
// - Paginated results via page_param + page_size

import 'dart:async';
import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../utils/remote_data.dart';
import '../../theme/niobium_theme.dart';
import '../../utils/schema_parser.dart';

class NbModalSearchField extends StatefulWidget {
  final NbFormField field;
  final dynamic value;
  final ValueChanged<dynamic> onChanged;

  const NbModalSearchField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  State<NbModalSearchField> createState() => _NbModalSearchFieldState();
}

class _NbModalSearchFieldState extends State<NbModalSearchField> {
  String? _selectedLabel;

  @override
  Widget build(BuildContext context) {
    final label = widget.field.label + (widget.field.required ? ' *' : '');
    final source = widget.field.dataSource!;
    final hasFilters = source.filters != null && source.filters!.isNotEmpty;

    return FormField<dynamic>(
      initialValue: widget.value,
      validator: (v) {
        if (widget.field.required && v == null) {
          return '${widget.field.label} is required';
        }
        return null;
      },
      builder: (state) {
        return InkWell(
          onTap: () => _openSearchModal(context, state),
          borderRadius: BorderRadius.circular(4),
          child: InputDecorator(
            decoration: InputDecoration(
              labelText: label,
              hintText: widget.field.description ?? 'Tap to search...',
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
                  Padding(
                    padding: const EdgeInsets.only(right: 12),
                    child: Icon(
                      hasFilters ? Icons.filter_list : Icons.search,
                      size: 20,
                    ),
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

  Future<void> _openSearchModal(
      BuildContext context, FormFieldState state) async {
    final result = await Navigator.of(context).push<RemoteOption>(
      MaterialPageRoute(
        fullscreenDialog: true,
        builder: (ctx) => _SearchPage(
          field: widget.field,
          source: widget.field.dataSource!,
        ),
      ),
    );

    if (result != null) {
      setState(() { _selectedLabel = result.label; });
      widget.onChanged(result.value);
      state.didChange(result.value);
    }
  }
}

// ── Full-screen search page ─────────────────────────────────────────────

class _SearchPage extends StatefulWidget {
  final NbFormField field;
  final FieldDataSource source;

  const _SearchPage({required this.field, required this.source});

  @override
  State<_SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends State<_SearchPage> {
  final TextEditingController _searchController = TextEditingController();
  final ScrollController _scrollController = ScrollController();
  final Map<String, String> _filterValues = {};
  List<RemoteOption> _results = [];
  bool _loading = false;
  String? _error;
  int _page = 1;
  bool _hasMore = true;
  Timer? _debounce;
  bool _filtersExpanded = false;

  @override
  void initState() {
    super.initState();
    _scrollController.addListener(_onScroll);
    _search();
  }

  @override
  void dispose() {
    _searchController.dispose();
    _scrollController.dispose();
    _debounce?.cancel();
    super.dispose();
  }

  void _onScroll() {
    if (_scrollController.position.extentAfter < 200) {
      _loadMore();
    }
  }

  void _onSearchChanged(String query) {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 300), () {
      _page = 1;
      _results.clear();
      _search();
    });
  }

  void _onFilterChanged(String param, String value) {
    if (value.isEmpty) {
      _filterValues.remove(param);
    } else {
      _filterValues[param] = value;
    }
    _page = 1;
    _results.clear();
    _search();
  }

  Future<void> _search() async {
    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      final result = await fetchRemoteOptions(
        widget.source,
        searchQuery: _searchController.text.isNotEmpty
            ? _searchController.text
            : null,
        page: _page,
        filterValues: _filterValues.isNotEmpty ? _filterValues : null,
      );

      if (mounted) {
        setState(() {
          if (_page == 1) {
            _results = result.options;
          } else {
            _results.addAll(result.options);
          }
          _hasMore =
              result.options.length >= (widget.source.pageSize ?? 20);
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

  void _loadMore() {
    if (!_loading && _hasMore) {
      _page++;
      _search();
    }
  }

  @override
  Widget build(BuildContext context) {
    final filters = widget.source.filters ?? [];
    final theme = Theme.of(context);

    return Scaffold(
      backgroundColor: NbColors.bg,
      appBar: AppBar(
        title: Text('Select ${widget.field.label}'),
        leading: IconButton(
          icon: const Icon(Icons.close),
          onPressed: () => Navigator.of(context).pop(),
        ),
        actions: [
          if (filters.isNotEmpty)
            IconButton(
              icon: Badge(
                isLabelVisible: _filterValues.isNotEmpty,
                label: Text('${_filterValues.length}'),
                child: Icon(
                  _filtersExpanded
                      ? Icons.filter_list_off
                      : Icons.filter_list,
                ),
              ),
              onPressed: () {
                setState(() {
                  _filtersExpanded = !_filtersExpanded;
                });
              },
            ),
        ],
      ),
      body: Column(
        children: [
          // Search bar
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
            child: TextField(
              controller: _searchController,
              autofocus: true,
              decoration: InputDecoration(
                hintText: 'Search...',
                prefixIcon: const Icon(Icons.search),
                suffixIcon: _searchController.text.isNotEmpty
                    ? IconButton(
                        icon: const Icon(Icons.clear, size: 18),
                        onPressed: () {
                          _searchController.clear();
                          _onSearchChanged('');
                        },
                      )
                    : null,
              ),
              onChanged: _onSearchChanged,
            ),
          ),

          // Collapsible filters
          if (filters.isNotEmpty && _filtersExpanded)
            Container(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
              decoration: BoxDecoration(
                border: Border(
                  bottom: BorderSide(
                      color: theme.dividerColor, width: 0.5),
                ),
              ),
              child: Wrap(
                spacing: 12,
                runSpacing: 8,
                children: filters.map((f) => _buildFilter(f)).toList(),
              ),
            ),

          // Results count
          if (_results.isNotEmpty && !_loading)
            Padding(
              padding:
                  const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
              child: Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  '${_results.length} results${_hasMore ? '+' : ''}',
                  style: theme.textTheme.bodySmall
                      ?.copyWith(color: theme.hintColor),
                ),
              ),
            ),

          // Results list
          Expanded(child: _buildResults()),
        ],
      ),
    );
  }

  Widget _buildFilter(SourceFilter filter) {
    if (filter.type == 'enum' && filter.values != null) {
      return SizedBox(
        width: 200,
        child: DropdownButtonFormField<String>(
          decoration: InputDecoration(
            labelText: filter.label,
            isDense: true,
            contentPadding:
                const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          ),
          value: _filterValues[filter.param],
          items: [
            const DropdownMenuItem(value: '', child: Text('All')),
            ...filter.values!.map((v) => DropdownMenuItem(
                  value: v.toString(),
                  child: Text(humanizeFieldName(v.toString())),
                )),
          ],
          onChanged: (v) => _onFilterChanged(filter.param, v ?? ''),
          isExpanded: true,
        ),
      );
    }

    return SizedBox(
      width: 200,
      child: TextField(
        decoration: InputDecoration(
          labelText: filter.label,
          isDense: true,
          contentPadding:
              const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        ),
        onChanged: (v) => _onFilterChanged(filter.param, v),
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
                color: Theme.of(context).colorScheme.error, size: 48),
            const SizedBox(height: 12),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 32),
              child: Text(_error!,
                  textAlign: TextAlign.center,
                  style: TextStyle(
                      color: Theme.of(context).colorScheme.error)),
            ),
            const SizedBox(height: 16),
            FilledButton.icon(
              onPressed: _search,
              icon: const Icon(Icons.refresh),
              label: const Text('Retry'),
            ),
          ],
        ),
      );
    }

    if (_results.isEmpty && !_loading) {
      return const Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.search_off, size: 48, color: Colors.grey),
            SizedBox(height: 12),
            Text('No results found',
                style: TextStyle(color: Colors.grey, fontSize: 16)),
          ],
        ),
      );
    }

    return ListView.separated(
      controller: _scrollController,
      itemCount: _results.length + (_loading ? 1 : 0),
      separatorBuilder: (_, __) => const Divider(height: 1),
      itemBuilder: (context, index) {
        if (index >= _results.length) {
          return const Padding(
            padding: EdgeInsets.all(20),
            child: Center(child: CircularProgressIndicator()),
          );
        }

        final opt = _results[index];
        return ListTile(
          title: Text(humanizeFieldName(opt.label)),
          onTap: () => Navigator.of(context).pop(opt),
        );
      },
    );
  }
}
