import 'package:flutter/material.dart';

import '../api_client.dart';
import '../widgets.dart';

const _roles = ['user', 'admin'];

class AdminScreen extends StatefulWidget {
  const AdminScreen({super.key, required this.api});

  final ApiClient api;

  @override
  State<AdminScreen> createState() => _AdminScreenState();
}

class _AdminScreenState extends State<AdminScreen> with ApiRunner<AdminScreen> {
  final _userId = TextEditingController();
  final _language = TextEditingController(text: 'pt-BR');
  String _role = _roles.first;

  @override
  void dispose() {
    _userId.dispose();
    _language.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final api = widget.api;
    return ListView(
      padding: const EdgeInsets.all(12),
      children: [
        SectionCard(
          title: 'Admin requests',
          subtitle: 'GET /admin/users · PATCH /admin/users/{id}',
          children: [
            AppTextField(
              controller: _userId,
              label: 'User id',
              hint: 'UUID',
            ),
            DropdownButtonFormField<String>(
              initialValue: _role,
              decoration: const InputDecoration(
                labelText: 'Role',
                isDense: true,
                border: OutlineInputBorder(),
              ),
              items: [
                for (final role in _roles)
                  DropdownMenuItem(value: role, child: Text(role)),
              ],
              onChanged: (value) {
                if (value != null) setState(() => _role = value);
              },
            ),
            const SizedBox(height: 10),
            AppTextField(
              controller: _language,
              label: 'Preferred language',
              hint: 'pt-BR',
            ),
            const SizedBox(height: 4),
            WrapActions(
              children: [
                FilledButton.tonal(
                  onPressed: busy ? null : () => runApi(api.listUsers),
                  child: const Text('List users'),
                ),
                FilledButton(
                  onPressed: busy
                      ? null
                      : () {
                          final id = emptyToNull(_userId.text);
                          if (id == null) {
                            setState(() {
                              errorMessage = 'User id is required';
                              result = null;
                            });
                            return;
                          }
                          runApi(
                            () => api.updateUser(
                              id,
                              role: _role,
                              preferredLanguage: _language.text,
                            ),
                          );
                        },
                  child: const Text('Patch user'),
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
