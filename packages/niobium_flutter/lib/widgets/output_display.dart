// Rich output display widget — renders text, markdown, JSON, table, diff, or tabbed.
//
// Follows the ConfirmationDialog pattern: NbTitleBar + scrollable content + Close.
// Blocks until user dismisses.
//
// Tabbed mode: output_type "tabbed", content is JSON array of {tab, type, content}.

import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';
import '../theme/niobium_theme.dart';

class OutputDisplay extends StatefulWidget {
  final String content;
  final String outputType;
  final String title;
  final Completer<bool> completer;

  const OutputDisplay({
    super.key,
    required this.content,
    required this.outputType,
    required this.title,
    required this.completer,
  });

  // ── Content builders (shared by single + tabbed views) ─────────────

  static Widget buildContentWidget(BuildContext context, String type, String content) {
    final accent = Theme.of(context).colorScheme.primary;
    return switch (type) {
      'json' => _buildJson(content, accent),
      'table' => _buildTable(content, accent),
      'diff' => _buildDiff(content, accent),
      'markdown' => _buildMarkdown(content),
      _ => _buildText(content),
    };
  }

  static Widget _buildText(String content) {
    return SelectableText(
      content,
      style: const TextStyle(
        color: NbColors.textPrimary,
        fontSize: 13,
        fontFamily: 'monospace',
        height: 1.5,
      ),
    );
  }

  static Widget _buildMarkdown(String content) {
    return SelectableText(
      content,
      style: const TextStyle(
        color: NbColors.textPrimary,
        fontSize: 13,
        height: 1.6,
      ),
    );
  }

  static Widget _buildJson(String content, Color accent) {
    String pretty;
    try {
      final parsed = jsonDecode(content);
      pretty = const JsonEncoder.withIndent('  ').convert(parsed);
    } catch (_) {
      pretty = content;
    }

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(NbSpacing.md),
      decoration: BoxDecoration(
        color: NbColors.inputFill,
        borderRadius: BorderRadius.circular(NbRadius.sm),
        border: Border.all(color: NbColors.inputBorder),
      ),
      child: SelectableText(
        pretty,
        style: TextStyle(
          color: accent,
          fontSize: 12,
          fontFamily: 'monospace',
          height: 1.5,
        ),
      ),
    );
  }

  static Widget _buildTable(String content, Color accent) {
    try {
      final parsed = jsonDecode(content);
      final headers = (parsed['headers'] as List<dynamic>).cast<String>();
      final rows = (parsed['rows'] as List<dynamic>)
          .map((r) => (r as List<dynamic>))
          .toList();

      return LayoutBuilder(builder: (context, constraints) {
        return Container(
          decoration: BoxDecoration(
            border: Border.all(color: NbColors.inputBorder),
            borderRadius: BorderRadius.circular(NbRadius.sm),
          ),
          clipBehavior: Clip.antiAlias,
          child: SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: ConstrainedBox(
              constraints: BoxConstraints(minWidth: constraints.maxWidth),
              child: DataTable(
            headingRowColor: WidgetStatePropertyAll(NbColors.surfaceElevated),
            dataRowColor: WidgetStatePropertyAll(NbColors.surface),
            columns: headers
                .map((h) => DataColumn(
                      label: Text(h,
                          style: TextStyle(
                              fontWeight: FontWeight.w600,
                              color: accent)),
                    ))
                .toList(),
            rows: rows
                .map((row) => DataRow(
                      cells: row
                          .map((cell) => DataCell(
                                SelectableText(cell?.toString() ?? '',
                                    style: const TextStyle(
                                        color: NbColors.textPrimary,
                                        fontSize: 12)),
                              ))
                          .toList(),
                    ))
                .toList(),
            ),
          ),
        ),
      );
      });
    } catch (_) {
      return _buildText(content);
    }
  }

  static Widget _buildDiff(String content, Color accent) {
    final lines = content.split('\n');
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: lines.map((line) {
        Color color;
        Color? bgColor;
        if (line.startsWith('+++') || line.startsWith('---')) {
          color = NbColors.textSecondary;
          bgColor = null;
        } else if (line.startsWith('+')) {
          color = NbColors.success;
          bgColor = NbColors.success.withValues(alpha: 0.08);
        } else if (line.startsWith('-')) {
          color = NbColors.error;
          bgColor = NbColors.error.withValues(alpha: 0.08);
        } else if (line.startsWith('@@')) {
          color = accent;
          bgColor = accent.withValues(alpha: 0.08);
        } else {
          color = NbColors.textPrimary;
          bgColor = null;
        }

        return Container(
          width: double.infinity,
          color: bgColor,
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 1),
          child: SelectableText(
            line,
            style: TextStyle(
              color: color,
              fontSize: 12,
              fontFamily: 'monospace',
              height: 1.4,
            ),
          ),
        );
      }).toList(),
    );
  }

  @override
  State<OutputDisplay> createState() => _OutputDisplayState();
}

class _OutputDisplayState extends State<OutputDisplay>
    with TickerProviderStateMixin {
  TabController? _tabController;
  List<_TabDef>? _tabs;

  @override
  void initState() {
    super.initState();
    if (widget.outputType == 'tabbed') {
      _parseTabs();
    }
  }

  void _parseTabs() {
    try {
      final list = jsonDecode(widget.content) as List<dynamic>;
      _tabs = list.map((item) {
        final m = item as Map<String, dynamic>;
        return _TabDef(
          label: (m['tab'] as String?) ?? 'Tab',
          type: (m['type'] as String?) ?? 'text',
          content: (m['content'] as String?) ?? '',
        );
      }).toList();
      _tabController = TabController(length: _tabs!.length, vsync: this);
    } catch (_) {
      _tabs = null;
    }
  }

  @override
  void dispose() {
    _tabController?.dispose();
    super.dispose();
  }

  void _close() {
    if (!widget.completer.isCompleted) widget.completer.complete(true);
  }

  @override
  Widget build(BuildContext context) {
    final isTabbed = widget.outputType == 'tabbed' && _tabs != null;

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Column(
        children: [
          DragToMoveArea(
            child: NbTitleBar(
              title: widget.title,
              onClose: _close,
            ),
          ),
          if (isTabbed) _buildTabBar(),
          Expanded(
            child: isTabbed ? _buildTabbedBody() : _buildSingleBody(),
          ),
          // Close button
          Padding(
            padding: const EdgeInsets.fromLTRB(
                NbSpacing.lg, 0, NbSpacing.lg, NbSpacing.lg),
            child: SizedBox(
              width: double.infinity,
              child: OutlinedButton(
                onPressed: _close,
                style: OutlinedButton.styleFrom(
                  padding: const EdgeInsets.symmetric(vertical: 14),
                ),
                child: const Text('Close'),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSingleBody() {
    return Builder(builder: (context) => SingleChildScrollView(
      padding: const EdgeInsets.all(NbSpacing.lg),
      child: OutputDisplay.buildContentWidget(
          context, widget.outputType, widget.content),
    ));
  }

  Widget _buildTabBar() {
    final accent = Theme.of(context).colorScheme.primary;
    return Container(
      decoration: const BoxDecoration(
        border: Border(
          bottom: BorderSide(color: NbColors.glassBorder, width: 0.5),
        ),
      ),
      child: TabBar(
        controller: _tabController,
        isScrollable: true,
        tabAlignment: TabAlignment.start,
        labelColor: accent,
        unselectedLabelColor: NbColors.textSecondary,
        indicatorColor: accent,
        indicatorSize: TabBarIndicatorSize.label,
        labelStyle: const TextStyle(fontSize: 13, fontWeight: FontWeight.w500),
        unselectedLabelStyle:
            const TextStyle(fontSize: 13, fontWeight: FontWeight.w400),
        dividerHeight: 0,
        tabs: _tabs!.map((t) => Tab(text: t.label)).toList(),
      ),
    );
  }

  Widget _buildTabbedBody() {
    return Builder(builder: (context) => TabBarView(
      controller: _tabController,
      children: _tabs!
          .map((t) => SingleChildScrollView(
                padding: const EdgeInsets.all(NbSpacing.lg),
                child: OutputDisplay.buildContentWidget(context, t.type, t.content),
              ))
          .toList(),
    ));
  }
}

class _TabDef {
  final String label;
  final String type;
  final String content;
  const _TabDef({required this.label, required this.type, required this.content});
}
