import 'package:flutter/material.dart';

import '../api_client.dart';
import '../widgets.dart';

const _weekDays = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'];

class GoalsScreen extends StatefulWidget {
  const GoalsScreen({super.key, required this.api});

  final ApiClient api;

  @override
  State<GoalsScreen> createState() => _GoalsScreenState();
}

class _GoalsScreenState extends State<GoalsScreen> with ApiRunner<GoalsScreen> {
  final _goalId = TextEditingController();
  final _bookId = TextEditingController();
  final _title = TextEditingController();
  final _why = TextEditingController();
  final _outcome = TextEditingController();
  final _successCriteria = TextEditingController();
  final _deadline = TextEditingController();
  final _targetDeadline = TextEditingController();
  final _minutes = TextEditingController(text: '60');
  final Set<String> _days = {'mon', 'wed', 'fri'};

  @override
  void dispose() {
    _goalId.dispose();
    _bookId.dispose();
    _title.dispose();
    _why.dispose();
    _outcome.dispose();
    _successCriteria.dispose();
    _deadline.dispose();
    _targetDeadline.dispose();
    _minutes.dispose();
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

  Map<String, dynamic> _createBody() {
    final target = emptyToNull(_targetDeadline.text);
    return {
      'title': _title.text.trim(),
      if (emptyToNull(_why.text) != null) 'why_statement': _why.text.trim(),
      'deadline': _deadline.text.trim(),
      if (target != null) 'target_deadline': target,
      'availability': {
        'days': _weekDays.where(_days.contains).toList(),
        'minutes_per_day': int.tryParse(_minutes.text.trim()) ?? 60,
      },
    };
  }

  Map<String, dynamic> _updateBody() {
    final title = emptyToNull(_title.text);
    final why = emptyToNull(_why.text);
    final outcome = emptyToNull(_outcome.text);
    final target = emptyToNull(_targetDeadline.text);
    final criteria = _successCriteria.text
        .split(',')
        .map((value) => value.trim())
        .where((value) => value.isNotEmpty)
        .toList();
    final body = <String, dynamic>{};
    if (title != null) body['title'] = title;
    if (why != null) body['why_statement'] = why;
    if (outcome != null) body['outcome'] = outcome;
    if (criteria.isNotEmpty) body['success_criteria'] = criteria;
    if (target != null) body['target_deadline'] = target;
    if (_days.isNotEmpty) {
      body['availability'] = {
        'days': _weekDays.where(_days.contains).toList(),
        'minutes_per_day': int.tryParse(_minutes.text.trim()) ?? 60,
      };
    }
    return body;
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(12),
      children: [
        SectionCard(
          title: 'Goal form',
          subtitle: 'POST /goals · PATCH /goals/{id}',
          children: [
            AppTextField(controller: _title, label: 'Title'),
            AppTextField(controller: _why, label: 'Why statement'),
            AppTextField(
              controller: _outcome,
              label: 'Outcome (PATCH only)',
              hint: 'measurable result of the goal',
            ),
            AppTextField(
              controller: _successCriteria,
              label: 'Success criteria (PATCH only, comma separated)',
            ),
            AppTextField(
              controller: _deadline,
              label: 'Deadline (yyyy-MM-dd)',
              hint: 'must be in the future',
              suffixIcon: IconButton(
                icon: const Icon(Icons.calendar_today, size: 18),
                onPressed: () async {
                  final picked = await pickDate(
                    context,
                    current: _deadline.text,
                  );
                  if (picked != null) _deadline.text = picked;
                },
              ),
            ),
            AppTextField(
              controller: _targetDeadline,
              label: 'Target deadline (optional)',
              suffixIcon: IconButton(
                icon: const Icon(Icons.calendar_today, size: 18),
                onPressed: () async {
                  final picked = await pickDate(
                    context,
                    current: _targetDeadline.text,
                  );
                  if (picked != null) _targetDeadline.text = picked;
                },
              ),
            ),
            AppTextField(
              controller: _minutes,
              label: 'Minutes per day',
              keyboardType: TextInputType.number,
            ),
            const SizedBox(height: 4),
            Text('Available days', style: Theme.of(context).textTheme.labelLarge),
            const SizedBox(height: 6),
            Wrap(
              spacing: 6,
              children: [
                for (final day in _weekDays)
                  FilterChip(
                    label: Text(day),
                    selected: _days.contains(day),
                    onSelected: (selected) {
                      setState(() {
                        if (selected) {
                          _days.add(day);
                        } else {
                          _days.remove(day);
                        }
                      });
                    },
                  ),
              ],
            ),
          ],
        ),
        SectionCard(
          title: 'Goal requests',
          children: [
            AppTextField(
              controller: _goalId,
              label: 'Goal id',
              hint: 'UUID',
            ),
            const SizedBox(height: 4),
            WrapActions(
              children: [
                FilledButton(
                  onPressed: busy
                      ? null
                      : () => runApi(
                            () => widget.api.createGoal(_createBody()),
                          ),
                  child: const Text('Create goal'),
                ),
                FilledButton.tonal(
                  onPressed:
                      busy ? null : () => runApi(widget.api.listGoals),
                  child: const Text('List goals'),
                ),
                OutlinedButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requireId(_goalId, 'Goal id');
                          if (id == null) return;
                          runApi(() => widget.api.getGoal(id));
                        },
                  child: const Text('Get goal'),
                ),
                OutlinedButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requireId(_goalId, 'Goal id');
                          if (id == null) return;
                          runApi(
                            () => widget.api.updateGoal(id, _updateBody()),
                          );
                        },
                  child: const Text('Patch goal'),
                ),
                OutlinedButton(
                  onPressed: busy
                      ? null
                      : () async {
                          final id = _requireId(_goalId, 'Goal id');
                          if (id == null) return;
                          final confirmed = await showDialog<bool>(
                            context: context,
                            builder: (context) => AlertDialog(
                              title: const Text('Redecompose goal?'),
                              content: const Text(
                                'This deletes the current modules, steps and activities '
                                'and generates new objective-driven modules. It is blocked '
                                'if any activity is completed.',
                              ),
                              actions: [
                                TextButton(
                                  onPressed: () =>
                                      Navigator.of(context).pop(false),
                                  child: const Text('Cancel'),
                                ),
                                FilledButton(
                                  onPressed: () =>
                                      Navigator.of(context).pop(true),
                                  child: const Text('Redecompose'),
                                ),
                              ],
                            ),
                          );
                          if (confirmed == true) {
                            await runApi(
                              () => widget.api.redecomposeGoal(id),
                            );
                          }
                        },
                  child: const Text('Redecompose goal'),
                ),
                OutlinedButton(
                  onPressed: busy
                      ? null
                      : () async {
                          final id = _requireId(_goalId, 'Goal id');
                          if (id == null) return;
                          await runApi(() => widget.api.deleteGoal(id));
                        },
                  child: const Text('Delete goal'),
                ),
              ],
            ),
          ],
        ),
        SectionCard(
          title: 'Módulos',
          subtitle:
              'GET /goals/{id}/books · GET /books/{id}',
          children: [
            AppTextField(
              controller: _bookId,
              label: 'Módulo id',
              hint: 'UUID',
            ),
            const SizedBox(height: 4),
            WrapActions(
              children: [
                FilledButton.tonal(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requireId(_goalId, 'Goal id');
                          if (id == null) return;
                          runApi(() => widget.api.listGoalBooks(id));
                        },
                  child: const Text('Listar módulos'),
                ),
                OutlinedButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requireId(_bookId, 'Módulo id');
                          if (id == null) return;
                          runApi(() => widget.api.getBook(id));
                        },
                  child: const Text('Ver módulo'),
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
