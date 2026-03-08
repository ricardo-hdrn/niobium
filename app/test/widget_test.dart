import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:niobium_app/theme/niobium_theme.dart';

void main() {
  testWidgets('Niobium theme builds without errors', (WidgetTester tester) async {
    final theme = buildNiobiumTheme();
    await tester.pumpWidget(
      MaterialApp(
        theme: theme,
        home: const Scaffold(
          body: Center(child: Text('Niobium')),
        ),
      ),
    );
    expect(find.text('Niobium'), findsOneWidget);
  });
}
