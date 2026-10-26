import 'package:flutter/material.dart';

import '../api_client.dart';
import '../widgets.dart';

const _choices = ['stay_on_deadline', 'push_deadline'];

class ReconciliationScreen extends StatefulWidget {
  const ReconciliationScreen({super.key, required this.api});

  final ApiClient api;

  @override
  State<ReconciliationScreen> createState() => _ReconciliationScreenState();
}

class _ReconciliationScreenState extends State<ReconciliationScreen>
    with ApiRunner<ReconciliationScreen> {
  final _bookId = TextEditingController();
  String _choice = _choices.first;

  @override
  void dispose() {
    _bookId.dispose();
    super.dispose();
  }

  String? _requiredBookId() {
    final value = emptyToNull(_bookId.text);
    if (value == null) {
      setState(() {
        errorMessage = 'Módulo id é obrigatório';
        result = null;
      });
    }
    return value;
  }

  @override
  Widget build(BuildContext context) {
    final api = widget.api;
    return ListView(
      padding: const EdgeInsets.all(12),
      children: [
        SectionCard(
          title: 'Módulo id',
          subtitle:
              'POST /books/{id}/complete · POST /books/{id}/reconcile-deadline',
          children: [
            AppTextField(
              controller: _bookId,
              label: 'Módulo id',
              hint: 'UUID',
            ),
            const SizedBox(height: 4),
            WrapActions(
              children: [
                FilledButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requiredBookId();
                          if (id == null) return;
                          runApi(() => api.completeBook(id));
                        },
                  child: const Text('Complete book'),
                ),
              ],
            ),
          ],
        ),
        SectionCard(
          title: 'Reconciliation choice',
          subtitle: 'Only after a completed book is marked "pending"',
          children: [
            DropdownButtonFormField<String>(
              initialValue: _choice,
              decoration: const InputDecoration(
                labelText: 'Choice',
                isDense: true,
                border: OutlineInputBorder(),
              ),
              items: [
                for (final choice in _choices)
                  DropdownMenuItem(value: choice, child: Text(choice)),
              ],
              onChanged: (value) {
                if (value != null) setState(() => _choice = value);
              },
            ),
            const SizedBox(height: 12),
            WrapActions(
              children: [
                FilledButton.tonal(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requiredBookId();
                          if (id == null) return;
                          runApi(() => api.reconcileDeadline(id, _choice));
                        },
                  child: const Text('Reconcile deadline'),
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
