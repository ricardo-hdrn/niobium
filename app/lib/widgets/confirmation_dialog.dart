import 'dart:async';
import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';
import '../theme/niobium_theme.dart';

class ConfirmationDialog extends StatelessWidget {
  final String message;
  final String title;
  final Completer<bool> completer;

  const ConfirmationDialog({
    super.key,
    required this.message,
    required this.title,
    required this.completer,
  });

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.primary;
    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Column(
        children: [
          DragToMoveArea(
            child: NbTitleBar(
              title: title,
              onClose: () {
                if (!completer.isCompleted) completer.complete(false);
              },
            ),
          ),
          Expanded(
            child: Center(
              child: Padding(
                padding: const EdgeInsets.all(NbSpacing.xl),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 56,
                      height: 56,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: accent.withValues(alpha: 0.08),
                        border: Border.all(
                            color: accent.withValues(alpha: 0.19), width: 1.5),
                      ),
                      child: Icon(Icons.help_outline,
                          size: 28, color: accent),
                    ),
                    const SizedBox(height: NbSpacing.lg),
                    Text(
                      message,
                      style: const TextStyle(
                        color: NbColors.textPrimary,
                        fontSize: 15,
                        height: 1.5,
                      ),
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: NbSpacing.xl),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        OutlinedButton(
                          onPressed: () {
                            if (!completer.isCompleted) {
                              completer.complete(false);
                            }
                          },
                          style: OutlinedButton.styleFrom(
                            padding: const EdgeInsets.symmetric(
                                horizontal: 32, vertical: 14),
                          ),
                          child: const Text('No'),
                        ),
                        const SizedBox(width: 12),
                        FilledButton(
                          onPressed: () {
                            if (!completer.isCompleted) {
                              completer.complete(true);
                            }
                          },
                          style: FilledButton.styleFrom(
                            padding: const EdgeInsets.symmetric(
                                horizontal: 32, vertical: 14),
                          ),
                          child: const Text('Yes'),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
