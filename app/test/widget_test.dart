import 'package:flutter_test/flutter_test.dart';
import 'package:niobium_app/main.dart';

void main() {
  testWidgets('App shows idle screen on startup', (WidgetTester tester) async {
    await tester.pumpWidget(const NiobiumApp());
    expect(find.text('Niobium'), findsOneWidget);
  });
}
