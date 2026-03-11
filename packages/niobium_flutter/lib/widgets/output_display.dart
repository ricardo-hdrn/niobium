// Rich output display widget — renders text, markdown, JSON, table, diff, or tabbed.
//
// Follows the ConfirmationDialog pattern: NbTitleBar + scrollable content + Close.
// Blocks until user dismisses.
//
// Tabbed mode: output_type "tabbed", content is JSON array of {tab, type, content}.

import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
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
      'table' || 'datatable' => _buildTable(content, accent),
      'diff' => _buildDiff(content, accent),
      'markdown' => _buildMarkdown(content),
      'grid' => _buildGrid(content, accent),
      'decision' => _buildDecision(content, accent),
      'toast' => _buildToast(content, accent),
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
    return MarkdownBody(
      data: content,
      selectable: true,
      styleSheet: MarkdownStyleSheet(
        p: const TextStyle(color: NbColors.textPrimary, fontSize: 13, height: 1.6),
        h1: const TextStyle(color: NbColors.accent, fontSize: 22, fontWeight: FontWeight.w700),
        h2: const TextStyle(color: NbColors.accent, fontSize: 18, fontWeight: FontWeight.w600),
        h3: const TextStyle(color: NbColors.textPrimary, fontSize: 16, fontWeight: FontWeight.w600),
        code: const TextStyle(
          color: NbColors.accent,
          backgroundColor: NbColors.inputFill,
          fontSize: 12,
          fontFamily: 'monospace',
        ),
        codeblockDecoration: BoxDecoration(
          color: NbColors.inputFill,
          borderRadius: BorderRadius.circular(NbRadius.sm),
          border: Border.all(color: NbColors.inputBorder),
        ),
        codeblockPadding: const EdgeInsets.all(NbSpacing.md),
        blockquoteDecoration: const BoxDecoration(
          border: Border(left: BorderSide(color: NbColors.accent, width: 3)),
        ),
        listBullet: const TextStyle(color: NbColors.accent),
        strong: const TextStyle(color: NbColors.textPrimary, fontWeight: FontWeight.w600),
        em: const TextStyle(color: NbColors.textSecondary, fontStyle: FontStyle.italic),
        a: const TextStyle(color: NbColors.accent, decoration: TextDecoration.underline),
      ),
    );
  }

  static Widget _buildGrid(String content, Color accent) {
    try {
      final items = jsonDecode(content) as List<dynamic>;
      return _GridContent(items: items, accent: accent);
    } catch (_) {
      return _buildText(content);
    }
  }

  static Widget _buildDecision(String content, Color accent) {
    try {
      final parsed = jsonDecode(content) as Map<String, dynamic>;
      final options = (parsed['options'] as List<dynamic>?) ?? [];

      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: options.asMap().entries.map((entry) {
          final i = entry.key;
          final opt = entry.value as Map<String, dynamic>;
          return Container(
            margin: const EdgeInsets.only(bottom: NbSpacing.sm),
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: NbColors.surfaceElevated,
              borderRadius: BorderRadius.circular(NbRadius.sm),
              border: Border.all(color: accent.withValues(alpha: 0.3)),
            ),
            child: Row(
              children: [
                Container(
                  width: 32, height: 32,
                  decoration: BoxDecoration(
                    color: accent.withValues(alpha: 0.1),
                    shape: BoxShape.circle,
                  ),
                  child: Center(
                    child: Text('${i + 1}',
                        style: TextStyle(color: accent, fontWeight: FontWeight.w700)),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        '${opt['label'] ?? ''}',
                        style: const TextStyle(
                          color: NbColors.textPrimary,
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      if (opt['description'] != null) ...[
                        const SizedBox(height: 2),
                        Text(
                          '${opt['description']}',
                          style: const TextStyle(color: NbColors.textSecondary, fontSize: 12),
                        ),
                      ],
                    ],
                  ),
                ),
              ],
            ),
          );
        }).toList(),
      );
    } catch (_) {
      return _buildText(content);
    }
  }

  static Widget _buildToast(String content, Color accent) {
    String message;
    try {
      final parsed = jsonDecode(content) as Map<String, dynamic>;
      message = (parsed['message'] ?? content) as String;
    } catch (_) {
      message = content;
    }
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
      decoration: BoxDecoration(
        color: NbColors.surfaceElevated,
        borderRadius: BorderRadius.circular(NbRadius.sm),
        border: Border.all(color: accent.withValues(alpha: 0.4)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.check_circle, color: accent, size: 22),
          const SizedBox(width: 10),
          Flexible(
            child: Text(message,
                style: const TextStyle(color: NbColors.textPrimary, fontSize: 14)),
          ),
        ],
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

/// Grid cards with tap-to-expand for truncated content.
class _GridContent extends StatelessWidget {
  final List<dynamic> items;
  final Color accent;

  const _GridContent({required this.items, required this.accent});

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: NbSpacing.sm,
      runSpacing: NbSpacing.sm,
      children: items.map((item) {
        final m = item as Map<String, dynamic>;
        final title = '${m['title'] ?? ''}';
        final subtitle = m['subtitle'] as String?;
        final source = m['source'] as String?;
        final detail = m['detail'] as String? ?? m['description'] as String?;

        return GestureDetector(
          onTap: () => _showDetail(context, m),
          child: Container(
            width: 200,
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: NbColors.surfaceElevated,
              borderRadius: BorderRadius.circular(NbRadius.sm),
              border: Border.all(color: NbColors.glassBorder),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                if (source != null)
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                    margin: const EdgeInsets.only(bottom: 6),
                    decoration: BoxDecoration(
                      color: accent.withValues(alpha: 0.1),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      source,
                      style: TextStyle(color: accent, fontSize: 10),
                    ),
                  ),
                Text(
                  title,
                  style: const TextStyle(
                    color: NbColors.textPrimary,
                    fontSize: 13,
                    fontWeight: FontWeight.w500,
                    height: 1.3,
                  ),
                  maxLines: 3,
                  overflow: TextOverflow.ellipsis,
                ),
                if (subtitle != null) ...[
                  const SizedBox(height: 4),
                  Text(
                    subtitle,
                    style: const TextStyle(color: NbColors.textTertiary, fontSize: 11),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
                if (detail != null || (subtitle != null && subtitle.length > 60))
                  Padding(
                    padding: const EdgeInsets.only(top: 6),
                    child: Text(
                      'Tap to read more',
                      style: TextStyle(color: accent, fontSize: 10),
                    ),
                  ),
              ],
            ),
          ),
        );
      }).toList(),
    );
  }

  void _showDetail(BuildContext context, Map<String, dynamic> m) {
    final title = '${m['title'] ?? ''}';
    final subtitle = m['subtitle'] as String?;
    final source = m['source'] as String?;
    final detail = m['detail'] as String? ?? m['description'] as String?;

    showModalBottomSheet(
      context: context,
      backgroundColor: NbColors.surface,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) => DraggableScrollableSheet(
        initialChildSize: 0.6,
        minChildSize: 0.3,
        maxChildSize: 0.9,
        expand: false,
        builder: (ctx, scrollController) => SingleChildScrollView(
          controller: scrollController,
          padding: const EdgeInsets.all(20),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Drag handle.
              Center(
                child: Container(
                  width: 40, height: 4,
                  margin: const EdgeInsets.only(bottom: 16),
                  decoration: BoxDecoration(
                    color: NbColors.textTertiary,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
              ),
              if (source != null)
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                  margin: const EdgeInsets.only(bottom: 10),
                  decoration: BoxDecoration(
                    color: accent.withValues(alpha: 0.1),
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: Text(source, style: TextStyle(color: accent, fontSize: 11)),
                ),
              Text(
                title,
                style: const TextStyle(
                  color: NbColors.textPrimary,
                  fontSize: 18,
                  fontWeight: FontWeight.w600,
                  height: 1.4,
                ),
              ),
              if (subtitle != null) ...[
                const SizedBox(height: 10),
                Text(
                  subtitle,
                  style: const TextStyle(
                    color: NbColors.textSecondary,
                    fontSize: 14,
                    height: 1.5,
                  ),
                ),
              ],
              if (detail != null) ...[
                const SizedBox(height: 12),
                Text(
                  detail,
                  style: const TextStyle(
                    color: NbColors.textPrimary,
                    fontSize: 14,
                    height: 1.6,
                  ),
                ),
              ],
              // Show all other fields as extra info.
              ..._extraFields(m),
            ],
          ),
        ),
      ),
    );
  }

  List<Widget> _extraFields(Map<String, dynamic> m) {
    const skip = {'title', 'subtitle', 'source', 'detail', 'description'};
    final extras = m.entries.where((e) => !skip.contains(e.key) && e.value != null).toList();
    if (extras.isEmpty) return [];
    return [
      const SizedBox(height: 16),
      const Divider(color: NbColors.glassBorder),
      const SizedBox(height: 8),
      ...extras.map((e) => Padding(
        padding: const EdgeInsets.only(bottom: 6),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(
              width: 80,
              child: Text(
                e.key,
                style: TextStyle(color: accent, fontSize: 12, fontWeight: FontWeight.w500),
              ),
            ),
            Expanded(
              child: Text(
                '${e.value}',
                style: const TextStyle(color: NbColors.textPrimary, fontSize: 13),
              ),
            ),
          ],
        ),
      )),
    ];
  }
}

class _TabDef {
  final String label;
  final String type;
  final String content;
  const _TabDef({required this.label, required this.type, required this.content});
}
