import 'package:flutter/material.dart';

import '../widgets.dart' show formatDate;
import 'mock_data.dart';

const _dayOptions = [
  (DateTime.monday, 'seg'),
  (DateTime.tuesday, 'ter'),
  (DateTime.wednesday, 'qua'),
  (DateTime.thursday, 'qui'),
  (DateTime.friday, 'sex'),
  (DateTime.saturday, 'sáb'),
  (DateTime.sunday, 'dom'),
];

/// Phase 08a onboarding: goal, deadline, availability and the emotional "why".
class PrototypeOnboardingPage extends StatefulWidget {
  const PrototypeOnboardingPage({
    super.key,
    required this.initialGoal,
    required this.onStart,
  });

  final MockGoal initialGoal;
  final ValueChanged<MockGoal> onStart;

  @override
  State<PrototypeOnboardingPage> createState() =>
      _PrototypeOnboardingPageState();
}

class _PrototypeOnboardingPageState extends State<PrototypeOnboardingPage> {
  late final TextEditingController _title;
  late final TextEditingController _why;
  late DateTime _deadline;
  late Set<int> _days;
  late double _minutes;

  @override
  void initState() {
    super.initState();
    _title = TextEditingController(text: widget.initialGoal.title);
    _why = TextEditingController(text: widget.initialGoal.why);
    _deadline = widget.initialGoal.deadline;
    _days = widget.initialGoal.availabilityDays.toSet();
    _minutes = widget.initialGoal.minutesPerDay.toDouble();
  }

  @override
  void dispose() {
    _title.dispose();
    _why.dispose();
    super.dispose();
  }

  Future<void> _pickDeadline() async {
    final today = DateUtils.dateOnly(DateTime.now());
    final picked = await showDatePicker(
      context: context,
      initialDate: _deadline,
      firstDate: today.add(const Duration(days: 1)),
      lastDate: today.add(const Duration(days: 730)),
      helpText: 'Escolha seu prazo',
    );
    if (picked != null) setState(() => _deadline = picked);
  }

  void _start() {
    final title = _title.text.trim().isEmpty
        ? widget.initialGoal.title
        : _title.text.trim();
    final why =
        _why.text.trim().isEmpty ? widget.initialGoal.why : _why.text.trim();
    widget.onStart(
      widget.initialGoal.copyWith(
        title: title,
        why: why,
        deadline: _deadline,
        availabilityDays: _days.isEmpty
            ? widget.initialGoal.availabilityDays
            : _days.toList(),
        minutesPerDay: _minutes.round(),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final daysLeft =
        _deadline.difference(DateUtils.dateOnly(DateTime.now())).inDays;
    return ListView(
      padding: const EdgeInsets.fromLTRB(20, 24, 20, 32),
      children: [
        Icon(
          Icons.auto_awesome,
          size: 42,
          color: theme.colorScheme.primary,
        ),
        const SizedBox(height: 16),
        Text(
          'Vamos montar seu cronograma',
          style: theme.textTheme.headlineSmall?.copyWith(
            fontWeight: FontWeight.w700,
          ),
        ),
        const SizedBox(height: 8),
        Text(
          'O protótipo usa dados fictícios para validar a experiência antes do '
          'app real.',
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 24),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('O que você quer aprender?',
                    style: theme.textTheme.titleMedium),
                const SizedBox(height: 10),
                TextField(
                  controller: _title,
                  textCapitalization: TextCapitalization.sentences,
                  decoration: const InputDecoration(
                    labelText: 'Meta',
                    hintText: 'Ex.: Passar em Cálculo II',
                    border: OutlineInputBorder(),
                    isDense: true,
                  ),
                ),
                const SizedBox(height: 16),
                Text('Por que isso importa?',
                    style: theme.textTheme.titleMedium),
                const SizedBox(height: 10),
                TextField(
                  controller: _why,
                  maxLines: 3,
                  textCapitalization: TextCapitalization.sentences,
                  decoration: const InputDecoration(
                    labelText: 'Seu porquê',
                    hintText: 'Esse lembrete aparece todos os dias.',
                    border: OutlineInputBorder(),
                    alignLabelWithHint: true,
                  ),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 12),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text('Quando você quer terminar?',
                          style: theme.textTheme.titleMedium),
                    ),
                    TextButton.icon(
                      onPressed: _pickDeadline,
                      icon: const Icon(Icons.calendar_today, size: 18),
                      label: const Text('Alterar'),
                    ),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  '${formatDate(_deadline)} · $daysLeft dias',
                  style: theme.textTheme.headlineSmall?.copyWith(
                    color: theme.colorScheme.primary,
                  ),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 12),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Dias disponíveis', style: theme.textTheme.titleMedium),
                const SizedBox(height: 10),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    for (final (weekday, label) in _dayOptions)
                      FilterChip(
                        label: Text(label),
                        selected: _days.contains(weekday),
                        onSelected: (selected) {
                          setState(() {
                            if (selected) {
                              _days.add(weekday);
                            } else {
                              _days.remove(weekday);
                            }
                          });
                        },
                      ),
                  ],
                ),
                const SizedBox(height: 20),
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        'Tempo por dia',
                        style: theme.textTheme.titleMedium,
                      ),
                    ),
                    Text(
                      '${_minutes.round()} min',
                      style: theme.textTheme.titleMedium?.copyWith(
                        color: theme.colorScheme.primary,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ],
                ),
                Slider(
                  value: _minutes,
                  min: 15,
                  max: 180,
                  divisions: 11,
                  label: '${_minutes.round()} min',
                  onChanged: (value) => setState(() => _minutes = value),
                ),
                Text(
                  'Usamos a menor disponibilidade para manter o plano realista.',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 20),
        FilledButton.icon(
          onPressed: _start,
          icon: const Icon(Icons.arrow_forward),
          label: const Padding(
            padding: EdgeInsets.symmetric(vertical: 12),
            child: Text('Criar meu plano'),
          ),
        ),
        const SizedBox(height: 8),
        Center(
          child: Text(
            'Fase 08a · protótipo sem backend',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      ],
    );
  }
}
