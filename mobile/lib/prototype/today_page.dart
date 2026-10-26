import 'package:flutter/material.dart';

import 'mock_data.dart';
import 'prototype_widgets.dart';

class TodayPage extends StatelessWidget {
  const TodayPage({
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
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final today = DateUtils.dateOnly(DateTime.now());
    final slots = goal.slotsFor(today);
    final entries = <(MockSlot, MockStep, MockActivity)>[];
    for (final slot in slots) {
      final step = goal.stepById(slot.stepId);
      if (step == null) continue;
      for (final activity in step.activities) {
        entries.add((slot, step, activity));
      }
    }
    final completedToday = entries
        .where((entry) => completedActivityIds.contains(entry.$3.id))
        .length;
    final allDone = entries.isNotEmpty && completedToday == entries.length;
    final streak = goal.streakDays + (allDone ? 1 : 0);
    final daysLeft = goal.deadline.difference(today).inDays;
    final totalCompleted = completedActivityIds.length;

    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 28),
      children: [
        WhyBanner(why: goal.why),
        const SizedBox(height: 14),
        Row(
          children: [
            StreakChip(days: streak, animating: allDone),
            const Spacer(),
            Chip(
              avatar: const Icon(Icons.flag_outlined, size: 18),
              label: Text('$daysLeft dias restantes'),
            ),
          ],
        ),
        const SizedBox(height: 14),
        Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: theme.colorScheme.surfaceContainerLow,
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: theme.colorScheme.outlineVariant),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Expanded(
                    child: Text(
                      'Progresso de hoje',
                      style: theme.textTheme.titleMedium,
                    ),
                  ),
                  Text(
                    '$completedToday/${entries.length}',
                    style: theme.textTheme.titleMedium?.copyWith(
                      color: theme.colorScheme.primary,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 10),
              LinearProgressIndicator(
                value: entries.isEmpty ? 0 : completedToday / entries.length,
                minHeight: 8,
                borderRadius: BorderRadius.circular(999),
              ),
              const SizedBox(height: 8),
              Text(
                allDone
                    ? 'Dia completo! Sua sequência cresceu.'
                    : 'Comece pela vitória rápida e mantenha o ritmo.',
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 20),
        Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Hoje',
                    style: theme.textTheme.headlineSmall?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  Text(
                    _todayLabel(today),
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
            Text(
              '$totalCompleted/${goal.totalActivities} no plano',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
        const SizedBox(height: 10),
        if (entries.isEmpty)
          const EmptyDayCard(
            title: 'Dia livre',
            message: 'Aproveite para revisar ou apenas descansar.',
          )
        else
          for (final (slot, step, activity) in entries)
            _buildTile(context, slot, step, activity),
        const SizedBox(height: 12),
        if (entries.isNotEmpty)
          Row(
            children: [
              Icon(
                Icons.lightbulb_outline,
                size: 18,
                color: theme.colorScheme.tertiary,
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  'A primeira atividade de hoje é uma vitória rápida de 10 min.',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            ],
          ),
      ],
    );
  }

  Widget _buildTile(
    BuildContext context,
    MockSlot slot,
    MockStep step,
    MockActivity activity,
  ) {
    final book = goal.bookForStep(step.id);
    final locked = book != null && !isBookUnlocked(book);
    final completed = completedActivityIds.contains(activity.id);
    return PrototypeActivityTile(
      slot: slot,
      step: step,
      activity: activity,
      completed: completed,
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
        onOpenActivity(slot, step, activity);
      },
      onToggle: locked
          ? null
          : (value) => onToggleActivity(slot, step, activity, value),
    );
  }

  String _todayLabel(DateTime date) {
    const weekdays = [
      'segunda',
      'terça',
      'quarta',
      'quinta',
      'sexta',
      'sábado',
      'domingo',
    ];
    const months = [
      'jan',
      'fev',
      'mar',
      'abr',
      'mai',
      'jun',
      'jul',
      'ago',
      'set',
      'out',
      'nov',
      'dez',
    ];
    return '${weekdays[date.weekday - 1]}, ${date.day} de ${months[date.month - 1]}';
  }
}
