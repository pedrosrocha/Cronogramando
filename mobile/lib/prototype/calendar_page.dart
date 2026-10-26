import 'package:flutter/material.dart';
import 'package:intl/intl.dart';

import 'mock_data.dart';
import 'prototype_widgets.dart';

class CalendarPage extends StatefulWidget {
  const CalendarPage({
    super.key,
    required this.goal,
    required this.completedActivityIds,
    required this.isBookUnlocked,
    required this.onOpenActivity,
    required this.onToggleActivity,
  });

  final MockGoal goal;
  final Set<String> completedActivityIds;
  final bool Function(MockBook book) isBookUnlocked;
  final void Function(MockSlot slot, MockStep step, MockActivity activity)
      onOpenActivity;
  final void Function(
    MockSlot slot,
    MockStep step,
    MockActivity activity,
    bool completed,
  ) onToggleActivity;

  @override
  State<CalendarPage> createState() => _CalendarPageState();
}

class _CalendarPageState extends State<CalendarPage> {
  late DateTime _weekStart;
  late DateTime _selectedDay;

  @override
  void initState() {
    super.initState();
    final today = DateUtils.dateOnly(DateTime.now());
    _selectedDay = today;
    _weekStart = today.subtract(Duration(days: today.weekday - 1));
  }

  void _changeWeek(int delta) {
    final next = _weekStart.add(Duration(days: 7 * delta));
    setState(() {
      _weekStart = next;
      _selectedDay = next;
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final days = List.generate(
      7,
      (index) => DateUtils.dateOnly(_weekStart.add(Duration(days: index))),
    );
    final slots = widget.goal.slotsFor(_selectedDay);
    final today = DateUtils.dateOnly(DateTime.now());

    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 28),
      children: [
        Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Calendário',
                    style: theme.textTheme.headlineSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  Text(
                    '${DateFormat('d MMM', 'pt_BR').format(_weekStart)} – '
                    '${DateFormat('d MMM', 'pt_BR').format(days.last)}',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
            IconButton(
              tooltip: 'Semana anterior',
              onPressed: () => _changeWeek(-1),
              icon: const Icon(Icons.chevron_left),
            ),
            IconButton(
              tooltip: 'Próxima semana',
              onPressed: () => _changeWeek(1),
              icon: const Icon(Icons.chevron_right),
            ),
          ],
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            for (final day in days)
              Expanded(
                child: _DayCell(
                  day: day,
                  selected: DateUtils.isSameDay(day, _selectedDay),
                  isToday: DateUtils.isSameDay(day, today),
                  available: widget.goal.availabilityDays.contains(day.weekday),
                  activityCount: widget.goal.slotsFor(day).length,
                  onTap: () => setState(() => _selectedDay = day),
                ),
              ),
          ],
        ),
        const SizedBox(height: 20),
        Row(
          children: [
            Expanded(
              child: Text(
                DateUtils.isSameDay(_selectedDay, today)
                    ? 'Hoje'
                    : DateFormat("EEEE, d 'de' MMMM", 'pt_BR')
                        .format(_selectedDay),
                style: theme.textTheme.titleLarge?.copyWith(
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
            Chip(
              avatar: const Icon(Icons.timelapse, size: 18),
              label: Text('${slots.length} blocos'),
            ),
          ],
        ),
        const SizedBox(height: 10),
        if (slots.isEmpty)
          EmptyDayCard(
            title: DateUtils.isSameDay(_selectedDay, today)
                ? 'Nada agendado para hoje'
                : 'Dia sem atividades',
            message: widget.goal.availabilityDays.contains(_selectedDay.weekday)
                ? 'Você marcou este dia como disponível, mas não há blocos.'
                : 'Este dia não faz parte da sua disponibilidade.',
          )
        else
          for (final slot in slots) _buildSlotCard(context, slot),
        const SizedBox(height: 12),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(14),
            child: Row(
              children: [
                Icon(
                  Icons.event_available,
                  color: theme.colorScheme.primary,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    'Disponibilidade: ${widget.goal.availabilityDays.map(_weekdayLabel).join(', ')} · '
                    '${widget.goal.minutesPerDay} min/dia',
                    style: theme.textTheme.bodySmall,
                  ),
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildSlotCard(BuildContext context, MockSlot slot) {
    final step = widget.goal.stepById(slot.stepId);
    if (step == null) return const SizedBox.shrink();
    final activity = step.activities.first;
    final book = widget.goal.bookForStep(step.id);
    final locked = book != null && !widget.isBookUnlocked(book);
    return PrototypeActivityTile(
      slot: slot,
      step: step,
      activity: activity,
      completed: widget.completedActivityIds.contains(activity.id),
      locked: locked,
      onTap: () {
        if (locked) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                'Conclua "${book.title}" para desbloquear esta atividade.',
              ),
            ),
          );
          return;
        }
        widget.onOpenActivity(slot, step, activity);
      },
      onToggle: locked
          ? null
          : (value) => widget.onToggleActivity(slot, step, activity, value),
    );
  }

  String _weekdayLabel(int weekday) {
    const labels = ['seg', 'ter', 'qua', 'qui', 'sex', 'sáb', 'dom'];
    return labels[weekday - 1];
  }
}

class _DayCell extends StatelessWidget {
  const _DayCell({
    required this.day,
    required this.selected,
    required this.isToday,
    required this.available,
    required this.activityCount,
    required this.onTap,
  });

  final DateTime day;
  final bool selected;
  final bool isToday;
  final bool available;
  final int activityCount;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final foreground = selected ? scheme.onPrimary : scheme.onSurface;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 2),
      child: InkWell(
        borderRadius: BorderRadius.circular(14),
        onTap: onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 180),
          padding: const EdgeInsets.symmetric(vertical: 10),
          decoration: BoxDecoration(
            color: selected
                ? scheme.primary
                : available
                    ? scheme.surfaceContainerHighest
                    : scheme.surface,
            borderRadius: BorderRadius.circular(14),
            border: Border.all(
              color: isToday && !selected ? scheme.primary : Colors.transparent,
            ),
          ),
          child: Column(
            children: [
              Text(
                _weekday(day.weekday),
                style: Theme.of(context).textTheme.labelSmall?.copyWith(
                      color: selected
                          ? scheme.onPrimary.withValues(alpha: 0.8)
                          : scheme.onSurfaceVariant,
                    ),
              ),
              const SizedBox(height: 3),
              Text(
                '${day.day}',
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      color: foreground,
                      fontWeight: FontWeight.w700,
                    ),
              ),
              const SizedBox(height: 3),
              SizedBox(
                height: 6,
                child: activityCount == 0
                    ? const SizedBox.shrink()
                    : Row(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          for (var i = 0; i < activityCount.clamp(1, 3); i++)
                            Container(
                              width: 4,
                              height: 4,
                              margin: const EdgeInsets.symmetric(horizontal: 1),
                              decoration: BoxDecoration(
                                color: selected
                                    ? scheme.onPrimary
                                    : scheme.primary,
                                shape: BoxShape.circle,
                              ),
                            ),
                        ],
                      ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  String _weekday(int weekday) {
    const labels = ['S', 'T', 'Q', 'Q', 'S', 'S', 'D'];
    return labels[weekday - 1];
  }
}
