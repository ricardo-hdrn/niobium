// Pills View — persistent feed of events from any source plugin.
//
// Each pill is a compact card in a scrolling feed. Tapping a pill opens the
// appropriate existing component (form, decision, output display, etc.).
// The pill itself only shows summary + type + status — no inline interaction.

import 'package:flutter/material.dart';
import 'package:window_manager/window_manager.dart';
import '../models/pill.dart';
import '../theme/niobium_theme.dart';

/// Maximum number of pills to keep in memory.
const _maxPills = 200;

class PillsView extends StatelessWidget {
  final List<Pill> events;
  final VoidCallback? onClose;
  final void Function(Pill pill)? onPillTap;

  const PillsView({
    super.key,
    required this.events,
    this.onClose,
    this.onPillTap,
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
              title: 'Activity',
              onClose: onClose,
              actions: [
                _PillCount(count: events.length, accent: accent),
                const SizedBox(width: 8),
              ],
            ),
          ),
          Expanded(
            child: events.isEmpty
                ? _EmptyState(accent: accent)
                : _PillsList(events: events, onPillTap: onPillTap),
          ),
        ],
      ),
    );
  }
}

class _PillCount extends StatelessWidget {
  final int count;
  final Color accent;

  const _PillCount({required this.count, required this.accent});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: accent.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(10),
      ),
      child: Text(
        '$count',
        style: TextStyle(
          color: accent,
          fontSize: 11,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

class _EmptyState extends StatelessWidget {
  final Color accent;

  const _EmptyState({required this.accent});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(Icons.stream, size: 40, color: NbColors.textTertiary),
          const SizedBox(height: NbSpacing.md),
          const Text(
            'Waiting for events...',
            style: TextStyle(
              color: NbColors.textSecondary,
              fontSize: 13,
            ),
          ),
        ],
      ),
    );
  }
}

class _PillsList extends StatelessWidget {
  final List<Pill> events;
  final void Function(Pill pill)? onPillTap;

  const _PillsList({required this.events, this.onPillTap});

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      padding: const EdgeInsets.symmetric(
        horizontal: NbSpacing.md,
        vertical: NbSpacing.sm,
      ),
      itemCount: events.length,
      itemBuilder: (context, index) => _PillCard(
        pill: events[index],
        onTap: onPillTap,
      ),
    );
  }
}

class _PillCard extends StatelessWidget {
  final Pill pill;
  final void Function(Pill pill)? onTap;

  const _PillCard({required this.pill, this.onTap});

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.primary;
    final (icon, color) = _iconAndColor(accent);
    final timeAgo = _formatTimeAgo(pill.createdAt);
    final tappable = pill.isTappable;

    return Padding(
      padding: const EdgeInsets.only(bottom: NbSpacing.sm),
      child: GestureDetector(
        onTap: tappable ? () => onTap?.call(pill) : null,
        child: Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: NbColors.surfaceElevated,
            borderRadius: BorderRadius.circular(NbRadius.sm),
            border: Border.all(
              color: pill.isAnswered
                  ? NbColors.success.withValues(alpha: 0.3)
                  : NbColors.glassBorder,
              width: 0.5,
            ),
          ),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Container(
                width: 28,
                height: 28,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: color.withValues(alpha: 0.1),
                ),
                child: Icon(icon, size: 14, color: color),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        _TypeBadge(type: pill.eventType, color: color),
                        if (pill.outputType != null) ...[
                          const SizedBox(width: 4),
                          _TypeBadge(type: pill.outputType!, color: accent),
                        ],
                        if (pill.isAnswered) ...[
                          const SizedBox(width: 4),
                          Icon(Icons.check_circle,
                              size: 12, color: NbColors.success),
                        ],
                        const Spacer(),
                        Text(
                          timeAgo,
                          style: const TextStyle(
                            color: NbColors.textTertiary,
                            fontSize: 11,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Text(
                      pill.summary,
                      style: const TextStyle(
                        color: NbColors.textPrimary,
                        fontSize: 13,
                        height: 1.4,
                      ),
                      maxLines: 3,
                      overflow: TextOverflow.ellipsis,
                    ),
                    if (pill.isAnswered) ...[
                      const SizedBox(height: 4),
                      Text(
                        pill.response!,
                        style: const TextStyle(
                          color: NbColors.success,
                          fontSize: 11,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ] else if (pill.sourceKind != null) ...[
                      const SizedBox(height: 4),
                      Text(
                        '${pill.sourceKind}${pill.sourceId != null ? ' · ${pill.sourceId}' : ''}',
                        style: const TextStyle(
                          color: NbColors.textTertiary,
                          fontSize: 11,
                        ),
                      ),
                    ],
                  ],
                ),
              ),
              if (tappable && !pill.isAnswered)
                Icon(Icons.chevron_right,
                    size: 16, color: NbColors.textTertiary),
            ],
          ),
        ),
      ),
    );
  }

  (IconData, Color) _iconAndColor(Color accent) {
    return switch (pill.outputType) {
      'decision' => (Icons.help_outline, accent),
      'form' => (Icons.edit_note, accent),
      'table' || 'datatable' => (Icons.table_chart_outlined, accent),
      'markdown' => (Icons.article_outlined, accent),
      _ => switch (pill.eventType) {
          'update_event' => (Icons.chat_bubble_outline, accent),
          'actionable_update' => (Icons.update, NbColors.warning),
          'actionable_state' => (Icons.swap_horiz, NbColors.success),
          'subject_status' => (Icons.circle, _statusColor()),
          _ => (Icons.notifications_none, NbColors.textSecondary),
        },
    };
  }

  Color _statusColor() {
    return switch (pill.newStatus) {
      'closed' => NbColors.textTertiary,
      'paused' => NbColors.warning,
      _ => NbColors.success,
    };
  }

  static String _formatTimeAgo(DateTime time) {
    final diff = DateTime.now().difference(time);
    if (diff.inSeconds < 60) return 'now';
    if (diff.inMinutes < 60) return '${diff.inMinutes}m';
    if (diff.inHours < 24) return '${diff.inHours}h';
    return '${diff.inDays}d';
  }
}

class _TypeBadge extends StatelessWidget {
  final String type;
  final Color color;

  const _TypeBadge({required this.type, required this.color});

  @override
  Widget build(BuildContext context) {
    final label = switch (type) {
      'update_event' => 'update',
      'actionable_update' => 'progress',
      'actionable_state' => 'state',
      'subject_status' => 'status',
      _ => type,
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 1),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: 10,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.3,
        ),
      ),
    );
  }
}

/// Maximum number of pills to retain in the event list.
int get maxPillCount => _maxPills;
