import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:niobium_app/models/pill.dart';
import 'package:niobium_app/widgets/pills_view.dart';
import 'package:niobium_app/theme/niobium_theme.dart';

Widget _wrap(Widget child) {
  return MaterialApp(
    theme: buildNiobiumTheme(),
    home: Container(
      decoration: const BoxDecoration(gradient: NbColors.bgGradient),
      child: child,
    ),
  );
}

Pill _makePill({
  String source = 'hub',
  String summary = 'Test event',
  String? eventType,
  String? sourceKind,
  String? newState,
  String? newStatus,
  String? outputType,
  List<String>? options,
  String? responseUrl,
}) {
  final meta = <String, dynamic>{};
  if (eventType != null) meta['event_type'] = eventType;
  if (sourceKind != null) meta['source_kind'] = sourceKind;
  if (newState != null) meta['new_state'] = newState;
  if (newStatus != null) meta['new_status'] = newStatus;

  return Pill.fromJson({
    'source': source,
    'summary': summary,
    'created_at': DateTime.now().toIso8601String(),
    if (outputType != null) 'output_type': outputType,
    if (options != null) 'options': options,
    if (responseUrl != null) 'response_url': responseUrl,
    if (meta.isNotEmpty) 'meta': meta,
  });
}

void main() {
  group('PillsView', () {
    testWidgets('shows empty state when no events', (tester) async {
      await tester.pumpWidget(_wrap(
        const PillsView(events: []),
      ));

      expect(find.text('Activity'), findsOneWidget);
      expect(find.text('Waiting for events...'), findsOneWidget);
      expect(find.text('0'), findsOneWidget);
    });

    testWidgets('renders event pills', (tester) async {
      final events = [
        _makePill(summary: 'Deployed v2.0'),
        _makePill(
          summary: 'proposed → dispatched',
          eventType: 'actionable_state',
        ),
      ];

      await tester.pumpWidget(_wrap(
        PillsView(events: events),
      ));

      expect(find.text('Deployed v2.0'), findsOneWidget);
      expect(find.text('proposed → dispatched'), findsOneWidget);
      expect(find.text('2'), findsOneWidget);
    });

    testWidgets('shows type badges', (tester) async {
      final events = [
        _makePill(eventType: 'update_event'),
        _makePill(eventType: 'actionable_update'),
        _makePill(eventType: 'actionable_state'),
        _makePill(eventType: 'subject_status'),
      ];

      await tester.pumpWidget(_wrap(
        PillsView(events: events),
      ));

      expect(find.text('update'), findsOneWidget);
      expect(find.text('progress'), findsOneWidget);
      expect(find.text('state'), findsOneWidget);
      expect(find.text('status'), findsOneWidget);
    });

    testWidgets('shows source info when available', (tester) async {
      final events = [_makePill(sourceKind: 'worker')];

      await tester.pumpWidget(_wrap(
        PillsView(events: events),
      ));

      expect(find.textContaining('worker'), findsOneWidget);
    });

    testWidgets('calls onClose when close button tapped', (tester) async {
      var closed = false;

      await tester.pumpWidget(_wrap(
        Scaffold(
          backgroundColor: Colors.transparent,
          body: Column(
            children: [
              NbTitleBar(
                title: 'Activity',
                onClose: () => closed = true,
              ),
            ],
          ),
        ),
      ));

      final closeIcon = find.byIcon(Icons.close);
      expect(closeIcon, findsOneWidget);
      await tester.tap(closeIcon);
      await tester.pump();

      expect(closed, isTrue);
    });

    testWidgets('shows time ago as "now" for recent events', (tester) async {
      final events = [_makePill()];

      await tester.pumpWidget(_wrap(
        PillsView(events: events),
      ));

      expect(find.text('now'), findsOneWidget);
    });
  });

  group('Pill tap routing', () {
    testWidgets('tappable pills have chevron icon', (tester) async {
      final events = [
        _makePill(
          summary: 'Deploy?',
          outputType: 'decision',
          options: ['Yes', 'No'],
        ),
      ];

      await tester.pumpWidget(_wrap(
        PillsView(events: events),
      ));

      expect(find.byIcon(Icons.chevron_right), findsOneWidget);
    });

    testWidgets('non-interactive pills have no chevron', (tester) async {
      final events = [_makePill(summary: 'Regular update')];

      await tester.pumpWidget(_wrap(
        PillsView(events: events),
      ));

      expect(find.byIcon(Icons.chevron_right), findsNothing);
    });

    testWidgets('calls onPillTap when tappable pill is tapped', (tester) async {
      Pill? tapped;

      final events = [
        _makePill(
          summary: 'Pick one',
          outputType: 'decision',
          options: ['A', 'B'],
        ),
      ];

      await tester.pumpWidget(_wrap(
        PillsView(
          events: events,
          onPillTap: (pill) => tapped = pill,
        ),
      ));

      await tester.tap(find.text('Pick one'));
      await tester.pump();

      expect(tapped, isNotNull);
      expect(tapped!.outputType, 'decision');
    });

    testWidgets('shows output_type badge on typed pills', (tester) async {
      final events = [
        _makePill(
          eventType: 'actionable_update',
          outputType: 'decision',
          options: ['A'],
        ),
      ];

      await tester.pumpWidget(_wrap(
        PillsView(events: events),
      ));

      expect(find.text('decision'), findsOneWidget);
      expect(find.text('progress'), findsOneWidget);
    });

    testWidgets('answered pill shows check icon and response', (tester) async {
      final pill = _makePill(
        summary: 'Pick one',
        outputType: 'decision',
        options: ['A', 'B'],
      );
      pill.response = 'A';

      await tester.pumpWidget(_wrap(
        PillsView(events: [pill]),
      ));

      expect(find.byIcon(Icons.check_circle), findsOneWidget);
      expect(find.text('A'), findsOneWidget);
      // No chevron on answered pills
      expect(find.byIcon(Icons.chevron_right), findsNothing);
    });
  });

  group('maxPillCount', () {
    test('returns bounded value', () {
      expect(maxPillCount, 200);
    });
  });
}
