import 'package:flutter/material.dart';

import 'mock_data.dart';
import 'prototype_widgets.dart';

class PrototypeActivityPage extends StatefulWidget {
  const PrototypeActivityPage({
    super.key,
    required this.goal,
    required this.step,
    required this.activity,
    required this.initiallyCompleted,
    required this.onToggle,
    this.slot,
  });

  final MockGoal goal;
  final MockSlot? slot;
  final MockStep step;
  final MockActivity activity;
  final bool initiallyCompleted;
  final ValueChanged<bool> onToggle;

  @override
  State<PrototypeActivityPage> createState() => _PrototypeActivityPageState();
}

class _PrototypeActivityPageState extends State<PrototypeActivityPage> {
  late bool _completed;
  bool _showReflowed = true;
  int? _selectedOption;
  bool _quizChecked = false;
  final Set<int> _checkedTasks = {};

  @override
  void initState() {
    super.initState();
    _completed = widget.initiallyCompleted;
  }

  void _setCompleted(bool value) {
    setState(() => _completed = value);
    widget.onToggle(value);
  }

  Widget _header(BuildContext context) {
    final theme = Theme.of(context);
    final book = widget.goal.bookForStep(widget.step.id);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            Chip(
              avatar: Icon(
                activityKindIcon(widget.activity.kind),
                size: 18,
              ),
              label: Text(activityKindLabel(widget.activity.kind)),
            ),
            Chip(
              avatar: const Icon(Icons.schedule, size: 18),
              label: Text(formatMinutes(widget.activity.durationMins)),
            ),
            if (widget.slot != null)
              Chip(
                avatar: const Icon(Icons.today_outlined, size: 18),
                label: Text(widget.slot!.timeRangeLabel),
              )
            else
              const Chip(
                avatar: Icon(Icons.event_repeat, size: 18),
                label: Text('sem horário'),
              ),
          ],
        ),
        const SizedBox(height: 16),
        Text(
          widget.activity.title,
          style: theme.textTheme.headlineSmall?.copyWith(
            fontWeight: FontWeight.w700,
          ),
        ),
        const SizedBox(height: 8),
        Text(
          book == null
              ? widget.step.title
              : '${book.title} · ${widget.step.title}',
          style: theme.textTheme.bodyMedium?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 16),
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            color: theme.colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(12),
          ),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Icon(
                Icons.flag_outlined,
                size: 20,
                color: theme.colorScheme.primary,
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  widget.step.outcome,
                  style: theme.textTheme.bodyMedium,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _video(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        AspectRatio(
          aspectRatio: 16 / 9,
          child: Container(
            decoration: BoxDecoration(
              color: Colors.black87,
              borderRadius: BorderRadius.circular(16),
            ),
            child: Stack(
              alignment: Alignment.center,
              children: [
                Icon(
                  Icons.play_circle_fill,
                  size: 72,
                  color: Colors.white.withValues(alpha: 0.9),
                ),
                Positioned(
                  left: 12,
                  bottom: 12,
                  child: Text(
                    'IFrame do YouTube entra na Phase 09d',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: Colors.white70,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 12),
        if (widget.activity.description != null)
          Text(
            widget.activity.description!,
            style: theme.textTheme.bodyMedium,
          ),
        const SizedBox(height: 12),
        SelectableText(
          widget.activity.resourceUrl ?? 'sem URL fictícia',
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
            fontFamily: 'monospace',
          ),
        ),
        const SizedBox(height: 8),
        Text(
          'O player real usa o IFrame oficial e as legendas nativas do YouTube.',
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      ],
    );
  }

  Widget _pdf(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SegmentedButton<bool>(
          segments: const [
            ButtonSegment(
              value: true,
              icon: Icon(Icons.notes),
              label: Text('Texto reflowado'),
            ),
            ButtonSegment(
              value: false,
              icon: Icon(Icons.picture_as_pdf_outlined),
              label: Text('PDF original'),
            ),
          ],
          selected: {_showReflowed},
          onSelectionChanged: (selection) {
            setState(() => _showReflowed = selection.first);
          },
        ),
        const SizedBox(height: 14),
        AnimatedSwitcher(
          duration: const Duration(milliseconds: 200),
          child: _showReflowed
              ? Container(
                  key: const ValueKey('reflowed'),
                  width: double.infinity,
                  padding: const EdgeInsets.all(16),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(color: theme.colorScheme.outlineVariant),
                  ),
                  child: SelectableText(
                    widget.activity.pdfText ?? 'Texto fictício indisponível.',
                    style: const TextStyle(fontSize: 16, height: 1.55),
                  ),
                )
              : Container(
                  key: const ValueKey('original'),
                  width: double.infinity,
                  padding: const EdgeInsets.symmetric(
                    horizontal: 20,
                    vertical: 36,
                  ),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.surfaceContainerLow,
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(color: theme.colorScheme.outlineVariant),
                  ),
                  child: Column(
                    children: [
                      Icon(
                        Icons.picture_as_pdf_outlined,
                        size: 48,
                        color: theme.colorScheme.tertiary,
                      ),
                      const SizedBox(height: 12),
                      Text(
                        'Visualizador do PDF original',
                        style: theme.textTheme.titleMedium,
                      ),
                      const SizedBox(height: 6),
                      Text(
                        'A Phase 09d embute o PDF real aqui, como fallback da '
                        'extração.',
                        textAlign: TextAlign.center,
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                      const SizedBox(height: 12),
                      SelectableText(
                        widget.activity.resourceUrl ?? 'sem URL fictícia',
                        style: theme.textTheme.bodySmall?.copyWith(
                          fontFamily: 'monospace',
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            Icon(
              Icons.info_outline,
              size: 16,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 6),
            Expanded(
              child: Text(
                'O toggle é o fallback humano para extrações imperfeitas.',
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

  Widget _quiz(BuildContext context) {
    final theme = Theme.of(context);
    final options = widget.activity.quizOptions;
    final answer = widget.activity.quizAnswerIndex;
    final isCorrect = _quizChecked && _selectedOption == answer;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          widget.activity.quizQuestion ?? 'Pergunta fictícia',
          style: theme.textTheme.titleMedium,
        ),
        const SizedBox(height: 12),
        RadioGroup<int>(
          groupValue: _selectedOption,
          onChanged: (value) {
            if (_quizChecked) return;
            setState(() => _selectedOption = value);
          },
          child: Column(
            children: [
              for (var i = 0; i < options.length; i++)
                Card(
                  margin: const EdgeInsets.only(bottom: 8),
                  color: _quizChecked && i == answer
                      ? theme.colorScheme.primaryContainer
                      : null,
                  child: RadioListTile<int>(
                    value: i,
                    title: Text(options[i]),
                  ),
                ),
            ],
          ),
        ),
        const SizedBox(height: 4),
        FilledButton.tonal(
          onPressed: _selectedOption == null
              ? null
              : () => setState(() => _quizChecked = true),
          child: const Text('Conferir resposta'),
        ),
        if (_quizChecked) ...[
          const SizedBox(height: 12),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: isCorrect
                  ? theme.colorScheme.primaryContainer
                  : theme.colorScheme.errorContainer,
              borderRadius: BorderRadius.circular(12),
            ),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(
                  isCorrect ? Icons.check_circle : Icons.error_outline,
                  color: isCorrect
                      ? theme.colorScheme.onPrimaryContainer
                      : theme.colorScheme.onErrorContainer,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    isCorrect
                        ? 'Isso! Você aplicou a propriedade da soma.'
                        : 'Quase. A soma dos limites é 2 + 3 = 5.',
                    style: TextStyle(
                      color: isCorrect
                          ? theme.colorScheme.onPrimaryContainer
                          : theme.colorScheme.onErrorContainer,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ],
    );
  }

  Widget _task(BuildContext context) {
    final theme = Theme.of(context);
    final checklist = widget.activity.taskChecklist;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          checklist.isEmpty ? 'Checklist fictício' : 'Passos da tarefa',
          style: theme.textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        for (var i = 0; i < checklist.length; i++)
          CheckboxListTile(
            contentPadding: EdgeInsets.zero,
            value: _checkedTasks.contains(i),
            title: Text(checklist[i]),
            controlAffinity: ListTileControlAffinity.leading,
            onChanged: (value) {
              setState(() {
                if (value ?? false) {
                  _checkedTasks.add(i);
                } else {
                  _checkedTasks.remove(i);
                }
              });
            },
          ),
        if (checklist.isNotEmpty) ...[
          const SizedBox(height: 4),
          LinearProgressIndicator(
            value: _checkedTasks.length / checklist.length,
            borderRadius: BorderRadius.circular(999),
          ),
          const SizedBox(height: 6),
          Text(
            '${_checkedTasks.length} de ${checklist.length} passos',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 10),
        ],
        Text(
          'Tarefas genéricas existem para quando nenhum recurso online resolve '
          'o objetivo — por exemplo, medir uma janela real.',
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final content = switch (widget.activity.kind) {
      MockActivityKind.video => _video(context),
      MockActivityKind.pdf => _pdf(context),
      MockActivityKind.quiz => _quiz(context),
      MockActivityKind.task => _task(context),
    };
    return Scaffold(
      appBar: AppBar(
        title: const Text('Atividade'),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 8),
            child: TextButton.icon(
              onPressed: () => _setCompleted(!_completed),
              icon: Icon(
                _completed ? Icons.check_circle : Icons.check_circle_outline,
              ),
              label: Text(_completed ? 'Concluída' : 'Concluir'),
            ),
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(20, 20, 20, 32),
        children: [
          _header(context),
          const SizedBox(height: 20),
          content,
          const SizedBox(height: 28),
          FilledButton.icon(
            onPressed: () => _setCompleted(!_completed),
            icon: Icon(
              _completed ? Icons.undo : Icons.check,
            ),
            label: Padding(
              padding: const EdgeInsets.symmetric(vertical: 10),
              child: Text(
                _completed
                    ? 'Marcar como não concluída'
                    : 'Marcar como concluída',
              ),
            ),
          ),
          const SizedBox(height: 8),
          Center(
            child: Text(
              'Dados fictícios · Phase 08a',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
            ),
          ),
        ],
      ),
    );
  }
}
