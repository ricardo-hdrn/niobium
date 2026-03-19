import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import '../../lib/widgets/page_view.dart';
import '../../lib/theme/niobium_theme.dart';
import '../../lib/models/display_config.dart';

/// Helper to build an NbPageView inside a MaterialApp with niobium theme.
Widget _buildPage(List<Map<String, dynamic>> children) {
  final completer = Completer<Map<String, dynamic>?>();
  return MaterialApp(
    theme: buildNiobiumTheme(),
    home: Scaffold(
      body: NbPageView(
        children: children,
        title: 'Test',
        completer: completer,
        display: NbDisplayConfig.defaultConfig,
      ),
    ),
  );
}

void main() {
  group('PageNode rendering', () {
    testWidgets('markdown node renders', (tester) async {
      await tester.pumpWidget(_buildPage([
        {'type': 'markdown', 'content': 'Hello world'},
      ]));
      await tester.pumpAndSettle();
      expect(find.text('Hello world'), findsOneWidget);
    });

    testWidgets('stat node renders label and value', (tester) async {
      await tester.pumpWidget(_buildPage([
        {
          'type': 'stat',
          'props': {'label': 'Tests', 'value': '42', 'variant': 'success'}
        },
      ]));
      await tester.pumpAndSettle();
      expect(find.text('Tests'), findsOneWidget);
      expect(find.text('42'), findsOneWidget);
    });

    testWidgets('stat node handles numeric value (not just string)', (tester) async {
      await tester.pumpWidget(_buildPage([
        {
          'type': 'stat',
          'props': {'label': 'Count', 'value': 142}
        },
      ]));
      await tester.pumpAndSettle();
      expect(find.text('Count'), findsOneWidget);
      expect(find.text('142'), findsOneWidget);
    });

    testWidgets('row renders children side by side', (tester) async {
      await tester.pumpWidget(_buildPage([
        {
          'type': 'row',
          'children': [
            {'type': 'markdown', 'content': 'Left'},
            {'type': 'markdown', 'content': 'Right'},
          ]
        },
      ]));
      await tester.pumpAndSettle();
      expect(find.text('Left'), findsOneWidget);
      expect(find.text('Right'), findsOneWidget);

      // Verify there's a Row containing Expanded children for the columns
      final rows = tester.widgetList<Row>(find.byType(Row));
      final hasExpandedRow = rows.any(
          (row) => row.children.whereType<Expanded>().length >= 2);
      expect(hasExpandedRow, isTrue);
    });

    testWidgets('row with col children renders content', (tester) async {
      await tester.pumpWidget(_buildPage([
        {
          'type': 'row',
          'children': [
            {
              'type': 'col',
              'children': [
                {'type': 'markdown', 'content': 'ColLeft'}
              ]
            },
            {
              'type': 'col',
              'children': [
                {'type': 'markdown', 'content': 'ColRight'}
              ]
            },
          ]
        },
      ]));
      await tester.pumpAndSettle();
      // Both col contents must be visible
      expect(find.text('ColLeft'), findsOneWidget);
      expect(find.text('ColRight'), findsOneWidget);
    });

    testWidgets('row with col containing stat renders stat', (tester) async {
      await tester.pumpWidget(_buildPage([
        {
          'type': 'row',
          'children': [
            {
              'type': 'col',
              'children': [
                {
                  'type': 'stat',
                  'props': {'label': 'Passed', 'value': 142, 'variant': 'success'}
                }
              ]
            },
            {
              'type': 'col',
              'children': [
                {
                  'type': 'stat',
                  'props': {'label': 'Failed', 'value': 3, 'variant': 'error'}
                }
              ]
            },
          ]
        },
      ]));
      await tester.pumpAndSettle();
      expect(find.text('Passed'), findsOneWidget);
      expect(find.text('142'), findsOneWidget);
      expect(find.text('Failed'), findsOneWidget);
      expect(find.text('3'), findsOneWidget);
    });

    testWidgets('alert renders with variant', (tester) async {
      await tester.pumpWidget(_buildPage([
        {
          'type': 'alert',
          'props': {'variant': 'warning'},
          'content': 'Watch out!',
        },
      ]));
      await tester.pumpAndSettle();
      expect(find.text('Warning'), findsOneWidget);
      expect(find.text('Watch out!'), findsOneWidget);
    });

    testWidgets('badge renders content', (tester) async {
      await tester.pumpWidget(_buildPage([
        {
          'type': 'badge',
          'content': 'PASSED',
          'props': {'variant': 'success'},
        },
      ]));
      await tester.pumpAndSettle();
      expect(find.text('PASSED'), findsOneWidget);
    });

    testWidgets('progress renders label', (tester) async {
      await tester.pumpWidget(_buildPage([
        {
          'type': 'progress',
          'props': {'value': 0.7, 'label': 'Loading', 'detail': '70%'},
        },
      ]));
      await tester.pumpAndSettle();
      expect(find.text('Loading'), findsOneWidget);
      expect(find.text('70%'), findsOneWidget);
      expect(find.byType(LinearProgressIndicator), findsOneWidget);
    });

    testWidgets('collapse starts collapsed and expands on tap', (tester) async {
      await tester.pumpWidget(_buildPage([
        {
          'type': 'collapse',
          'title': 'Details',
          'children': [
            {'type': 'markdown', 'content': 'Hidden content'},
          ],
        },
      ]));
      await tester.pumpAndSettle();

      // Title visible, content hidden
      expect(find.text('Details'), findsOneWidget);
      expect(find.text('Hidden content'), findsNothing);

      // Tap to expand
      await tester.tap(find.text('Details'));
      await tester.pumpAndSettle();
      expect(find.text('Hidden content'), findsOneWidget);
    });

    testWidgets('hero renders title and subtitle', (tester) async {
      await tester.pumpWidget(_buildPage([
        {
          'type': 'hero',
          'title': 'Welcome',
          'content': 'Hero subtitle',
        },
      ]));
      await tester.pumpAndSettle();
      expect(find.text('Welcome'), findsOneWidget);
      expect(find.text('Hero subtitle'), findsOneWidget);
    });

    testWidgets('input node collects data on submit', (tester) async {
      final completer = Completer<Map<String, dynamic>?>();
      await tester.pumpWidget(MaterialApp(
        theme: buildNiobiumTheme(),
        home: Scaffold(
          body: NbPageView(
            children: [
              {
                'type': 'input',
                'key': 'name',
                'field': {'type': 'string', 'title': 'Name'},
              },
            ],
            title: 'Test Form',
            completer: completer,
          ),
        ),
      ));
      await tester.pumpAndSettle();

      // Type into the text field
      await tester.enterText(find.byType(TextFormField).first, 'Ricardo');
      await tester.pumpAndSettle();

      // Tap Submit
      await tester.tap(find.text('Submit'));
      await tester.pumpAndSettle();

      final result = await completer.future;
      expect(result?['name'], 'Ricardo');
    });
  });
}
