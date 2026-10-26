import 'package:flutter/material.dart';

import 'mock_data.dart';
import 'prototype_widgets.dart';

class RoadmapPage extends StatelessWidget {
  const RoadmapPage({
    super.key,
    required this.goal,
    required this.completedActivityIds,
    required this.isBookUnlocked,
    required this.onOpenStep,
    required this.onPreviewCelebration,
  });

  final MockGoal goal;
  final Set<String> completedActivityIds;
  final bool Function(MockBook book) isBookUnlocked;
  final void Function(MockBook book, MockStep step) onOpenStep;
  final void Function(MockBook book) onPreviewCelebration;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final total = goal.totalActivities;
    final completed = completedActivityIds.length;
    final daysLeft =
        goal.deadline.difference(DateUtils.dateOnly(DateTime.now())).inDays;

    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 28),
      children: [
        Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            gradient: LinearGradient(
              colors: [
                theme.colorScheme.primaryContainer,
                theme.colorScheme.tertiaryContainer,
              ],
            ),
            borderRadius: BorderRadius.circular(18),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                'Roadmap',
                style: theme.textTheme.labelLarge?.copyWith(
                  color: theme.colorScheme.primary,
                  fontWeight: FontWeight.w700,
                ),
              ),
              const SizedBox(height: 6),
              Text(
                goal.title,
                style: theme.textTheme.headlineSmall?.copyWith(
                  fontWeight: FontWeight.w700,
                ),
              ),
              const SizedBox(height: 6),
              Text(
                goal.why,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onPrimaryContainer,
                ),
              ),
              const SizedBox(height: 14),
              Row(
                children: [
                  Expanded(
                    child: LinearProgressIndicator(
                      value: total == 0 ? 0 : completed / total,
                      minHeight: 8,
                      borderRadius: BorderRadius.circular(999),
                    ),
                  ),
                  const SizedBox(width: 10),
                  Text(
                    '$completed/$total',
                    style: theme.textTheme.labelLarge?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 10),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  Chip(
                    avatar: const Icon(Icons.flag_outlined, size: 18),
                    label: Text('$daysLeft dias'),
                  ),
                  Chip(
                    avatar: const Icon(Icons.schedule, size: 18),
                    label: Text('${goal.minutesPerDay} min/dia'),
                  ),
                  Chip(
                    avatar: const Icon(Icons.menu_book_outlined, size: 18),
                    label: Text('${goal.books.length} módulos'),
                  ),
                ],
              ),
            ],
          ),
        ),
        const SizedBox(height: 20),
        Text(
          'Módulos',
          style: theme.textTheme.titleLarge?.copyWith(
            fontWeight: FontWeight.w700,
          ),
        ),
        const SizedBox(height: 4),
        Text(
          'Cada módulo só abre quando o anterior é concluído.',
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 10),
        for (var i = 0; i < goal.books.length; i++)
          _BookCard(
            book: goal.books[i],
            index: i,
            completedActivityIds: completedActivityIds,
            unlocked: isBookUnlocked(goal.books[i]),
            onOpenStep: onOpenStep,
            onPreviewCelebration: onPreviewCelebration,
          ),
      ],
    );
  }
}

class _BookCard extends StatelessWidget {
  const _BookCard({
    required this.book,
    required this.index,
    required this.completedActivityIds,
    required this.unlocked,
    required this.onOpenStep,
    required this.onPreviewCelebration,
  });

  final MockBook book;
  final int index;
  final Set<String> completedActivityIds;
  final bool unlocked;
  final void Function(MockBook book, MockStep step) onOpenStep;
  final void Function(MockBook book) onPreviewCelebration;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final activities = book.steps.expand((step) => step.activities).toList();
    final completed =
        activities.where((a) => completedActivityIds.contains(a.id)).length;
    final isComplete = activities.isNotEmpty && completed == activities.length;
    final status = isComplete
        ? 'concluído'
        : !unlocked
            ? 'bloqueado'
            : completed > 0
                ? 'em andamento'
                : 'disponível';
    final statusColor = isComplete
        ? theme.colorScheme.primary
        : !unlocked
            ? theme.colorScheme.onSurfaceVariant
            : theme.colorScheme.tertiary;

    return Card(
      margin: const EdgeInsets.only(bottom: 14),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                CircleAvatar(
                  radius: 18,
                  backgroundColor: unlocked
                      ? theme.colorScheme.primaryContainer
                      : theme.colorScheme.surfaceContainerHighest,
                  child: isComplete
                      ? Icon(
                          Icons.check,
                          size: 20,
                          color: theme.colorScheme.primary,
                        )
                      : Text(
                          '${index + 1}',
                          style: TextStyle(
                            fontWeight: FontWeight.w700,
                            color: unlocked
                                ? theme.colorScheme.primary
                                : theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        book.title,
                        style: theme.textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        status,
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: statusColor,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                    ],
                  ),
                ),
                IconButton(
                  tooltip: 'Pré-visualizar celebração',
                  onPressed: () => onPreviewCelebration(book),
                  icon: const Icon(Icons.celebration_outlined),
                ),
              ],
            ),
            const SizedBox(height: 6),
            Text(
              book.objective,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: LinearProgressIndicator(
                    value:
                        activities.isEmpty ? 0 : completed / activities.length,
                    minHeight: 6,
                    borderRadius: BorderRadius.circular(999),
                  ),
                ),
                const SizedBox(width: 10),
                Text(
                  '$completed/${activities.length}',
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            if (!unlocked)
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: theme.colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Row(
                  children: [
                    Icon(
                      Icons.lock_outline,
                      size: 18,
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        'Conclua o módulo anterior para desbloquear.',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                  ],
                ),
              )
            else
              Theme(
                data: theme.copyWith(dividerColor: Colors.transparent),
                child: ExpansionTile(
                  tilePadding: EdgeInsets.zero,
                  childrenPadding: const EdgeInsets.only(bottom: 4),
                  title: Text(
                    '${book.steps.length} etapas',
                    style: theme.textTheme.labelLarge,
                  ),
                  children: [
                    for (final step in book.steps)
                      ListTile(
                        dense: true,
                        contentPadding: const EdgeInsets.symmetric(
                          horizontal: 4,
                        ),
                        leading: Icon(
                          completedActivityIds.contains(
                            step.activities.first.id,
                          )
                              ? Icons.check_circle
                              : Icons.radio_button_unchecked,
                          size: 20,
                          color: completedActivityIds.contains(
                            step.activities.first.id,
                          )
                              ? theme.colorScheme.primary
                              : theme.colorScheme.onSurfaceVariant,
                        ),
                        title: Text(step.title),
                        subtitle: Text(
                          '${formatMinutes(step.durationMins)} · ${step.outcome}',
                          maxLines: 2,
                          overflow: TextOverflow.ellipsis,
                        ),
                        trailing: const Icon(Icons.chevron_right, size: 18),
                        onTap: () => onOpenStep(book, step),
                      ),
                  ],
                ),
              ),
          ],
        ),
      ),
    );
  }
}
