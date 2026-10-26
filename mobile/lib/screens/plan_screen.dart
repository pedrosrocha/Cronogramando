import 'package:flutter/material.dart';

import '../api_client.dart';
import '../widgets.dart';

const _kinds = ['resource', 'quiz', 'task'];
const _activityTypes = ['video', 'podcast', 'pdf', 'article', 'quiz', 'task', 'lesson'];
const _contentIntents = [
  '',
  'explanation',
  'worked_example',
  'practice',
  'assessment',
  'review',
];

class PlanScreen extends StatefulWidget {
  const PlanScreen({super.key, required this.api});

  final ApiClient api;

  @override
  State<PlanScreen> createState() => _PlanScreenState();
}

class _PlanScreenState extends State<PlanScreen> with ApiRunner<PlanScreen> {
  final _bookId = TextEditingController();
  final _stepId = TextEditingController();
  final _activityId = TextEditingController();
  final _title = TextEditingController();
  final _duration = TextEditingController(text: '30');
  final _references = TextEditingController();
  final _resourceUrl = TextEditingController();
  final _orderIndex = TextEditingController();
  final _learningOutcome = TextEditingController();
  final _successCriteria = TextEditingController();
  final _bookObjective = TextEditingController();
  final _bookAssessment = TextEditingController();
  final _bookScopeIn = TextEditingController();
  final _bookScopeOut = TextEditingController();
  final _bookPrerequisites = TextEditingController();
  String _kind = 'resource';
  String _activityType = '';
  String _contentIntent = '';
  bool _completed = true;

  @override
  void dispose() {
    _bookId.dispose();
    _stepId.dispose();
    _activityId.dispose();
    _title.dispose();
    _duration.dispose();
    _references.dispose();
    _resourceUrl.dispose();
    _orderIndex.dispose();
    _learningOutcome.dispose();
    _successCriteria.dispose();
    _bookObjective.dispose();
    _bookAssessment.dispose();
    _bookScopeIn.dispose();
    _bookScopeOut.dispose();
    _bookPrerequisites.dispose();
    super.dispose();
  }

  String? _requireId(TextEditingController controller, String label) {
    final value = emptyToNull(controller.text);
    if (value == null) {
      setState(() {
        errorMessage = '$label is required';
        result = null;
      });
    }
    return value;
  }

  Map<String, dynamic> _createStepBody() {
    final references = emptyToNull(_references.text);
    final learning = emptyToNull(_learningOutcome.text);
    final criteria = emptyToNull(_successCriteria.text);
    final body = <String, dynamic>{
      'title': _title.text.trim(),
      'activity_kind': _kind,
      'estimated_duration_mins': int.tryParse(_duration.text.trim()) ?? 30,
    };
    if (references != null) body['references_step_id'] = references;
    if (learning != null) body['learning_outcome'] = learning;
    if (criteria != null) body['success_criteria'] = criteria;
    if (_contentIntent.isNotEmpty) body['content_intent'] = _contentIntent;
    if (_activityType.isNotEmpty) {
      final url = emptyToNull(_resourceUrl.text);
      body['activity'] = {
        'type': _activityType,
        if (url != null) 'resource_url': url,
      };
    }
    return body;
  }

  Map<String, dynamic> _updateStepBody() {
    final title = emptyToNull(_title.text);
    final order = emptyToNull(_orderIndex.text);
    final references = emptyToNull(_references.text);
    final learning = emptyToNull(_learningOutcome.text);
    final criteria = emptyToNull(_successCriteria.text);
    return {
      if (title != null) 'title': title,
      'activity_kind': _kind,
      'estimated_duration_mins': int.tryParse(_duration.text.trim()) ?? 30,
      if (references != null) 'references_step_id': references,
      if (order != null) 'order_index': int.tryParse(order) ?? 0,
      if (learning != null) 'learning_outcome': learning,
      if (criteria != null) 'success_criteria': criteria,
      if (_contentIntent.isNotEmpty) 'content_intent': _contentIntent,
    };
  }

  List<String> _listFrom(TextEditingController controller) => controller.text
      .split(',')
      .map((value) => value.trim())
      .where((value) => value.isNotEmpty)
      .toList();

  Map<String, dynamic> _bookBody() {
    final body = <String, dynamic>{};
    final objective = emptyToNull(_bookObjective.text);
    final assessment = emptyToNull(_bookAssessment.text);
    if (objective != null) body['objective'] = objective;
    if (assessment != null) body['assessment'] = assessment;
    final scopeIn = _listFrom(_bookScopeIn);
    if (scopeIn.isNotEmpty) body['scope_in'] = scopeIn;
    final scopeOut = _listFrom(_bookScopeOut);
    if (scopeOut.isNotEmpty) body['scope_out'] = scopeOut;
    final prerequisites = _listFrom(_bookPrerequisites);
    if (prerequisites.isNotEmpty) body['prerequisites'] = prerequisites;
    return body;
  }

  @override
  Widget build(BuildContext context) {
    final api = widget.api;
    return ListView(
      padding: const EdgeInsets.all(12),
      children: [
        SectionCard(
          title: 'Plan requests',
          subtitle:
              'GET /books/{id}/plan · POST /books/{id}/generate-schedule',
          children: [
            AppTextField(controller: _bookId, label: 'Módulo id', hint: 'UUID'),
            const SizedBox(height: 4),
            WrapActions(
              children: [
                FilledButton.tonal(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requireId(_bookId, 'Módulo id');
                          if (id == null) return;
                          runApi(() => api.getPlan(id));
                        },
                  child: const Text('Load plan'),
                ),
                FilledButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requireId(_bookId, 'Módulo id');
                          if (id == null) return;
                          runApi(() => api.generateSchedule(id));
                        },
                  child: const Text('Generate plan'),
                ),
              ],
            ),
          ],
        ),
        SectionCard(
          title: 'Module contract',
          subtitle: 'PATCH /books/{id} · edit the objective before generating the plan',
          children: [
            AppTextField(controller: _bookObjective, label: 'Objective'),
            AppTextField(controller: _bookAssessment, label: 'Assessment'),
            AppTextField(
              controller: _bookScopeIn,
              label: 'Scope in (comma separated)',
            ),
            AppTextField(
              controller: _bookScopeOut,
              label: 'Scope out (comma separated)',
            ),
            AppTextField(
              controller: _bookPrerequisites,
              label: 'Prerequisites (comma separated)',
            ),
            const SizedBox(height: 8),
            FilledButton.tonal(
              onPressed: busy
                  ? null
                  : () {
                      final id = _requireId(_bookId, 'Módulo id');
                      if (id == null) return;
                      runApi(() => api.updateBook(id, _bookBody()));
                    },
              child: const Text('Patch module'),
            ),
          ],
        ),
        SectionCard(
          title: 'Step form',
          subtitle: 'POST /books/{id}/steps · PATCH /steps/{id}',
          children: [
            AppTextField(controller: _title, label: 'Step title'),
            DropdownButtonFormField<String>(
              initialValue: _kind,
              decoration: const InputDecoration(
                labelText: 'Activity kind',
                isDense: true,
                border: OutlineInputBorder(),
              ),
              items: [
                for (final kind in _kinds)
                  DropdownMenuItem(value: kind, child: Text(kind)),
              ],
              onChanged: (value) {
                if (value != null) setState(() => _kind = value);
              },
            ),
            const SizedBox(height: 10),
            AppTextField(
              controller: _duration,
              label: 'Estimated duration (minutes)',
              keyboardType: TextInputType.number,
            ),
            AppTextField(
              controller: _references,
              label: 'References step id (optional)',
              hint: 'UUID',
            ),
            AppTextField(
              controller: _learningOutcome,
              label: 'Learning outcome',
            ),
            AppTextField(
              controller: _successCriteria,
              label: 'Success criteria',
            ),
            DropdownButtonFormField<String>(
              initialValue: _contentIntent,
              decoration: const InputDecoration(
                labelText: 'Content intent',
                isDense: true,
                border: OutlineInputBorder(),
              ),
              items: [
                for (final intent in _contentIntents)
                  DropdownMenuItem(
                    value: intent,
                    child: Text(intent.isEmpty ? '(keep/default)' : intent),
                  ),
              ],
              onChanged: (value) {
                if (value != null) setState(() => _contentIntent = value);
              },
            ),
            const SizedBox(height: 10),
            AppTextField(
              controller: _orderIndex,
              label: 'Order index (PATCH only)',
              keyboardType: TextInputType.number,
            ),
            AppTextField(
              controller: _resourceUrl,
              label: 'Activity resource URL (optional)',
            ),
            DropdownButtonFormField<String>(
              initialValue: _activityType,
              decoration: const InputDecoration(
                labelText: 'Activity type',
                isDense: true,
                border: OutlineInputBorder(),
              ),
              items: [
                const DropdownMenuItem(value: '', child: Text('(none)')),
                for (final type in _activityTypes)
                  DropdownMenuItem(value: type, child: Text(type)),
              ],
              onChanged: (value) {
                if (value != null) setState(() => _activityType = value);
              },
            ),
            const SizedBox(height: 12),
            AppTextField(
              controller: _stepId,
              label: 'Step id (PATCH/DELETE/regenerate)',
              hint: 'UUID',
            ),
            const SizedBox(height: 4),
            WrapActions(
              children: [
                FilledButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requireId(_bookId, 'Módulo id');
                          if (id == null) return;
                          runApi(() => api.createStep(id, _createStepBody()));
                        },
                  child: const Text('Create step'),
                ),
                FilledButton.tonal(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requireId(_stepId, 'Step id');
                          if (id == null) return;
                          runApi(() => api.updateStep(id, _updateStepBody()));
                        },
                  child: const Text('Patch step'),
                ),
                OutlinedButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requireId(_stepId, 'Step id');
                          if (id == null) return;
                          runApi(() => api.deleteStep(id));
                        },
                  child: const Text('Delete step'),
                ),
                OutlinedButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requireId(_stepId, 'Step id');
                          if (id == null) return;
                          runApi(() => api.regenerateResource(id));
                        },
                  child: const Text('Regenerate resource'),
                ),
                OutlinedButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requireId(_stepId, 'Step id');
                          if (id == null) return;
                          runApi(() => api.generateLesson(id));
                        },
                  child: const Text('Generate lesson'),
                ),
              ],
            ),
          ],
        ),
        SectionCard(
          title: 'Activity completion',
          subtitle: 'PATCH /activities/{id}/complete',
          children: [
            AppTextField(
              controller: _activityId,
              label: 'Activity id',
              hint: 'UUID',
            ),
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('Completed'),
              value: _completed,
              onChanged: (value) => setState(() => _completed = value),
            ),
            WrapActions(
              children: [
                FilledButton.tonal(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requireId(_activityId, 'Activity id');
                          if (id == null) return;
                          runApi(
                            () => api.completeActivity(id, _completed),
                          );
                        },
                  child: const Text('Send completion'),
                ),
              ],
            ),
          ],
        ),
        ResultPanel(
          busy: busy,
          errorMessage: errorMessage,
          result: result,
        ),
      ],
    );
  }
}
