import 'package:flutter/material.dart';

import '../api_client.dart';
import '../widgets.dart';

const _methods = ['GET', 'POST', 'PATCH', 'PUT', 'DELETE'];

class ConsoleScreen extends StatefulWidget {
  const ConsoleScreen({super.key, required this.api});

  final ApiClient api;

  @override
  State<ConsoleScreen> createState() => _ConsoleScreenState();
}

class _ConsoleScreenState extends State<ConsoleScreen>
    with ApiRunner<ConsoleScreen> {
  final _path = TextEditingController(text: '/health');
  final _body = TextEditingController();
  String _method = 'GET';
  bool _authenticated = true;

  @override
  void dispose() {
    _path.dispose();
    _body.dispose();
    super.dispose();
  }

  Future<void> _send() async {
    final path = emptyToNull(_path.text);
    if (path == null) {
      setState(() {
        errorMessage = 'Path is required';
        result = null;
      });
      return;
    }
    try {
      final parsed = parseJsonObject(_body.text);
      final sendsBody = _method != 'GET' && _method != 'DELETE';
      await runApi(
        () => widget.api.request(
          _method,
          path,
          body: sendsBody && parsed.isNotEmpty ? parsed : null,
          authenticated: _authenticated,
        ),
      );
    } on FormatException catch (error) {
      setState(() {
        errorMessage = 'invalid JSON body: $error';
        result = null;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(12),
      children: [
        SectionCard(
          title: 'Raw request',
          subtitle: 'Escape hatch for any route not covered by a form',
          children: [
            DropdownButtonFormField<String>(
              initialValue: _method,
              decoration: const InputDecoration(
                labelText: 'Method',
                isDense: true,
                border: OutlineInputBorder(),
              ),
              items: [
                for (final method in _methods)
                  DropdownMenuItem(value: method, child: Text(method)),
              ],
              onChanged: (value) {
                if (value != null) setState(() => _method = value);
              },
            ),
            const SizedBox(height: 10),
            AppTextField(
              controller: _path,
              label: 'Path',
              hint: '/goals/00000000-0000-0000-0000-000000000000',
            ),
            AppTextField(
              controller: _body,
              label: 'JSON body',
              hint: '{"key": "value"}',
              maxLines: 8,
              keyboardType: TextInputType.multiline,
            ),
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('Send bearer token'),
              value: _authenticated,
              onChanged: (value) => setState(() => _authenticated = value),
            ),
            const SizedBox(height: 4),
            WrapActions(
              children: [
                FilledButton(
                  onPressed: busy ? null : _send,
                  child: const Text('Send request'),
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
