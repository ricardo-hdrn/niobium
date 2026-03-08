import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../theme/niobium_theme.dart';

/// Editable data grid for array of objects.
/// Trigger: type=array, items.type=object, items.properties defined.
class NbDataGridField extends StatefulWidget {
  final NbFormField field;
  final dynamic value;
  final ValueChanged<dynamic> onChanged;

  const NbDataGridField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  State<NbDataGridField> createState() => _NbDataGridFieldState();
}

class _NbDataGridFieldState extends State<NbDataGridField> {
  late List<Map<String, dynamic>> _rows;
  late Map<String, NbFormField> _columns;

  // Controllers keyed by "rowIndex:columnName"
  final Map<String, TextEditingController> _cellControllers = {};

  @override
  void initState() {
    super.initState();
    _columns = widget.field.items?.properties ?? {};

    final initial = widget.value;
    if (initial is List) {
      _rows = initial.map((e) => Map<String, dynamic>.from(e as Map)).toList();
    } else {
      _rows = [];
    }

    _rebuildControllers();
  }

  @override
  void dispose() {
    for (final c in _cellControllers.values) {
      c.dispose();
    }
    super.dispose();
  }

  void _rebuildControllers() {
    // Dispose old controllers not in use
    final needed = <String>{};
    for (var i = 0; i < _rows.length; i++) {
      for (final col in _columns.entries) {
        if (col.value.type == 'string' ||
            col.value.type == 'number' ||
            col.value.type == 'integer') {
          needed.add('$i:${col.key}');
        }
      }
    }

    _cellControllers.removeWhere((key, controller) {
      if (!needed.contains(key)) {
        controller.dispose();
        return true;
      }
      return false;
    });

    for (var i = 0; i < _rows.length; i++) {
      for (final col in _columns.entries) {
        if (col.value.type == 'string' ||
            col.value.type == 'number' ||
            col.value.type == 'integer') {
          final key = '$i:${col.key}';
          if (!_cellControllers.containsKey(key)) {
            _cellControllers[key] = TextEditingController(
              text: _rows[i][col.key]?.toString() ?? '',
            );
          }
        }
      }
    }
  }

  Map<String, dynamic> _newRow() {
    final row = <String, dynamic>{};
    for (final col in _columns.entries) {
      row[col.key] = col.value.defaultValue ?? switch (col.value.type) {
        'boolean' => false,
        'number' || 'integer' => null,
        _ => '',
      };
    }
    return row;
  }

  void _addRow() {
    setState(() {
      _rows.add(_newRow());
      _rebuildControllers();
    });
    _emit();
  }

  void _removeRow(int index) {
    setState(() {
      _rows.removeAt(index);
      _rebuildControllers();
    });
    _emit();
  }

  void _emit() {
    widget.onChanged(List<Map<String, dynamic>>.from(_rows));
  }

  @override
  Widget build(BuildContext context) {
    return InputDecorator(
      decoration: InputDecoration(
        labelText: widget.field.label + (widget.field.required ? ' *' : ''),
        hintText: widget.field.description,
        border: InputBorder.none,
        enabledBorder: InputBorder.none,
        focusedBorder: InputBorder.none,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (_rows.isNotEmpty)
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: DataTable(
                columnSpacing: 16,
                headingRowHeight: 40,
                dataRowMinHeight: 40,
                dataRowMaxHeight: 48,
                columns: [
                  ..._columns.entries.map((col) => DataColumn(
                        label: Text(
                          col.value.label,
                          style: const TextStyle(
                            fontWeight: FontWeight.w600,
                            fontSize: 12,
                          ),
                        ),
                      )),
                  const DataColumn(label: SizedBox.shrink()), // delete button
                ],
                rows: List.generate(_rows.length, (i) => _buildRow(i)),
              ),
            ),

          const SizedBox(height: 8),

          Align(
            alignment: Alignment.centerLeft,
            child: TextButton.icon(
              onPressed: _addRow,
              icon: const Icon(Icons.add, size: 16),
              label: const Text('Add row'),
              style: TextButton.styleFrom(
                foregroundColor: NbColors.accent,
                textStyle: const TextStyle(fontSize: 12),
              ),
            ),
          ),
        ],
      ),
    );
  }

  DataRow _buildRow(int rowIndex) {
    return DataRow(
      cells: [
        ..._columns.entries.map((col) => _buildCell(rowIndex, col.key, col.value)),
        DataCell(
          IconButton(
            icon: const Icon(Icons.delete_outline, size: 16),
            color: NbColors.textTertiary,
            onPressed: () => _removeRow(rowIndex),
          ),
        ),
      ],
    );
  }

  DataCell _buildCell(int rowIndex, String colName, NbFormField colField) {
    if (colField.type == 'boolean') {
      return DataCell(
        Checkbox(
          value: _rows[rowIndex][colName] as bool? ?? false,
          onChanged: (v) {
            setState(() => _rows[rowIndex][colName] = v ?? false);
            _emit();
          },
        ),
      );
    }

    final key = '$rowIndex:$colName';
    final controller = _cellControllers[key]!;

    return DataCell(
      SizedBox(
        width: 120,
        child: TextField(
          controller: controller,
          style: const TextStyle(fontSize: 13),
          decoration: const InputDecoration(
            isDense: true,
            contentPadding: EdgeInsets.symmetric(horizontal: 8, vertical: 6),
            border: InputBorder.none,
          ),
          onChanged: (v) {
            if (colField.type == 'integer') {
              _rows[rowIndex][colName] = int.tryParse(v);
            } else if (colField.type == 'number') {
              _rows[rowIndex][colName] = num.tryParse(v);
            } else {
              _rows[rowIndex][colName] = v;
            }
            _emit();
          },
        ),
      ),
    );
  }
}
