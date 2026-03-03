// File and directory picker field.
//
// Triggered by format: "file" or "directory" on string-type fields.
// Uses file_selector for native OS file dialogs (GTK on Linux).

import 'package:flutter/material.dart';
import 'package:file_selector/file_selector.dart';
import '../../models/form_schema.dart';
import '../../theme/niobium_theme.dart';

class NbFilePickerField extends StatefulWidget {
  final NbFormField field;
  final dynamic value;
  final ValueChanged<dynamic> onChanged;

  const NbFilePickerField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  State<NbFilePickerField> createState() => _NbFilePickerFieldState();
}

class _NbFilePickerFieldState extends State<NbFilePickerField> {
  String? _selectedPath;

  bool get _isDirectory => widget.field.format == 'directory';

  @override
  void initState() {
    super.initState();
    if (widget.value is String && (widget.value as String).isNotEmpty) {
      _selectedPath = widget.value as String;
    }
  }

  Future<void> _pick() async {
    if (_isDirectory) {
      final path = await getDirectoryPath(
        confirmButtonText: 'Select',
      );
      if (path != null) {
        setState(() => _selectedPath = path);
        widget.onChanged(path);
      }
    } else {
      // Build file type filters from x-file-types
      final typeGroups = <XTypeGroup>[];
      if (widget.field.fileTypes != null && widget.field.fileTypes!.isNotEmpty) {
        typeGroups.add(XTypeGroup(
          label: 'Allowed files',
          extensions: widget.field.fileTypes,
        ));
      }
      // Always add an "All files" option
      typeGroups.add(const XTypeGroup(label: 'All files'));

      final file = await openFile(
        acceptedTypeGroups: typeGroups,
        confirmButtonText: 'Select',
      );
      if (file != null) {
        setState(() => _selectedPath = file.path);
        widget.onChanged(file.path);
      }
    }
  }

  void _clear() {
    setState(() => _selectedPath = null);
    widget.onChanged(null);
  }

  @override
  Widget build(BuildContext context) {
    final label = widget.field.label + (widget.field.required ? ' *' : '');
    final hasValue = _selectedPath != null;

    return FormField<String>(
      initialValue: _selectedPath,
      validator: (_) {
        if (widget.field.required && !hasValue) {
          return '${widget.field.label} is required';
        }
        return null;
      },
      builder: (state) {
        return InkWell(
          onTap: _pick,
          borderRadius: BorderRadius.circular(NbRadius.sm),
          child: InputDecorator(
            decoration: InputDecoration(
              labelText: label,
              hintText: widget.field.description ??
                  (_isDirectory ? 'Select directory' : 'Select file'),
              errorText: state.errorText,
              suffixIcon: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (hasValue)
                    IconButton(
                      icon: const Icon(Icons.clear, size: 18),
                      onPressed: () {
                        _clear();
                        state.didChange(null);
                      },
                    ),
                  Padding(
                    padding: const EdgeInsets.only(right: 12),
                    child: Icon(
                      _isDirectory ? Icons.folder_open : Icons.attach_file,
                      size: 20,
                    ),
                  ),
                ],
              ),
            ),
            child: Text(
              hasValue ? _selectedPath! : '',
              style: hasValue
                  ? Theme.of(context).textTheme.bodyMedium
                  : Theme.of(context)
                      .textTheme
                      .bodyMedium
                      ?.copyWith(color: NbColors.textTertiary),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        );
      },
    );
  }
}
