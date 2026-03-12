// Page view widget — renders a layout tree of content and input nodes.
//
// Mixes markdown, text, dividers with form input fields inside sections.
// Returns collected input values on submit, or null on cancel.

import 'dart:async';
import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';
import '../models/form_schema.dart';
import '../models/display_config.dart';
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

/// A node in a page layout tree.
class PageNode {
  final String type;
  final String? content;
  final String? title;
  final String? key;
  final Map<String, dynamic>? field;
  final List<PageNode>? children;

  PageNode({
    required this.type,
    this.content,
    this.title,
    this.key,
    this.field,
    this.children,
  });

  factory PageNode.fromJson(Map<String, dynamic> json) {
    return PageNode(
      type: json['type'] as String,
      content: json['content'] as String?,
      title: json['title'] as String?,
      key: json['key'] as String?,
      field: json['field'] as Map<String, dynamic>?,
      children: (json['children'] as List<dynamic>?)
          ?.map((c) => PageNode.fromJson(c as Map<String, dynamic>))
          .toList(),
    );
  }

  bool get isInput => type == 'input' && key != null && field != null;
}

bool _hasInputNodes(List<PageNode> nodes) {
  for (final node in nodes) {
    if (node.isInput) return true;
    if (node.children != null && _hasInputNodes(node.children!)) return true;
  }
  return false;
}

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
    _hasInputs = _hasInputNodes(_nodes);
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

    if (field.enumValues != null && field.enumValues!.isNotEmpty) {
      return NbEnumField(
          field: field, value: _formData[field.name], onChanged: onChanged);
    }

    if (field.type == 'string' &&
        (field.format == 'date' ||
            field.format == 'date-time' ||
            field.format == 'time')) {
      return NbDateField(
          field: field, value: _formData[field.name], onChanged: onChanged);
    }

    if (field.type == 'string' &&
        (field.format == 'file' || field.format == 'directory')) {
      return NbFilePickerField(
          field: field, value: _formData[field.name], onChanged: onChanged);
    }

    if (field.type == 'string' && field.format == 'password') {
      return NbPasswordField(
        field: field,
        controller: _controllers[field.name]!,
        onChanged: (value) => _formData[field.name] = value,
      );
    }

    if (field.type == 'string' && field.format == 'color') {
      return NbColorPickerField(
          field: field, value: _formData[field.name], onChanged: onChanged);
    }

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

    if ((field.items?.enumValues != null &&
            field.items!.enumValues!.isNotEmpty) ||
        field.items?.dataSource != null) {
      return NbMultiSelectField(
        field: field,
        value: _formData[field.name],
        onChanged: (value) => setState(() => _formData[field.name] = value),
      );
    }

    return TextFormField(
      decoration: InputDecoration(
        labelText:
            '${field.label} (comma-separated)${field.required ? ' *' : ''}',
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
                  color: _hovering
                      ? NbColors.textPrimary
                      : NbColors.textTertiary),
              const SizedBox(width: 4),
              Text(
                widget.label,
                style: TextStyle(
                  fontSize: 12,
                  color:
                      _hovering ? NbColors.textPrimary : NbColors.textTertiary,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
