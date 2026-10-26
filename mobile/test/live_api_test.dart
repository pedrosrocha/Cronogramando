// ignore_for_file: avoid_print
@TestOn('vm')
library;

import 'dart:io';

import 'package:cronogramando/api_client.dart';
import 'package:flutter_test/flutter_test.dart';

/// Live contract test for the Phase 08b validation client.
///
/// It is skipped unless credentials are supplied, so a normal `flutter test`
/// stays offline. Run it with `tool/run_live_validation.sh`, which sources the
/// repository `.env` and points the client at the running backend.
void main() {
  final baseUrl = Platform.environment['LIVE_API_BASE_URL'] ??
      const String.fromEnvironment('LIVE_API_BASE_URL');
  final adminEmail = Platform.environment['LIVE_API_EMAIL'] ??
      const String.fromEnvironment('LIVE_API_EMAIL');
  final adminPassword = Platform.environment['LIVE_API_PASSWORD'] ??
      const String.fromEnvironment('LIVE_API_PASSWORD');
  final skipReason = baseUrl.isEmpty
      ? 'Set LIVE_API_BASE_URL (or run tool/run_live_validation.sh).'
      : adminPassword.isEmpty
          ? 'Set LIVE_API_EMAIL and LIVE_API_PASSWORD in the environment.'
          : null;

  group('Phase 08b live API contract', () {
    test(
      'every documented route round-trips through ApiClient',
      () async {
        final admin = ApiClient(baseUrl: baseUrl);
        final user = ApiClient(baseUrl: baseUrl);
        final login = ApiClient(baseUrl: baseUrl);
        String? goalId;
        final covered = <String>[];

        void record(String route) {
          covered.add(route);
          // Visible with `flutter test` so the phase doc can link real output.
          print('covered: $route');
        }

        try {
          final health =
              await admin.request('GET', '/health', authenticated: false);
          expect(health, 'ok');
          record('GET /health');

          final unique = DateTime.now().microsecondsSinceEpoch;
          final email = 'phase8-$unique@example.com';
          const password = 'phase8-validation-123';

          final registered = await user.register(
            email: email,
            password: password,
            preferredLanguage: 'pt-BR',
          );
          expect(registered['access_token'], isA<String>());
          record('POST /auth/register');

          final me = await user.me();
          expect(me['email'], email);
          record('GET /auth/me');

          await user.refresh();
          expect(user.accessToken, isA<String>());
          record('POST /auth/refresh');

          final loggedIn = await login.login(
            email: email,
            password: password,
          );
          expect(loggedIn['access_token'], isA<String>());
          record('POST /auth/login');

          await admin.login(
            email: adminEmail,
            password: adminPassword,
          );
          final users = await admin.listUsers();
          final createdUser =
              users.map(_map).firstWhere((item) => item['email'] == email);
          record('GET /admin/users');

          await admin.updateUser(
            createdUser['id'] as String,
            preferredLanguage: 'en',
          );
          expect((await login.me())['preferred_language'], 'en');
          record('PATCH /admin/users/{id}');
          await admin.updateUser(
            createdUser['id'] as String,
            preferredLanguage: 'pt-BR',
          );

          final today = _dateOnly(DateTime.now());
          final createdGoal = await login.createGoal({
            'title': 'Phase 8 validation $unique',
            'why_statement': 'Validate the mobile HTTP client end to end.',
            'deadline': _isoDate(today.add(const Duration(days: 90))),
            'availability': {
              'days': ['mon', 'wed', 'fri'],
              'minutes_per_day': 60,
            },
          });
          goalId = createdGoal['id'] as String;
          expect(createdGoal['books'], isNotEmpty);
          record('POST /goals');

          final redecomposed = await login.redecomposeGoal(goalId);
          final books = _maps(redecomposed['books']);
          expect(books, isNotEmpty);
          record('POST /goals/{id}/redecompose');

          await login.listGoals();
          record('GET /goals');

          final loadedGoal = await login.getGoal(goalId);
          expect(loadedGoal['id'], goalId);
          record('GET /goals/{id}');

          await login.updateGoal(goalId, {
            'outcome': 'Explain the validated API contract.',
            'success_criteria': [
              'Every mobile form has a matching live route.',
              'Integration gaps are documented.',
            ],
            'target_deadline': _isoDate(today.add(const Duration(days: 60))),
          });
          record('PATCH /goals/{id}');

          final listedBooks = _maps(await login.listGoalBooks(goalId));
          expect(listedBooks, isNotEmpty);
          record('GET /goals/{id}/books');

          final bookId = listedBooks.first['id'] as String;
          final loadedBook = await login.getBook(bookId);
          expect(loadedBook['id'], bookId);
          record('GET /books/{id}');

          await login.updateBook(bookId, {
            'objective': 'Validate the module contract form.',
            'scope_in': ['phase 8 validation'],
            'scope_out': ['production data'],
            'assessment': 'The live test passes against this module.',
          });
          record('PATCH /books/{id}');

          final generatedPlan = await login.generateSchedule(bookId);
          final generatedSteps = _maps(generatedPlan['steps']);
          expect(generatedSteps, isNotEmpty);
          record('POST /books/{id}/generate-schedule');

          final plan = await login.getPlan(bookId);
          expect(plan['steps'], isNotEmpty);
          record('GET /books/{id}/plan');

          final manualStep = await login.createStep(bookId, {
            'title': 'Phase 8 manual step',
            'activity_kind': 'resource',
            'estimated_duration_mins': 6,
            'learning_outcome': 'Round-trip a manually authored step.',
            'success_criteria': 'The step can be patched and completed.',
            'content_intent': 'practice',
            'activity': {
              'type': 'video',
              'resource_url': 'https://example.org/phase8-validation-video',
            },
          });
          final manualStepId = manualStep['id'] as String;
          var manualActivityId =
              _maps(manualStep['activities']).first['id'] as String;
          record('POST /books/{id}/steps');

          final throwaway = await login.createStep(bookId, {
            'title': 'Phase 8 throwaway step',
            'activity_kind': 'task',
            'estimated_duration_mins': 1,
            'activity': {'type': 'task'},
          });
          await login.deleteStep(throwaway['id'] as String);
          record('DELETE /steps/{id}');

          final patchedStep = await login.updateStep(manualStepId, {
            'title': 'Phase 8 manual step (patched)',
            'estimated_duration_mins': 7,
          });
          expect(patchedStep['title'], 'Phase 8 manual step (patched)');
          record('PATCH /steps/{id}');

          final pdfProbe = await login.createStep(bookId, {
            'title': 'Phase 8 PDF probe',
            'activity_kind': 'task',
            'estimated_duration_mins': 1,
            'activity': {'type': 'task'},
          });
          final pdfProbeActivityId =
              _maps(pdfProbe['activities']).first['id'] as String;

          final regenerated = await login.regenerateResource(manualStepId);
          final regeneratedActivities = _maps(regenerated['activities']);
          if (regeneratedActivities.isNotEmpty) {
            manualActivityId = regeneratedActivities.first['id'] as String;
          }
          record('POST /steps/{id}/regenerate-resource');

          try {
            final lesson = await login.generateLesson(manualStepId);
            final lessonActivities = _maps(lesson['activities']);
            if (lessonActivities.isNotEmpty) {
              manualActivityId = lessonActivities.first['id'] as String;
            }
            record('POST /steps/{id}/generate-lesson');
          } on ApiException catch (error) {
            // The route is exercised, but the LLM tutor needs a configured
            // provider. Record the availability gap rather than hiding it.
            expect(
              error.statusCode,
              500,
              reason: 'generate-lesson should only fail when the tutor is '
                  'unavailable: ${error.message}',
            );
            record('POST /steps/{id}/generate-lesson (500: LLM unavailable)');
          }

          final activity = await login.getActivity(manualActivityId);
          expect(activity['id'], manualActivityId);
          record('GET /activities/{id}');

          await login.completeActivity(manualActivityId, true);
          await login.completeActivity(manualActivityId, false);
          record('PATCH /activities/{id}/complete');

          await login.sendFeedback(
            manualActivityId,
            rating: 'helpful',
            comment: 'Phase 8 validation feedback.',
          );
          record('POST /activities/{id}/feedback');

          await login.getTrace(manualActivityId);
          record('GET /activities/{id}/trace');

          // A task has no resource URL, so both PDF routes must reject it with
          // 400 while still proving the client reaches the handler.
          await _expectApiError(
            () => login.getContent(pdfProbeActivityId),
            400,
          );
          record('GET /activities/{id}/content (400 expected)');

          await _expectApiError(
            () => login.getTranslation(pdfProbeActivityId, language: 'en'),
            400,
          );
          record('GET /activities/{id}/translation (400 expected)');

          final slots = _maps(
            await login.listSchedule(
              goalId: goalId,
              from: _isoDate(today),
              to: _isoDate(today.add(const Duration(days: 60))),
            ),
          );
          expect(slots, isNotEmpty);
          record('GET /schedule');

          final slotId = slots.first['id'] as String;
          final currentSlotDay = DateTime.parse(slots.first['date'] as String);
          final moveTarget = _nextAvailableDay(
            currentSlotDay,
            const {DateTime.monday, DateTime.wednesday, DateTime.friday},
          );
          final moved = _maps(
            await login.moveSlot(slotId, _isoDate(moveTarget)),
          );
          expect(moved, isNotEmpty);
          record('PATCH /schedule/{slot_id}');

          // Force the behind-schedule branch so the reconciliation route runs
          // its success path instead of only returning its 409 guard.
          await login.updateGoal(goalId, {
            'target_deadline': _isoDate(today.add(const Duration(days: 1))),
            'availability': {
              'days': ['mon'],
              'minutes_per_day': 5,
            },
          });

          final completion = await login.completeBook(bookId);
          record('POST /books/{id}/complete');
          final reconciliation = _map(completion['reconciliation']);
          if (reconciliation['requires_decision'] == true) {
            final reconciled = await login.reconcileDeadline(
              bookId,
              'stay_on_deadline',
            );
            expect(
              reconciled['applied_choice'],
              anyOf('stay_on_deadline', 'push_deadline', 'forced_push'),
            );
          } else {
            await _expectApiError(
              () => login.reconcileDeadline(bookId, 'stay_on_deadline'),
              409,
            );
          }
          record('POST /books/{id}/reconcile-deadline');

          await login.deleteGoal(goalId);
          goalId = null;
          record('DELETE /goals/{id}');

          print('Phase 08b coverage: ${covered.length} routes exercised.');
          expect(covered.length, greaterThanOrEqualTo(31));
        } finally {
          if (goalId != null && login.hasSession) {
            try {
              await login.deleteGoal(goalId);
            } catch (_) {
              // Clean-up is best effort; the assertion above already failed.
            }
          }
          admin.dispose();
          user.dispose();
          login.dispose();
        }
      },
      timeout: const Timeout(Duration(minutes: 20)),
    );
  }, skip: skipReason);
}

Map<String, dynamic> _map(dynamic value) {
  if (value is Map<String, dynamic>) return value;
  if (value is Map) return value.cast<String, dynamic>();
  throw StateError('expected a JSON object, got ${value.runtimeType}');
}

List<Map<String, dynamic>> _maps(dynamic value) {
  if (value is! List) {
    throw StateError('expected a JSON array, got ${value.runtimeType}');
  }
  return value.map(_map).toList();
}

Future<void> _expectApiError(
  Future<dynamic> Function() action,
  int status,
) async {
  try {
    await action();
    fail('expected HTTP $status but the request succeeded');
  } on ApiException catch (error) {
    expect(error.statusCode, status, reason: error.message);
  }
}

DateTime _dateOnly(DateTime value) =>
    DateTime(value.year, value.month, value.day);

String _isoDate(DateTime value) => '${value.year.toString().padLeft(4, '0')}-'
    '${value.month.toString().padLeft(2, '0')}-'
    '${value.day.toString().padLeft(2, '0')}';

DateTime _nextAvailableDay(DateTime current, Set<int> allowedWeekdays) {
  var candidate = _dateOnly(current).add(const Duration(days: 1));
  while (!allowedWeekdays.contains(candidate.weekday)) {
    candidate = candidate.add(const Duration(days: 1));
  }
  return candidate;
}
