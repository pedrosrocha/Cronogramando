import 'package:flutter/material.dart';

import '../api_client.dart';
import '../widgets.dart';

class ScheduleScreen extends StatefulWidget {
  const ScheduleScreen({super.key, required this.api});

  final ApiClient api;

  @override
  State<ScheduleScreen> createState() => _ScheduleScreenState();
}

class _ScheduleScreenState extends State<ScheduleScreen>
    with ApiRunner<ScheduleScreen> {
  final _date = TextEditingController();
  final _from = TextEditingController();
  final _to = TextEditingController();
  final _goalId = TextEditingController();
  final _slotId = TextEditingController();
  final _moveDate = TextEditingController();

  @override
  void dispose() {
    _date.dispose();
    _from.dispose();
    _to.dispose();
    _goalId.dispose();
    _slotId.dispose();
    _moveDate.dispose();
    super.dispose();
  }

  Widget _dateField(
    TextEditingController controller,
    String label,
  ) {
    return AppTextField(
      controller: controller,
      label: label,
      hint: 'yyyy-MM-dd',
      suffixIcon: IconButton(
        icon: const Icon(Icons.calendar_today, size: 18),
        onPressed: () async {
          final picked = await pickDate(context, current: controller.text);
          if (picked != null) controller.text = picked;
        },
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final api = widget.api;
    return ListView(
      padding: const EdgeInsets.all(12),
      children: [
        SectionCard(
          title: 'Schedule query',
          subtitle: 'GET /schedule?date= | ?goal_id=&from=&to=',
          children: [
            _dateField(_date, 'Date'),
            AppTextField(
              controller: _goalId,
              label: 'Goal id (optional)',
              hint: 'UUID',
            ),
            _dateField(_from, 'From'),
            _dateField(_to, 'To'),
            const SizedBox(height: 4),
            WrapActions(
              children: [
                FilledButton(
                  onPressed: busy
                      ? null
                      : () => runApi(
                            () => api.listSchedule(
                              date: emptyToNull(_date.text),
                              goalId: emptyToNull(_goalId.text),
                              from: emptyToNull(_from.text),
                              to: emptyToNull(_to.text),
                            ),
                          ),
                  child: const Text('List schedule'),
                ),
                TextButton(
                  onPressed: () {
                    _date.clear();
                    _from.clear();
                    _to.clear();
                    _goalId.clear();
                  },
                  child: const Text('Clear filters'),
                ),
              ],
            ),
          ],
        ),
        SectionCard(
          title: 'Move slot',
          subtitle: 'PATCH /schedule/{slot_id} (triggers re-fit)',
          children: [
            AppTextField(
              controller: _slotId,
              label: 'Slot id',
              hint: 'UUID',
            ),
            _dateField(_moveDate, 'New date'),
            const SizedBox(height: 4),
            WrapActions(
              children: [
                FilledButton.tonal(
                  onPressed: busy
                      ? null
                      : () {
                          final slotId = emptyToNull(_slotId.text);
                          final date = emptyToNull(_moveDate.text);
                          if (slotId == null || date == null) {
                            setState(() {
                              errorMessage = 'Slot id and new date are required';
                              result = null;
                            });
                            return;
                          }
                          runApi(() => api.moveSlot(slotId, date));
                        },
                  child: const Text('Move slot'),
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
