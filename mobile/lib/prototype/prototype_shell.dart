import 'package:flutter/material.dart';

import 'activity_detail_page.dart';
import 'calendar_page.dart';
import 'mock_data.dart';
import 'onboarding_page.dart';
import 'roadmap_page.dart';
import 'today_page.dart';

/// Phase 08a clickable prototype.
///
/// Everything is mocked and local: onboarding, today, calendar, roadmap and the
/// four activity detail variants (video, reflowed PDF, quiz and task).
class PrototypeShell extends StatefulWidget {
  const PrototypeShell({super.key, this.loadOverride});

  /// Test seam: lets widget tests provide the mock data without touching the
  /// asynchronous asset bundle.
  final Future<MockGoal>? loadOverride;

  @override
  State<PrototypeShell> createState() => _PrototypeShellState();
}

class _PrototypeShellState extends State<PrototypeShell> {
  static const _repository = PrototypeRepository();

  late Future<MockGoal> _loading;
  MockGoal? _goal;
  bool _started = false;
  int _tab = 0;
  final Set<String> _completedActivities = {};
  final Set<String> _celebratedBooks = {};

  @override
  void initState() {
    super.initState();
    _loading = widget.loadOverride ?? _repository.load();
  }

  bool _isBookCompleted(MockBook book) {
    final activities = book.steps.expand((step) => step.activities).toList();
    return activities.isNotEmpty &&
        activities
            .every((activity) => _completedActivities.contains(activity.id));
  }

  bool _isBookUnlocked(MockBook book) {
    final goal = _goal;
    if (goal == null) return false;
    final index = goal.books.indexWhere((item) => item.id == book.id);
    if (index <= 0) return true;
    return _isBookCompleted(goal.books[index - 1]);
  }

  void _toggleActivity(MockActivity activity, bool completed) {
    final goal = _goal;
    if (goal == null) return;
    MockBook? book;
    for (final candidate in goal.books) {
      if (candidate.steps
          .expand((step) => step.activities)
          .any((item) => item.id == activity.id)) {
        book = candidate;
        break;
      }
    }
    final wasComplete = book == null ? false : _isBookCompleted(book);
    setState(() {
      if (completed) {
        _completedActivities.add(activity.id);
      } else {
        _completedActivities.remove(activity.id);
      }
    });
    if (book == null) return;
    final isComplete = _isBookCompleted(book);
    if (isComplete && !wasComplete && !_celebratedBooks.contains(book.id)) {
      _celebratedBooks.add(book.id);
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _showCelebration(book!, preview: false);
      });
    }
    if (!isComplete) {
      _celebratedBooks.remove(book.id);
    }
  }

  void _openActivity(
    MockSlot? slot,
    MockStep step,
    MockActivity activity,
  ) {
    final goal = _goal;
    if (goal == null) return;
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => PrototypeActivityPage(
          goal: goal,
          slot: slot,
          step: step,
          activity: activity,
          initiallyCompleted: _completedActivities.contains(activity.id),
          onToggle: (value) => _toggleActivity(activity, value),
        ),
      ),
    );
  }

  void _openStep(MockBook book, MockStep step) {
    if (!_isBookUnlocked(book) || step.activities.isEmpty) return;
    final goal = _goal;
    MockSlot? slot;
    if (goal != null) {
      for (final candidate in goal.slots) {
        if (candidate.stepId == step.id) {
          slot = candidate;
          break;
        }
      }
    }
    _openActivity(slot, step, step.activities.first);
  }

  void _showCelebration(MockBook book, {required bool preview}) {
    final goal = _goal;
    if (goal == null) return;
    final index = goal.books.indexWhere((item) => item.id == book.id);
    final next = index >= 0 && index + 1 < goal.books.length
        ? goal.books[index + 1]
        : null;
    showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        icon: Icon(
          Icons.celebration,
          size: 42,
          color: Theme.of(context).colorScheme.primary,
        ),
        title: Text(preview ? 'Prévia da celebração' : 'Módulo concluído!'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Você concluiu "${book.title}".',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Text(
              next == null
                  ? 'Esse era o último módulo do plano. Hora de olhar o caminho '
                      'percorrido!'
                  : '"${next.title}" foi desbloqueado.',
            ),
            const SizedBox(height: 16),
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Theme.of(context).colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(10),
              ),
              child: Text(
                'Placeholder da Phase 08a: a Phase 10b troca este diálogo por '
                'confete, som e um resumo do módulo.',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
              ),
            ),
          ],
        ),
        actions: [
          FilledButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Continuar'),
          ),
        ],
      ),
    );
  }

  void _reset() {
    setState(() {
      _started = false;
      _tab = 0;
      _completedActivities.clear();
      _celebratedBooks.clear();
      _goal = null;
      _loading = _repository.load();
    });
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<MockGoal>(
      future: _loading,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return Center(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(Icons.error_outline, size: 40),
                  const SizedBox(height: 10),
                  const Text('Não foi possível carregar os dados fictícios.'),
                  const SizedBox(height: 4),
                  Text(
                    '${snapshot.error}',
                    textAlign: TextAlign.center,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
            ),
          );
        }
        if (!snapshot.hasData) {
          return const Center(child: CircularProgressIndicator());
        }
        _goal ??= snapshot.data;
        final goal = _goal!;
        if (!_started) {
          return PrototypeOnboardingPage(
            initialGoal: goal,
            onStart: (value) {
              setState(() {
                _goal = value;
                _started = true;
                _tab = 0;
              });
            },
          );
        }
        return Scaffold(
          body: Column(
            children: [
              _PrototypeRibbon(onReset: _reset),
              Expanded(
                child: IndexedStack(
                  index: _tab,
                  children: [
                    TodayPage(
                      goal: goal,
                      completedActivityIds: _completedActivities,
                      isBookUnlocked: _isBookUnlocked,
                      onOpenActivity: (slot, step, activity) =>
                          _openActivity(slot, step, activity),
                      onToggleActivity: (slot, step, activity, completed) =>
                          _toggleActivity(activity, completed),
                    ),
                    CalendarPage(
                      goal: goal,
                      completedActivityIds: _completedActivities,
                      isBookUnlocked: _isBookUnlocked,
                      onOpenActivity: (slot, step, activity) =>
                          _openActivity(slot, step, activity),
                      onToggleActivity: (slot, step, activity, completed) =>
                          _toggleActivity(activity, completed),
                    ),
                    RoadmapPage(
                      goal: goal,
                      completedActivityIds: _completedActivities,
                      isBookUnlocked: _isBookUnlocked,
                      onOpenStep: _openStep,
                      onPreviewCelebration: (book) =>
                          _showCelebration(book, preview: true),
                    ),
                  ],
                ),
              ),
            ],
          ),
          bottomNavigationBar: NavigationBar(
            selectedIndex: _tab,
            onDestinationSelected: (index) => setState(() => _tab = index),
            destinations: const [
              NavigationDestination(
                icon: Icon(Icons.today_outlined),
                selectedIcon: Icon(Icons.today),
                label: 'Hoje',
              ),
              NavigationDestination(
                icon: Icon(Icons.calendar_month_outlined),
                selectedIcon: Icon(Icons.calendar_month),
                label: 'Calendário',
              ),
              NavigationDestination(
                icon: Icon(Icons.route_outlined),
                selectedIcon: Icon(Icons.route),
                label: 'Roadmap',
              ),
            ],
          ),
        );
      },
    );
  }
}

class _PrototypeRibbon extends StatelessWidget {
  const _PrototypeRibbon({required this.onReset});

  final VoidCallback onReset;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      color: theme.colorScheme.surfaceContainerHighest,
      padding: const EdgeInsets.only(left: 14),
      child: Row(
        children: [
          Icon(
            Icons.science_outlined,
            size: 16,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              'Protótipo 08a · dados fictícios',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          TextButton(
            onPressed: onReset,
            child: const Text('Reiniciar'),
          ),
        ],
      ),
    );
  }
}
