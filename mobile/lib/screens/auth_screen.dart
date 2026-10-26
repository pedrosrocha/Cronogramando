import 'package:flutter/material.dart';

import '../api_client.dart';
import '../widgets.dart';

class AuthScreen extends StatefulWidget {
  const AuthScreen({super.key, required this.api});

  final ApiClient api;

  @override
  State<AuthScreen> createState() => _AuthScreenState();
}

class _AuthScreenState extends State<AuthScreen> with ApiRunner<AuthScreen> {
  final _baseUrl = TextEditingController();
  final _email = TextEditingController();
  final _password = TextEditingController();
  final _language = TextEditingController(text: 'pt-BR');

  @override
  void initState() {
    super.initState();
    _baseUrl.text = widget.api.baseUrl;
  }

  @override
  void dispose() {
    _baseUrl.dispose();
    _email.dispose();
    _password.dispose();
    _language.dispose();
    super.dispose();
  }

  void _applyBaseUrl() {
    widget.api.baseUrl = _baseUrl.text;
    _baseUrl.text = widget.api.baseUrl;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Base URL: ${widget.api.baseUrl}')),
    );
  }

  @override
  Widget build(BuildContext context) {
    final session = widget.api.currentUser;
    return ListView(
      padding: const EdgeInsets.all(12),
      children: [
        SectionCard(
          title: 'Connection',
          subtitle: 'POST /auth/* and GET /auth/me',
          children: [
            AppTextField(
              controller: _baseUrl,
              label: 'API base URL',
              hint: defaultApiBaseUrl,
              onSubmitted: (_) => _applyBaseUrl(),
            ),
            WrapActions(
              children: [
                FilledButton(
                  onPressed: _applyBaseUrl,
                  child: const Text('Apply base URL'),
                ),
              ],
            ),
          ],
        ),
        SectionCard(
          title: 'Credentials',
          children: [
            AppTextField(
              controller: _email,
              label: 'Email',
              keyboardType: TextInputType.emailAddress,
            ),
            AppTextField(
              controller: _password,
              label: 'Password',
              hint: 'at least 8 characters',
            ),
            AppTextField(
              controller: _language,
              label: 'Preferred language',
              hint: 'pt-BR',
            ),
          ],
        ),
        SectionCard(
          title: 'Auth requests',
          children: [
            WrapActions(
              children: [
                FilledButton(
                  onPressed: busy
                      ? null
                      : () => runApi(() => widget.api.register(
                            email: _email.text.trim(),
                            password: _password.text,
                            preferredLanguage: _language.text,
                          )),
                  child: const Text('Register'),
                ),
                FilledButton.tonal(
                  onPressed: busy
                      ? null
                      : () => runApi(() => widget.api.login(
                            email: _email.text.trim(),
                            password: _password.text,
                          )),
                  child: const Text('Login'),
                ),
                OutlinedButton(
                  onPressed: busy ? null : () => runApi(widget.api.refresh),
                  child: const Text('Refresh token'),
                ),
                OutlinedButton(
                  onPressed: busy ? null : () => runApi(widget.api.me),
                  child: const Text('GET /auth/me'),
                ),
                TextButton(
                  onPressed: () {
                    widget.api.clearSession();
                    setState(() {
                      result = null;
                      errorMessage = null;
                    });
                  },
                  child: const Text('Clear session'),
                ),
              ],
            ),
          ],
        ),
        SectionCard(
          title: 'Session',
          children: [
            SelectableText(
              widget.api.hasSession
                  ? 'Authenticated${session == null ? '' : ' as ${session['email']}'}'
                  : 'No access token stored',
            ),
            if (widget.api.accessToken != null) ...[
              const SizedBox(height: 8),
              Text(
                'Access token',
                style: Theme.of(context).textTheme.labelMedium,
              ),
              SelectableText(
                widget.api.accessToken!,
                style: const TextStyle(fontFamily: 'monospace', fontSize: 11),
              ),
            ],
            if (session != null) ...[
              const SizedBox(height: 8),
              Text(
                'Current user',
                style: Theme.of(context).textTheme.labelMedium,
              ),
              SelectableText(
                prettyJson(session),
                style: const TextStyle(fontFamily: 'monospace', fontSize: 11),
              ),
            ],
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
