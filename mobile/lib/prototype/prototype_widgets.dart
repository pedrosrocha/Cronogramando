import 'package:flutter/material.dart';

import 'mock_data.dart';

IconData activityKindIcon(MockActivityKind kind) {
  return switch (kind) {
    MockActivityKind.video => Icons.play_circle_outline,
    MockActivityKind.pdf => Icons.menu_book_outlined,
    MockActivityKind.quiz => Icons.quiz_outlined,
    MockActivityKind.task => Icons.edit_note,
  };
}

String activityKindLabel(MockActivityKind kind) {
  return switch (kind) {
    MockActivityKind.video => 'vídeo',
    MockActivityKind.pdf => 'PDF',
    MockActivityKind.quiz => 'quiz',
    MockActivityKind.task => 'tarefa',
  };
}

Color activityKindColor(BuildContext context, MockActivityKind kind) {
  final scheme = Theme.of(context).colorScheme;
  return switch (kind) {
    MockActivityKind.video => scheme.error,
    MockActivityKind.pdf => scheme.tertiary,
    MockActivityKind.quiz => scheme.secondary,
    MockActivityKind.task => scheme.primary,
  };
}

String formatMinutes(int minutes) {
  if (minutes < 60) return '$minutes min';
  final hours = minutes ~/ 60;
  final rest = minutes % 60;
  return rest == 0 ? '${hours}h' : '${hours}h ${rest}min';
}

class WhyBanner extends StatelessWidget {
  const WhyBanner({super.key, required this.why});

  final String why;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      decoration: BoxDecoration(
        gradient: LinearGradient(
          colors: [scheme.primaryContainer, scheme.secondaryContainer],
        ),
        borderRadius: BorderRadius.circular(16),
      ),
      padding: const EdgeInsets.all(16),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.favorite, color: scheme.primary, size: 22),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Seu porquê',
                  style: Theme.of(context).textTheme.labelLarge?.copyWith(
                        color: scheme.primary,
                        fontWeight: FontWeight.w700,
                      ),
                ),
                const SizedBox(height: 4),
                Text(
                  why,
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        color: scheme.onPrimaryContainer,
                      ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class StreakChip extends StatelessWidget {
  const StreakChip({super.key, required this.days, this.animating = false});

  final int days;
  final bool animating;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return AnimatedContainer(
      duration: const Duration(milliseconds: 300),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: animating ? scheme.primary : scheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(
            Icons.local_fire_department,
            size: 20,
            color: animating ? scheme.onPrimary : Colors.orange.shade700,
          ),
          const SizedBox(width: 6),
          Text(
            '$days ${days == 1 ? 'dia' : 'dias'}',
            style: Theme.of(context).textTheme.labelLarge?.copyWith(
                  color: animating ? scheme.onPrimary : scheme.onSurface,
                  fontWeight: FontWeight.w700,
                ),
          ),
        ],
      ),
    );
  }
}

class PrototypeActivityTile extends StatelessWidget {
  const PrototypeActivityTile({
    super.key,
    required this.slot,
    required this.step,
    required this.activity,
    required this.completed,
    required this.locked,
    required this.onTap,
    this.onToggle,
  });

  final MockSlot slot;
  final MockStep step;
  final MockActivity activity;
  final bool completed;
  final bool locked;
  final VoidCallback onTap;
  final ValueChanged<bool>? onToggle;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final colour = activityKindColor(context, activity.kind);
    return Card(
      margin: const EdgeInsets.only(bottom: 10),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(12, 12, 4, 12),
          child: Row(
            children: [
              Container(
                width: 46,
                height: 46,
                decoration: BoxDecoration(
                  color: colour.withValues(alpha: 0.12),
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Icon(
                  locked ? Icons.lock_outline : activityKindIcon(activity.kind),
                  color: locked ? scheme.onSurfaceVariant : colour,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      activity.title,
                      style: theme.textTheme.titleSmall?.copyWith(
                        decoration:
                            completed ? TextDecoration.lineThrough : null,
                        color: completed
                            ? scheme.onSurfaceVariant
                            : scheme.onSurface,
                      ),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '${slot.timeRangeLabel} · '
                      '${formatMinutes(activity.durationMins)} · '
                      '${activityKindLabel(activity.kind)}',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: scheme.onSurfaceVariant,
                      ),
                    ),
                    if (locked) ...[
                      const SizedBox(height: 2),
                      Text(
                        'Conclua o módulo anterior para desbloquear',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: scheme.tertiary,
                        ),
                      ),
                    ],
                  ],
                ),
              ),
              Checkbox(
                value: completed,
                onChanged: locked || onToggle == null
                    ? null
                    : (value) => onToggle!(value ?? false),
                shape: const CircleBorder(),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class EmptyDayCard extends StatelessWidget {
  const EmptyDayCard({super.key, required this.title, required this.message});

  final String title;
  final String message;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          children: [
            Icon(
              Icons.park_outlined,
              size: 36,
              color: theme.colorScheme.primary,
            ),
            const SizedBox(height: 10),
            Text(title, style: theme.textTheme.titleMedium),
            const SizedBox(height: 4),
            Text(
              message,
              textAlign: TextAlign.center,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
