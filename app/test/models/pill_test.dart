import 'package:flutter_test/flutter_test.dart';
import 'package:niobium_app/models/pill.dart';

void main() {
  group('Pill.fromJson', () {
    test('parses flat pill JSON', () {
      final pill = Pill.fromJson({
        'source': 'hub',
        'summary': 'Deployed v2.0 to staging',
        'created_at': '2026-03-07T12:00:00Z',
        'meta': {
          'event_type': 'update_event',
          'subject_id': 'sub-1',
          'source_kind': 'agent',
          'source_id': 'claude-1',
        },
      });

      expect(pill.source, 'hub');
      expect(pill.summary, 'Deployed v2.0 to staging');
      expect(pill.sourceKind, 'agent');
      expect(pill.sourceId, 'claude-1');
      expect(pill.subjectId, 'sub-1');
      expect(pill.eventType, 'update_event');
      expect(pill.isDecision, isFalse);
    });

    test('parses pill with output_type and options', () {
      final pill = Pill.fromJson({
        'source': 'hub',
        'summary': 'Deploy to production?',
        'created_at': '2026-03-07T12:00:00Z',
        'output_type': 'decision',
        'options': ['Yes, deploy', 'No, rollback', 'Wait'],
        'response_url': 'https://hub.example.com/route/act-5/10',
        'meta': {
          'event_type': 'actionable_update',
          'subject_id': 'sub-1',
          'source_kind': 'worker',
          'source_id': 'w-1',
        },
      });

      expect(pill.isDecision, isTrue);
      expect(pill.options, ['Yes, deploy', 'No, rollback', 'Wait']);
      expect(pill.hasRemoteSink, isTrue);
      expect(pill.responseUrl, 'https://hub.example.com/route/act-5/10');
      expect(pill.isAnswered, isFalse);
    });

    test('parses state transition pill', () {
      final pill = Pill.fromJson({
        'source': 'hub',
        'summary': 'proposed → dispatched',
        'created_at': '2026-03-07T12:30:00Z',
        'meta': {
          'event_type': 'actionable_state',
          'subject_id': 'sub-1',
          'actionable_id': 'act-5',
          'old_state': 'proposed',
          'new_state': 'dispatched',
        },
      });

      expect(pill.eventType, 'actionable_state');
      expect(pill.newState, 'dispatched');
      expect(pill.summary, 'proposed → dispatched');
    });

    test('parses status change pill', () {
      final pill = Pill.fromJson({
        'source': 'hub',
        'summary': 'open → closed',
        'created_at': '2026-03-07T13:00:00Z',
        'meta': {
          'event_type': 'subject_status',
          'subject_id': 'sub-1',
          'old_status': 'open',
          'new_status': 'closed',
        },
      });

      expect(pill.eventType, 'subject_status');
      expect(pill.newStatus, 'closed');
    });

    test('handles missing created_at gracefully', () {
      final pill = Pill.fromJson({
        'source': 'hub',
        'summary': 'test',
      });

      expect(pill.createdAt.year, DateTime.now().year);
    });

    test('handles missing meta gracefully', () {
      final pill = Pill.fromJson({
        'source': 'watcher',
        'summary': 'file changed',
        'created_at': '2026-03-07T12:00:00Z',
      });

      expect(pill.meta, isNull);
      expect(pill.sourceKind, isNull);
      expect(pill.eventType, 'watcher'); // falls back to source
    });
  });

  group('Pill interaction', () {
    test('isDecision is false without output_type', () {
      final pill = Pill.fromJson({
        'source': 'hub',
        'summary': 'Regular update',
        'created_at': '2026-03-07T12:00:00Z',
      });

      expect(pill.isDecision, isFalse);
      expect(pill.hasRemoteSink, isFalse);
    });

    test('marks as answered when response is set', () {
      final pill = Pill.fromJson({
        'source': 'hub',
        'summary': 'Pick one',
        'created_at': '2026-03-07T12:00:00Z',
        'output_type': 'decision',
        'options': ['A', 'B'],
        'response_url': 'https://hub.example.com/route/act-5/10',
      });

      expect(pill.isAnswered, isFalse);
      pill.response = 'A';
      expect(pill.isAnswered, isTrue);
    });

    test('isTappable when outputType is set', () {
      final pill = Pill.fromJson({
        'source': 'hub',
        'summary': 'Table data',
        'output_type': 'table',
      });

      expect(pill.isTappable, isTrue);
    });
  });
}
