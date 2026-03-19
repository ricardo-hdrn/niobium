// Page view widget — renders a layout tree of content and input nodes.
//
// Mixes markdown, text, dividers with form input fields inside sections.
// Returns collected input values on submit, or null on cancel.

import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';
import '../models/form_schema.dart';
import '../models/display_config.dart';
import '../models/page_node.dart';
import '../theme/niobium_theme.dart';
import '../utils/schema_parser.dart';
import 'output_display.dart';
import 'field_widgets/text_field.dart';
import 'field_widgets/number_field.dart';
import 'field_widgets/boolean_field.dart';
import 'field_widgets/enum_field.dart';
import 'field_widgets/remote_enum_field.dart';
import 'field_widgets/radio_group_field.dart';
import 'field_widgets/searchable_dropdown_field.dart';
import 'field_widgets/autocomplete_field.dart';
import 'field_widgets/date_field.dart';
import 'field_widgets/multi_select_field.dart';
import 'field_widgets/modal_search_field.dart';
import 'field_widgets/file_picker_field.dart';
import 'field_widgets/password_field.dart';
import 'field_widgets/toggle_field.dart';
import 'field_widgets/slider_field.dart';
import 'field_widgets/color_picker_field.dart';
import 'field_widgets/data_grid_field.dart';

class NbPageView extends StatefulWidget {
  final List<dynamic> children;
  final String title;
  final Map<String, dynamic>? prefill;
  final Completer<Map<String, dynamic>?> completer;
  final NbDisplayConfig display;

  const NbPageView({
    super.key,
    required this.children,
    required this.title,
    this.prefill,
    required this.completer,
    this.display = NbDisplayConfig.defaultConfig,
  });

  @override
  State<NbPageView> createState() => _NbPageViewState();
}

class _NbPageViewState extends State<NbPageView> {
  final _formKey = GlobalKey<FormState>();
  final Map<String, dynamic> _formData = {};
  final Map<String, TextEditingController> _controllers = {};
  late final List<PageNode> _nodes;
  late final bool _hasInputs;

  @override
  void initState() {
    super.initState();
    _nodes = widget.children
        .map((c) => PageNode.fromJson(c as Map<String, dynamic>))
        .toList();
    _hasInputs = hasInputNodes(_nodes);
    _initInputs(_nodes);
  }

  void _initInputs(List<PageNode> nodes) {
    for (final node in nodes) {
      if (node.isInput) {
        _initField(node);
      }
      if (node.children != null) {
        _initInputs(node.children!);
      }
    }
  }

  void _initField(PageNode node) {
    final key = node.key!;
    final fieldDef = node.field!;
    final prefillValue = widget.prefill?[key];

    // Parse the field to get its type info
    final fields = parseSchema({
      'type': 'object',
      'properties': {key: fieldDef},
    });
    if (fields.isEmpty) return;
    final field = fields.first;

    final initialValue = prefillValue ?? field.defaultValue;

    if (field.type == 'boolean') {
      _formData[key] = initialValue ?? false;
    } else if (field.type == 'array') {
      if (initialValue is List) {
        _formData[key] = initialValue;
      } else if (field.items?.type == 'object') {
        _formData[key] = <Map<String, dynamic>>[];
      }
    } else if (field.type == 'string' &&
        (field.format == 'date' ||
            field.format == 'date-time' ||
            field.format == 'time' ||
            field.format == 'file' ||
            field.format == 'directory' ||
            field.format == 'color')) {
      if (initialValue != null) {
        _formData[key] = initialValue;
      }
    } else if ((field.type == 'number' || field.type == 'integer') &&
        field.format == 'slider') {
      if (initialValue != null) {
        _formData[key] = initialValue;
      }
    } else if (field.type == 'string' ||
        field.type == 'number' ||
        field.type == 'integer') {
      final text = initialValue?.toString() ?? '';
      _controllers[key] = TextEditingController(text: text);
      if (text.isNotEmpty) {
        _formData[key] = initialValue;
      }
    } else if (field.enumValues != null && initialValue != null) {
      _formData[key] = initialValue;
    }
  }

  @override
  void dispose() {
    for (final controller in _controllers.values) {
      controller.dispose();
    }
    if (!widget.completer.isCompleted) {
      widget.completer.complete(null);
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Column(
        children: [
          DragToMoveArea(
            child: NbTitleBar(
              title: widget.title,
              actions: [
                _TitleBarButton(
                  icon: Icons.close,
                  label: _hasInputs ? 'Cancel' : 'Close',
                  onTap: _handleCancel,
                ),
              ],
              onClose: _handleCancel,
            ),
          ),
          Expanded(
            child: SingleChildScrollView(
              padding: EdgeInsets.fromLTRB(
                widget.display.bodyPaddingH,
                widget.display.bodyPaddingV,
                widget.display.bodyPaddingH,
                widget.display.bodyPaddingH,
              ),
              child: Form(
                key: _formKey,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    ..._buildNodes(_nodes),
                    const SizedBox(height: NbSpacing.md),
                    if (_hasInputs)
                      FilledButton(
                        onPressed: _handleSubmit,
                        style: FilledButton.styleFrom(
                          padding: const EdgeInsets.symmetric(vertical: 16),
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(NbRadius.sm),
                          ),
                        ),
                        child: const Text('Submit'),
                      )
                    else
                      OutlinedButton(
                        onPressed: _handleDismiss,
                        style: OutlinedButton.styleFrom(
                          padding: const EdgeInsets.symmetric(vertical: 16),
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(NbRadius.sm),
                          ),
                        ),
                        child: const Text('Done'),
                      ),
                    const SizedBox(height: NbSpacing.sm),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  List<Widget> _buildNodes(List<PageNode> nodes) {
    return nodes.map(_buildNode).toList();
  }

  Widget _buildNode(PageNode node) {
    return switch (node.type) {
      'markdown' => Padding(
          padding: const EdgeInsets.only(bottom: NbSpacing.md),
          child: OutputDisplay.buildContentWidget(
              context, 'markdown', node.content ?? ''),
        ),
      'text' => Padding(
          padding: const EdgeInsets.only(bottom: NbSpacing.md),
          child: SelectableText(
            node.content ?? '',
            style: const TextStyle(
              color: NbColors.textPrimary,
              fontSize: 14,
              height: 1.6,
            ),
          ),
        ),
      'divider' => const Padding(
          padding: EdgeInsets.symmetric(vertical: NbSpacing.sm),
          child: Divider(color: NbColors.glassBorder),
        ),
      'spacer' => const SizedBox(height: NbSpacing.lg),
      'input' => _buildInputNode(node),
      'section' => _buildSection(node),
      // Leaf nodes
      'alert' => _buildAlert(node),
      'stat' => _buildStat(node),
      'badge' => _buildBadge(node),
      'progress' => _buildProgress(node),
      'blockquote' => _buildBlockquote(node),
      'image' => _buildImage(node),
      'table' => Padding(
          padding: const EdgeInsets.only(bottom: NbSpacing.md),
          child: OutputDisplay.buildContentWidget(
              context, 'table', node.content ?? '{}'),
        ),
      // Container nodes
      'row' => _buildRow(node),
      'card' => _buildCard(node),
      'collapse' => _NbCollapse(
          node: node,
          children:
              node.children != null ? _buildNodes(node.children!) : [],
        ),
      'tabs' => _buildTabs(node),
      'tab' || 'col' => Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: node.children != null ? _buildNodes(node.children!) : [],
        ),
      'hero' => _buildHero(node),
      _ => const SizedBox.shrink(),
    };
  }

  Widget _buildSection(PageNode node) {
    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.md),
      child: GlassPanel(
        child: Padding(
          padding: const EdgeInsets.all(NbSpacing.md),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (node.title != null) ...[
                Text(
                  node.title!,
                  style: const TextStyle(
                    color: NbColors.textPrimary,
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: NbSpacing.sm),
              ],
              ...node.children != null
                  ? _buildNodes(node.children!)
                  : [const SizedBox.shrink()],
            ],
          ),
        ),
      ),
    );
  }

  // ── Leaf node builders ──────────────────────────────────────────────────

  Widget _buildAlert(PageNode node) {
    final variant = node.props['variant'] as String? ?? 'info';
    final (color, icon) = switch (variant) {
      'warning' => (NbColors.warning, Icons.warning_amber_rounded),
      'error' => (NbColors.error, Icons.error_outline),
      'success' => (NbColors.success, Icons.check_circle_outline),
      'tip' => (const Color(0xFFA78BFA), Icons.lightbulb_outline),
      _ => (NbColors.accent, Icons.info_outline), // info
    };
    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.md),
      child: Container(
        decoration: BoxDecoration(
          color: color.withValues(alpha: 0.08),
          borderRadius: BorderRadius.circular(NbRadius.sm),
          border: Border(left: BorderSide(color: color, width: 3)),
        ),
        padding: const EdgeInsets.all(NbSpacing.md),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(children: [
              Icon(icon, color: color, size: 18),
              const SizedBox(width: 8),
              if (node.title != null)
                Text(node.title!,
                    style: TextStyle(
                        color: color,
                        fontWeight: FontWeight.w600,
                        fontSize: 14))
              else
                Text(variant[0].toUpperCase() + variant.substring(1),
                    style: TextStyle(
                        color: color,
                        fontWeight: FontWeight.w600,
                        fontSize: 14)),
            ]),
            if (node.content != null) ...[
              const SizedBox(height: 8),
              OutputDisplay.buildContentWidget(
                  context, 'markdown', node.content!),
            ],
            if (node.children != null) ...[
              const SizedBox(height: 8),
              ..._buildNodes(node.children!),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildStat(PageNode node) {
    final label = node.props['label']?.toString() ?? '';
    final value = node.props['value']?.toString() ?? '';
    final detail = node.props['detail'] as String?;
    final variant = node.props['variant'] as String?;
    final valueColor = switch (variant) {
      'success' => NbColors.success,
      'warning' => NbColors.warning,
      'error' => NbColors.error,
      _ => NbColors.textPrimary,
    };
    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.md),
      child: GlassPanel(
        child: Padding(
          padding: const EdgeInsets.all(NbSpacing.md),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(label,
                  style: const TextStyle(
                      color: NbColors.textSecondary,
                      fontSize: 12,
                      fontWeight: FontWeight.w500)),
              const SizedBox(height: 4),
              Text(value,
                  style: TextStyle(
                      color: valueColor,
                      fontSize: 28,
                      fontWeight: FontWeight.w700)),
              if (detail != null) ...[
                const SizedBox(height: 2),
                Text(detail,
                    style: const TextStyle(
                        color: NbColors.textTertiary, fontSize: 12)),
              ],
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildBadge(PageNode node) {
    final variant = node.props['variant'] as String? ?? 'default';
    final color = switch (variant) {
      'success' => NbColors.success,
      'warning' => NbColors.warning,
      'error' => NbColors.error,
      'info' => NbColors.accent,
      _ => NbColors.textSecondary,
    };
    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.sm),
      child: Align(
        alignment: Alignment.centerLeft,
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
          decoration: BoxDecoration(
            color: color.withValues(alpha: 0.15),
            borderRadius: BorderRadius.circular(NbRadius.sm),
          ),
          child: Text(
            node.content ?? '',
            style: TextStyle(
                color: color, fontSize: 12, fontWeight: FontWeight.w500),
          ),
        ),
      ),
    );
  }

  Widget _buildProgress(PageNode node) {
    final value = (node.props['value'] as num?)?.toDouble();
    final label = node.props['label'] as String?;
    final detail = node.props['detail'] as String?;
    final variant = node.props['variant'] as String?;
    final color = switch (variant) {
      'success' => NbColors.success,
      'warning' => NbColors.warning,
      'error' => NbColors.error,
      _ => Theme.of(context).colorScheme.primary,
    };
    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.md),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (label != null)
            Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: Text(label,
                  style: const TextStyle(
                      color: NbColors.textSecondary,
                      fontSize: 12,
                      fontWeight: FontWeight.w500)),
            ),
          ClipRRect(
            borderRadius: BorderRadius.circular(4),
            child: value != null
                ? LinearProgressIndicator(
                    value: value,
                    backgroundColor: NbColors.surfaceElevated,
                    color: color,
                    minHeight: 6)
                : LinearProgressIndicator(
                    backgroundColor: NbColors.surfaceElevated,
                    color: color,
                    minHeight: 6),
          ),
          if (detail != null)
            Padding(
              padding: const EdgeInsets.only(top: 4),
              child: Align(
                alignment: Alignment.centerRight,
                child: Text(detail,
                    style: const TextStyle(
                        color: NbColors.textTertiary, fontSize: 11)),
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildBlockquote(PageNode node) {
    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.md),
      child: Container(
        decoration: const BoxDecoration(
          border: Border(
              left: BorderSide(color: NbColors.textTertiary, width: 2)),
        ),
        padding: const EdgeInsets.only(left: 16, top: 4, bottom: 4),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              node.content ?? '',
              style: const TextStyle(
                  color: NbColors.textSecondary,
                  fontSize: 13,
                  fontStyle: FontStyle.italic,
                  height: 1.6),
            ),
            if (node.title != null) ...[
              const SizedBox(height: 4),
              Align(
                alignment: Alignment.centerRight,
                child: Text(node.title!,
                    style: const TextStyle(
                        color: NbColors.textTertiary, fontSize: 12)),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildImage(PageNode node) {
    final url = node.content ?? '';
    final alt = node.props['alt'] as String? ?? '';
    final maxHeight = (node.props['height'] as num?)?.toDouble();
    final maxWidth = (node.props['width'] as num?)?.toDouble();

    Widget image;
    if (url.startsWith('data:')) {
      // Base64 data URI
      final parts = url.split(',');
      if (parts.length == 2) {
        try {
          final bytes = base64Decode(parts[1]);
          image = Image.memory(bytes, fit: BoxFit.contain);
        } catch (_) {
          image = Text(alt.isEmpty ? 'Invalid image' : alt,
              style: const TextStyle(color: NbColors.textSecondary));
        }
      } else {
        image = Text(alt,
            style: const TextStyle(color: NbColors.textSecondary));
      }
    } else {
      image = Image.network(
        url,
        fit: BoxFit.contain,
        errorBuilder: (_, __, ___) => Text(
            alt.isEmpty ? 'Failed to load image' : alt,
            style: const TextStyle(color: NbColors.textSecondary)),
      );
    }

    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.md),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(NbRadius.md),
        child: ConstrainedBox(
          constraints: BoxConstraints(
            maxHeight: maxHeight ?? double.infinity,
            maxWidth: maxWidth ?? double.infinity,
          ),
          child: image,
        ),
      ),
    );
  }

  // ── Container node builders ─────────────────────────────────────────────

  Widget _buildRow(PageNode node) {
    if (node.children == null || node.children!.isEmpty) {
      return const SizedBox.shrink();
    }
    final gap = (node.props['gap'] as num?)?.toDouble() ?? 16;
    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.md),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          for (var i = 0; i < node.children!.length; i++) ...[
            if (i > 0) SizedBox(width: gap),
            Expanded(
              flex: (node.children![i].props['flex'] as int?) ?? 1,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                // Unwrap col nodes — render their children directly
                children: node.children![i].type == 'col' && node.children![i].children != null
                    ? _buildNodes(node.children![i].children!)
                    : [_buildNode(node.children![i])],
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildCard(PageNode node) {
    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.md),
      child: GlassPanel(
        elevated: true,
        child: Padding(
          padding: const EdgeInsets.all(NbSpacing.md),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (node.title != null) ...[
                Text(node.title!,
                    style: const TextStyle(
                        color: NbColors.textPrimary,
                        fontSize: 14,
                        fontWeight: FontWeight.w500)),
                const SizedBox(height: NbSpacing.sm),
              ],
              if (node.content != null)
                OutputDisplay.buildContentWidget(
                    context, 'markdown', node.content!),
              if (node.children != null) ..._buildNodes(node.children!),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildTabs(PageNode node) {
    final tabs = node.children ?? [];
    if (tabs.isEmpty) return const SizedBox.shrink();
    final tabHeight = (node.props['height'] as num?)?.toDouble() ?? 300;
    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.md),
      child: DefaultTabController(
        length: tabs.length,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TabBar(
              isScrollable: true,
              tabAlignment: TabAlignment.start,
              labelColor: Theme.of(context).colorScheme.primary,
              unselectedLabelColor: NbColors.textSecondary,
              indicatorColor: Theme.of(context).colorScheme.primary,
              dividerColor: NbColors.glassBorder,
              tabs: tabs.map((t) => Tab(text: t.title ?? 'Tab')).toList(),
            ),
            SizedBox(
              height: tabHeight,
              child: TabBarView(
                children: tabs
                    .map((tab) => SingleChildScrollView(
                          padding: const EdgeInsets.all(NbSpacing.md),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.stretch,
                            children: tab.children != null
                                ? _buildNodes(tab.children!)
                                : [],
                          ),
                        ))
                    .toList(),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildHero(PageNode node) {
    final accent = Theme.of(context).colorScheme.primary;
    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.lg),
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.symmetric(
            vertical: NbSpacing.xl, horizontal: NbSpacing.lg),
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: [accent.withValues(alpha: 0.08), Colors.transparent],
          ),
          borderRadius: BorderRadius.circular(NbRadius.md),
          border: Border.all(color: accent.withValues(alpha: 0.15)),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (node.title != null)
              Text(node.title!,
                  style: const TextStyle(
                      color: NbColors.textPrimary,
                      fontSize: 24,
                      fontWeight: FontWeight.w700)),
            if (node.content != null) ...[
              const SizedBox(height: 8),
              Text(node.content!,
                  style: const TextStyle(
                      color: NbColors.textSecondary,
                      fontSize: 14,
                      height: 1.5)),
            ],
            if (node.children != null) ...[
              const SizedBox(height: NbSpacing.md),
              ..._buildNodes(node.children!),
            ],
          ],
        ),
      ),
    );
  }

  // ── Input node builder ──────────────────────────────────────────────────

  Widget _buildInputNode(PageNode node) {
    if (node.key == null || node.field == null) return const SizedBox.shrink();

    final key = node.key!;
    final fields = parseSchema({
      'type': 'object',
      'properties': {key: node.field},
    });
    if (fields.isEmpty) return const SizedBox.shrink();
    final field = fields.first;

    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.sm),
      child: _buildFieldWidget(field),
    );
  }

  Widget _buildFieldWidget(NbFormField field) {
    void onChanged(dynamic value) =>
        setState(() => _formData[field.name] = value);

    // Remote data source
    if (field.dataSource != null) {
      final component = field.dataSource!.resolvedComponent;
      return switch (component) {
        SelectComponent.radio => NbRadioGroupField(
            field: field, value: _formData[field.name], onChanged: onChanged),
        SelectComponent.dropdown => NbRemoteEnumField(
            field: field, value: _formData[field.name], onChanged: onChanged),
        SelectComponent.searchableDropdown => NbSearchableDropdownField(
            field: field, value: _formData[field.name], onChanged: onChanged),
        SelectComponent.autocomplete => NbAutocompleteField(
            field: field, value: _formData[field.name], onChanged: onChanged),
        SelectComponent.modalSearch => NbModalSearchField(
            field: field, value: _formData[field.name], onChanged: onChanged),
      };
    }

    // Static enum
    if (field.enumValues != null && field.enumValues!.isNotEmpty) {
      return NbEnumField(
        field: field, value: _formData[field.name], onChanged: onChanged);
    }

    // Date/time
    if (field.type == 'string' &&
        (field.format == 'date' ||
            field.format == 'date-time' ||
            field.format == 'time')) {
      return NbDateField(
        field: field, value: _formData[field.name], onChanged: onChanged);
    }

    // File/directory
    if (field.type == 'string' &&
        (field.format == 'file' || field.format == 'directory')) {
      return NbFilePickerField(
        field: field, value: _formData[field.name], onChanged: onChanged);
    }

    // Password
    if (field.type == 'string' && field.format == 'password') {
      return NbPasswordField(
        field: field,
        controller: _controllers[field.name]!,
        onChanged: (value) => _formData[field.name] = value,
      );
    }

    // Color
    if (field.type == 'string' && field.format == 'color') {
      return NbColorPickerField(
        field: field, value: _formData[field.name], onChanged: onChanged);
    }

    // Slider
    if ((field.type == 'number' || field.type == 'integer') &&
        field.format == 'slider') {
      return NbSliderField(
        field: field, value: _formData[field.name], onChanged: onChanged);
    }

    return switch (field.type) {
      'string' => NbTextField(
          field: field,
          controller: _controllers[field.name]!,
          onChanged: (value) => _formData[field.name] = value,
        ),
      'number' || 'integer' => NbNumberField(
          field: field,
          controller: _controllers[field.name]!,
          onChanged: (value) {
            if (value.isNotEmpty) {
              _formData[field.name] = field.type == 'integer'
                  ? int.tryParse(value)
                  : num.tryParse(value);
            } else {
              _formData.remove(field.name);
            }
          },
        ),
      'boolean' => field.format == 'toggle'
          ? NbToggleField(
              field: field,
              value: _formData[field.name] as bool? ?? false,
              onChanged: (value) =>
                  setState(() => _formData[field.name] = value ?? false),
            )
          : NbBooleanField(
              field: field,
              value: _formData[field.name] as bool? ?? false,
              onChanged: (value) =>
                  setState(() => _formData[field.name] = value ?? false),
            ),
      'array' => _buildArrayField(field),
      _ => NbTextField(
          field: field,
          controller: _controllers.putIfAbsent(
              field.name, () => TextEditingController()),
          onChanged: (value) => _formData[field.name] = value,
        ),
    };
  }

  Widget _buildArrayField(NbFormField field) {
    if (field.items?.type == 'object' &&
        field.items?.properties != null &&
        field.items!.properties!.isNotEmpty) {
      return NbDataGridField(
        field: field,
        value: _formData[field.name],
        onChanged: (value) => setState(() => _formData[field.name] = value),
      );
    }

    if ((field.items?.enumValues != null && field.items!.enumValues!.isNotEmpty) ||
        field.items?.dataSource != null) {
      return NbMultiSelectField(
        field: field,
        value: _formData[field.name],
        onChanged: (value) => setState(() => _formData[field.name] = value),
      );
    }

    return TextFormField(
      decoration: InputDecoration(
        labelText: '${field.label} (comma-separated)${field.required ? ' *' : ''}',
        hintText: field.description,
      ),
      onChanged: (value) {
        if (value.isNotEmpty) {
          _formData[field.name] =
              value.split(',').map((e) => e.trim()).toList();
        } else {
          _formData.remove(field.name);
        }
      },
    );
  }

  void _handleSubmit() {
    if (_formKey.currentState!.validate()) {
      final result = Map<String, dynamic>.from(_formData)
        ..removeWhere((_, v) => v == null || (v is String && v.isEmpty));
      if (!widget.completer.isCompleted) {
        widget.completer.complete(result);
      }
    }
  }

  void _handleDismiss() {
    if (!widget.completer.isCompleted) {
      widget.completer.complete({'dismissed': true});
    }
  }

  void _handleCancel() {
    if (!widget.completer.isCompleted) {
      widget.completer.complete(null);
    }
  }
}

// ── Stateful node widgets ───────────────────────────────────────────────

class _NbCollapse extends StatefulWidget {
  final PageNode node;
  final List<Widget> children;
  const _NbCollapse({required this.node, required this.children});
  @override
  State<_NbCollapse> createState() => _NbCollapseState();
}

class _NbCollapseState extends State<_NbCollapse> {
  late bool _expanded;
  @override
  void initState() {
    super.initState();
    _expanded = widget.node.props['expanded'] == true;
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.md),
      child: GlassPanel(
        child: Column(
          children: [
            InkWell(
              onTap: () => setState(() => _expanded = !_expanded),
              child: Padding(
                padding: const EdgeInsets.all(NbSpacing.md),
                child: Row(
                  children: [
                    Icon(
                      _expanded ? Icons.expand_less : Icons.expand_more,
                      color: NbColors.textSecondary,
                      size: 20,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        widget.node.title ?? 'Details',
                        style: const TextStyle(
                            color: NbColors.textPrimary,
                            fontSize: 14,
                            fontWeight: FontWeight.w500),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            if (_expanded)
              Padding(
                padding: const EdgeInsets.fromLTRB(
                    NbSpacing.md, 0, NbSpacing.md, NbSpacing.md),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: widget.children,
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _TitleBarButton extends StatefulWidget {
  final IconData icon;
  final String label;
  final VoidCallback onTap;

  const _TitleBarButton({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  @override
  State<_TitleBarButton> createState() => _TitleBarButtonState();
}

class _TitleBarButtonState extends State<_TitleBarButton> {
  bool _hovering = false;

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      onEnter: (_) => setState(() => _hovering = true),
      onExit: (_) => setState(() => _hovering = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
          decoration: BoxDecoration(
            color: _hovering ? NbColors.glassHover : Colors.transparent,
            borderRadius: BorderRadius.circular(NbRadius.sm),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(widget.icon,
                  size: 14,
                  color:
                      _hovering ? NbColors.textPrimary : NbColors.textTertiary),
              const SizedBox(width: 4),
              Text(
                widget.label,
                style: TextStyle(
                  fontSize: 12,
                  color: _hovering
                      ? NbColors.textPrimary
                      : NbColors.textTertiary,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
