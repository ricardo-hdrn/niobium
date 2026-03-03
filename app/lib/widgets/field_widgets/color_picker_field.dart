import 'package:flutter/material.dart';
import '../../models/form_schema.dart';
import '../../theme/niobium_theme.dart';

/// Color picker field (format: color). Returns hex string #RRGGBB.
class NbColorPickerField extends StatefulWidget {
  final NbFormField field;
  final dynamic value;
  final ValueChanged<dynamic> onChanged;

  const NbColorPickerField({
    super.key,
    required this.field,
    required this.value,
    required this.onChanged,
  });

  @override
  State<NbColorPickerField> createState() => _NbColorPickerFieldState();
}

class _NbColorPickerFieldState extends State<NbColorPickerField> {
  late Color _color;

  @override
  void initState() {
    super.initState();
    _color = _parseHex(widget.value as String?) ?? const Color(0xFF00D4AA);
  }

  Color? _parseHex(String? hex) {
    if (hex == null || hex.isEmpty) return null;
    final clean = hex.replaceFirst('#', '');
    if (clean.length != 6) return null;
    final value = int.tryParse(clean, radix: 16);
    if (value == null) return null;
    return Color(0xFF000000 | value);
  }

  String _toHex(Color c) =>
      '#${(c.r * 255).round().toRadixString(16).padLeft(2, '0')}'
      '${(c.g * 255).round().toRadixString(16).padLeft(2, '0')}'
      '${(c.b * 255).round().toRadixString(16).padLeft(2, '0')}';

  void _openPicker() async {
    final result = await showDialog<Color>(
      context: context,
      builder: (_) => _ColorPickerDialog(selected: _color),
    );
    if (result != null) {
      setState(() => _color = result);
      widget.onChanged(_toHex(result));
    }
  }

  @override
  Widget build(BuildContext context) {
    return InputDecorator(
      decoration: InputDecoration(
        labelText: widget.field.label + (widget.field.required ? ' *' : ''),
        hintText: widget.field.description,
      ),
      child: GestureDetector(
        onTap: _openPicker,
        child: Row(
          children: [
            Container(
              width: 32,
              height: 32,
              decoration: BoxDecoration(
                color: _color,
                borderRadius: BorderRadius.circular(NbRadius.sm),
                border: Border.all(color: NbColors.glassBorder),
              ),
            ),
            const SizedBox(width: 12),
            Text(
              _toHex(_color).toUpperCase(),
              style: const TextStyle(
                color: NbColors.textPrimary,
                fontSize: 13,
                fontFamily: 'monospace',
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ColorPickerDialog extends StatefulWidget {
  final Color selected;
  const _ColorPickerDialog({required this.selected});

  @override
  State<_ColorPickerDialog> createState() => _ColorPickerDialogState();
}

class _ColorPickerDialogState extends State<_ColorPickerDialog> {
  late Color _current;
  final _hexController = TextEditingController();

  static const _palette = <Color>[
    // Reds
    Color(0xFFEF5350), Color(0xFFF44336), Color(0xFFE53935), Color(0xFFC62828),
    // Pinks
    Color(0xFFEC407A), Color(0xFFE91E63), Color(0xFFD81B60), Color(0xFFAD1457),
    // Purples
    Color(0xFFAB47BC), Color(0xFF9C27B0), Color(0xFF8E24AA), Color(0xFF6A1B9A),
    // Blues
    Color(0xFF42A5F5), Color(0xFF2196F3), Color(0xFF1E88E5), Color(0xFF1565C0),
    // Cyans
    Color(0xFF26C6DA), Color(0xFF00BCD4), Color(0xFF00ACC1), Color(0xFF00838F),
    // Teals
    Color(0xFF26A69A), Color(0xFF009688), Color(0xFF00897B), Color(0xFF00695C),
    // Greens
    Color(0xFF66BB6A), Color(0xFF4CAF50), Color(0xFF43A047), Color(0xFF2E7D32),
    // Yellows/Oranges
    Color(0xFFFFEE58), Color(0xFFFFEB3B), Color(0xFFFFA726), Color(0xFFFF9800),
    // Browns/Grays
    Color(0xFF8D6E63), Color(0xFF795548), Color(0xFF757575), Color(0xFF616161),
    // Neutrals
    Color(0xFFBDBDBD), Color(0xFF9E9E9E), Color(0xFF424242), Color(0xFF212121),
    // Accents
    Color(0xFF00D4AA), Color(0xFF00E676), Color(0xFFFF5252), Color(0xFFFFD740),
    // Light/White
    Color(0xFFFFFFFF), Color(0xFFF5F5F5), Color(0xFFE0E0E0), Color(0xFF000000),
  ];

  @override
  void initState() {
    super.initState();
    _current = widget.selected;
    _syncHex();
  }

  @override
  void dispose() {
    _hexController.dispose();
    super.dispose();
  }

  void _syncHex() {
    _hexController.text =
        '#${(_current.r * 255).round().toRadixString(16).padLeft(2, '0')}'
        '${(_current.g * 255).round().toRadixString(16).padLeft(2, '0')}'
        '${(_current.b * 255).round().toRadixString(16).padLeft(2, '0')}';
  }

  void _onHexSubmitted(String text) {
    final clean = text.replaceFirst('#', '');
    if (clean.length != 6) return;
    final value = int.tryParse(clean, radix: 16);
    if (value == null) return;
    setState(() {
      _current = Color(0xFF000000 | value);
      _syncHex();
    });
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Pick a color'),
      content: SizedBox(
        width: 320,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            // Color grid
            Wrap(
              spacing: 6,
              runSpacing: 6,
              children: _palette.map((c) {
                final selected = c.toARGB32() == _current.toARGB32();
                return GestureDetector(
                  onTap: () => setState(() {
                    _current = c;
                    _syncHex();
                  }),
                  child: Container(
                    width: 32,
                    height: 32,
                    decoration: BoxDecoration(
                      color: c,
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(
                        color: selected ? NbColors.textPrimary : NbColors.glassBorder,
                        width: selected ? 2 : 1,
                      ),
                    ),
                    child: selected
                        ? Icon(Icons.check, size: 16,
                            color: c.computeLuminance() > 0.5
                                ? Colors.black
                                : Colors.white)
                        : null,
                  ),
                );
              }).toList(),
            ),

            const SizedBox(height: 16),

            // Preview + hex input
            Row(
              children: [
                Container(
                  width: 48,
                  height: 48,
                  decoration: BoxDecoration(
                    color: _current,
                    borderRadius: BorderRadius.circular(NbRadius.sm),
                    border: Border.all(color: NbColors.glassBorder),
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: TextField(
                    controller: _hexController,
                    decoration: const InputDecoration(
                      labelText: 'Hex',
                      hintText: '#RRGGBB',
                    ),
                    style: const TextStyle(fontFamily: 'monospace'),
                    onSubmitted: _onHexSubmitted,
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, _current),
          child: const Text('Select'),
        ),
      ],
    );
  }
}
