// Dynamic form widget that renders JSON Schema as native Flutter fields.

import 'dart:async';
import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';
import '../models/form_schema.dart';
import '../theme/niobium_theme.dart';
import '../models/display_config.dart';
import '../utils/schema_parser.dart';
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

class DynamicForm extends StatefulWidget {
  final Map<String, dynamic> schema;
  final String title;
  final Map<String, dynamic>? prefill;
  final Completer<Map<String, dynamic>?> completer;
  final NbDisplayConfig display;

  const DynamicForm({
    super.key,
    required this.schema,
    required this.title,
    this.prefill,
    required this.completer,
    this.display = NbDisplayConfig.defaultConfig,
  });

  @override
  State<DynamicForm> createState() => _DynamicFormState();
}

class _DynamicFormState extends State<DynamicForm>
    with TickerProviderStateMixin {
  final _formKey = GlobalKey<FormState>();
  final Map<String, dynamic> _formData = {};
  final Map<String, TextEditingController> _controllers = {};
  late final List<NbFormField> _fields;
  late final AnimationController _staggerController;

  @override
  void initState() {
    super.initState();
    _fields = parseSchema(widget.schema);

    // Stagger animation: total duration scales with field count
    final totalMs = 300 + (_fields.length * 80).clamp(0, 600);
    _staggerController = AnimationController(
      duration: Duration(milliseconds: widget.display.animate ? totalMs : 0),
      vsync: this,
    );
    _staggerController.forward();

    for (final field in _fields) {
      final prefillValue = widget.prefill?[field.name];
      final initialValue = prefillValue ?? field.defaultValue;

      if (field.type == 'boolean') {
        _formData[field.name] = initialValue ?? false;
      } else if (field.type == 'array') {
        // Data grid (object items) or multi-select — store list directly
        if (initialValue is List) {
          _formData[field.name] = initialValue;
        } else if (field.items?.type == 'object') {
          _formData[field.name] = <Map<String, dynamic>>[];
        }
      } else if (field.type == 'string' &&
          (field.format == 'date' ||
              field.format == 'date-time' ||
              field.format == 'time' ||
              field.format == 'file' ||
              field.format == 'directory' ||
              field.format == 'color')) {
        // These fields manage their own state — just store the value
        if (initialValue != null) {
          _formData[field.name] = initialValue;
        }
      } else if ((field.type == 'number' || field.type == 'integer') &&
          field.format == 'slider') {
        // Slider manages its own state — store numeric value directly
        if (initialValue != null) {
          _formData[field.name] = initialValue;
        }
      } else if (field.type == 'string' ||
          field.type == 'number' ||
          field.type == 'integer') {
        final text = initialValue?.toString() ?? '';
        _controllers[field.name] = TextEditingController(text: text);
        if (text.isNotEmpty) {
          _formData[field.name] = initialValue;
        }
      } else if (field.enumValues != null && initialValue != null) {
        _formData[field.name] = initialValue;
      }
    }
  }

  @override
  void dispose() {
    _staggerController.dispose();
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
          // ── Custom title bar ──
          DragToMoveArea(
            child: NbTitleBar(
              title: widget.title,
              actions: [
                _WindowButton(
                  icon: Icons.close,
                  label: 'Cancel',
                  onTap: _handleCancel,
                ),
              ],
              onClose: _handleCancel,
            ),
          ),

          // ── Form body ──
          Expanded(
            child: SingleChildScrollView(
              padding: EdgeInsets.fromLTRB(
                  widget.display.bodyPaddingH, widget.display.bodyPaddingV,
                  widget.display.bodyPaddingH, widget.display.bodyPaddingH),
              child: Form(
                key: _formKey,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    // Fields with stagger animation
                    ..._fields.asMap().entries.map((entry) {
                      final i = entry.key;
                      final field = entry.value;
                      final fieldWidget = Padding(
                        padding: EdgeInsets.only(bottom: widget.display.fieldSpacing),
                        child: _buildFieldWithLabel(field),
                      );

                      if (!widget.display.animate) return fieldWidget;

                      final start = _fields.length <= 1
                          ? 0.0
                          : (i / _fields.length * 0.6).clamp(0.0, 0.6);
                      final end = (start + 0.5).clamp(0.0, 1.0);
                      final animation = CurvedAnimation(
                        parent: _staggerController,
                        curve: Interval(start, end, curve: Curves.easeOutCubic),
                      );
                      return FadeTransition(
                        opacity: animation,
                        child: SlideTransition(
                          position: Tween<Offset>(
                            begin: const Offset(0, 0.15),
                            end: Offset.zero,
                          ).animate(animation),
                          child: fieldWidget,
                        ),
                      );
                    }),

                    const SizedBox(height: NbSpacing.sm),

                    // Submit button
                    FilledButton(
                      onPressed: _handleSubmit,
                      style: FilledButton.styleFrom(
                        padding: const EdgeInsets.symmetric(vertical: 16),
                        shape: RoundedRectangleBorder(
                          borderRadius:
                              BorderRadius.circular(NbRadius.sm),
                        ),
                      ),
                      child: const Text('Submit'),
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

  /// Wraps a field widget with its description shown below.
  Widget _buildFieldWithLabel(NbFormField field) {
    final widget = _buildField(field);
    if (field.description == null || field.description!.isEmpty) {
      return widget;
    }
    // Description is already handled by InputDecoration.hintText
    // in most widgets, so just return the widget directly.
    return widget;
  }

  Widget _buildField(NbFormField field) {
    // Remote data source fields — Niobium picks the best widget
    if (field.dataSource != null) {
      final component = field.dataSource!.resolvedComponent;
      void onChanged(dynamic value) =>
          setState(() => _formData[field.name] = value);
      return switch (component) {
        SelectComponent.radio => NbRadioGroupField(
            field: field,
            value: _formData[field.name],
            onChanged: onChanged),
        SelectComponent.dropdown => NbRemoteEnumField(
            field: field,
            value: _formData[field.name],
            onChanged: onChanged),
        SelectComponent.searchableDropdown => NbSearchableDropdownField(
            field: field,
            value: _formData[field.name],
            onChanged: onChanged),
        SelectComponent.autocomplete => NbAutocompleteField(
            field: field,
            value: _formData[field.name],
            onChanged: onChanged),
        SelectComponent.modalSearch => NbModalSearchField(
            field: field,
            value: _formData[field.name],
            onChanged: onChanged),
      };
    }

    // Static enum fields
    if (field.enumValues != null && field.enumValues!.isNotEmpty) {
      return NbEnumField(
        field: field,
        value: _formData[field.name],
        onChanged: (value) => setState(() => _formData[field.name] = value),
      );
    }

    // Date/time formats get a specialized picker
    if (field.type == 'string' &&
        (field.format == 'date' ||
            field.format == 'date-time' ||
            field.format == 'time')) {
      return NbDateField(
        field: field,
        value: _formData[field.name],
        onChanged: (value) => setState(() => _formData[field.name] = value),
      );
    }

    // File/directory picker
    if (field.type == 'string' &&
        (field.format == 'file' || field.format == 'directory')) {
      return NbFilePickerField(
        field: field,
        value: _formData[field.name],
        onChanged: (value) => setState(() => _formData[field.name] = value),
      );
    }

    // Password field with visibility toggle
    if (field.type == 'string' && field.format == 'password') {
      return NbPasswordField(
        field: field,
        controller: _controllers[field.name]!,
        onChanged: (value) => _formData[field.name] = value,
      );
    }

    // Color picker
    if (field.type == 'string' && field.format == 'color') {
      return NbColorPickerField(
        field: field,
        value: _formData[field.name],
        onChanged: (value) => setState(() => _formData[field.name] = value),
      );
    }

    // Slider
    if ((field.type == 'number' || field.type == 'integer') &&
        field.format == 'slider') {
      return NbSliderField(
        field: field,
        value: _formData[field.name],
        onChanged: (value) => setState(() => _formData[field.name] = value),
      );
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
    // Data grid: items are objects with defined properties
    if (field.items?.type == 'object' &&
        field.items?.properties != null &&
        field.items!.properties!.isNotEmpty) {
      return NbDataGridField(
        field: field,
        value: _formData[field.name],
        onChanged: (value) => setState(() => _formData[field.name] = value),
      );
    }

    // Multi-select: items have enum values or a remote data source
    if ((field.items?.enumValues != null && field.items!.enumValues!.isNotEmpty) ||
        field.items?.dataSource != null) {
      return NbMultiSelectField(
        field: field,
        value: _formData[field.name],
        onChanged: (value) => setState(() => _formData[field.name] = value),
      );
    }

    // Fallback: plain comma-separated text input for simple string arrays
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

  void _handleCancel() {
    if (!widget.completer.isCompleted) {
      widget.completer.complete(null);
    }
  }
}

/// Small text button for the title bar actions.
class _WindowButton extends StatefulWidget {
  final IconData icon;
  final String label;
  final VoidCallback onTap;

  const _WindowButton({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  @override
  State<_WindowButton> createState() => _WindowButtonState();
}

class _WindowButtonState extends State<_WindowButton> {
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
