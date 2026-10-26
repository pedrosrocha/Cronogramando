import 'package:flutter/material.dart';

import '../api_client.dart';
import '../widgets.dart';

class ContentScreen extends StatefulWidget {
  const ContentScreen({super.key, required this.api});

  final ApiClient api;

  @override
  State<ContentScreen> createState() => _ContentScreenState();
}

class _ContentScreenState extends State<ContentScreen>
    with ApiRunner<ContentScreen> {
  final _activityId = TextEditingController();
  final _language = TextEditingController(text: 'pt-BR');
  final _reasons = TextEditingController();
  final _comment = TextEditingController();
  String _rating = 'helpful';

  @override
  void dispose() {
    _activityId.dispose();
    _language.dispose();
    _reasons.dispose();
    _comment.dispose();
    super.dispose();
  }

  String? _requiredActivityId() {
    final value = emptyToNull(_activityId.text);
    if (value == null) {
      setState(() {
        errorMessage = 'Activity id is required';
        result = null;
      });
    }
    return value;
  }

  @override
  Widget build(BuildContext context) {
    final api = widget.api;
    final markdown = result is Map ? result['markdown'] : null;
    final lessonBody = result is Map ? result['content_body'] : null;
    return ListView(
      padding: const EdgeInsets.all(12),
      children: [
        SectionCard(
          title: 'PDF content',
          subtitle:
              'GET /activities/{id}/content · GET /activities/{id}/translation',
          children: [
            AppTextField(
              controller: _activityId,
              label: 'Activity id',
              hint: 'UUID',
            ),
            AppTextField(
              controller: _language,
              label: 'Translation language',
              hint: 'pt-BR',
            ),
            const SizedBox(height: 4),
            WrapActions(
              children: [
                FilledButton.tonal(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requiredActivityId();
                          if (id == null) return;
                          runApi(() => api.getContent(id));
                        },
                  child: const Text('Get content'),
                ),
                FilledButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requiredActivityId();
                          if (id == null) return;
                          runApi(
                            () => api.getTranslation(
                              id,
                              language: emptyToNull(_language.text),
                            ),
                          );
                        },
                  child: const Text('Get translation'),
                ),
              ],
            ),
          ],
        ),
        if (markdown is String && markdown.isNotEmpty)
          SectionCard(
            title: 'Rendered markdown',
            children: [
              SelectableText(
                markdown,
                style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
              ),
            ],
          ),
        SectionCard(
          title: 'Content quality',
          subtitle:
              'GET /activities/{id} · POST /activities/{id}/feedback · GET /activities/{id}/trace',
          children: [
            DropdownButtonFormField<String>(
              initialValue: _rating,
              decoration: const InputDecoration(
                labelText: 'Rating',
                isDense: true,
                border: OutlineInputBorder(),
              ),
              items: const [
                DropdownMenuItem(value: 'helpful', child: Text('Helpful')),
                DropdownMenuItem(
                  value: 'not_helpful',
                  child: Text('Not helpful'),
                ),
              ],
              onChanged: (value) {
                if (value != null) setState(() => _rating = value);
              },
            ),
            AppTextField(
              controller: _reasons,
              label: 'Reasons (comma separated)',
              hint: 'too_long, unrelated, not_digitized',
            ),
            AppTextField(controller: _comment, label: 'Comment (optional)'),
            const SizedBox(height: 4),
            WrapActions(
              children: [
                FilledButton.tonal(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requiredActivityId();
                          if (id == null) return;
                          runApi(() => api.getActivity(id));
                        },
                  child: const Text('Load activity'),
                ),
                FilledButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requiredActivityId();
                          if (id == null) return;
                          runApi(
                            () => api.sendFeedback(
                              id,
                              rating: _rating,
                              reasons: _reasons.text
                                  .split(',')
                                  .map((value) => value.trim())
                                  .where((value) => value.isNotEmpty)
                                  .toList(),
                              comment: emptyToNull(_comment.text),
                            ),
                          );
                        },
                  child: const Text('Send feedback'),
                ),
                OutlinedButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = _requiredActivityId();
                          if (id == null) return;
                          runApi(() => api.getTrace(id));
                        },
                  child: const Text('Load trace'),
                ),
              ],
            ),
          ],
        ),
        if (lessonBody is String && lessonBody.isNotEmpty)
          SectionCard(
            title: 'Generated lesson',
            children: [
              SelectableText(
                lessonBody,
                style: const TextStyle(fontSize: 13, height: 1.4),
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
