import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/date_symbol_data_local.dart';

import 'api_client.dart';
import 'prototype/prototype_shell.dart';
import 'screens/admin_screen.dart';
import 'screens/auth_screen.dart';
import 'screens/console_screen.dart';
import 'screens/content_screen.dart';
import 'screens/goals_screen.dart';
import 'screens/plan_screen.dart';
import 'screens/reconciliation_screen.dart';
import 'screens/schedule_screen.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await initializeDateFormatting('pt_BR');
  runApp(const CronogramandoApp());
}

String resolveApiBaseUrl() {
  if (defaultApiBaseUrl != 'http://localhost:8000') return defaultApiBaseUrl;
  final base = Uri.base;
  if ((base.scheme == 'http' || base.scheme == 'https') &&
      base.host.isNotEmpty) {
    return '${base.scheme}://${base.host}:8000';
  }
  return defaultApiBaseUrl;
}

class CronogramandoApp extends StatefulWidget {
  const CronogramandoApp({super.key});

  @override
  State<CronogramandoApp> createState() => _CronogramandoAppState();
}

class _Section {
  const _Section({
    required this.title,
    required this.subtitle,
    required this.icon,
    required this.builder,
  });

  final String title;
  final String subtitle;
  final IconData icon;
  final Widget Function(ApiClient api) builder;
}

class _CronogramandoAppState extends State<CronogramandoApp> {
  final ApiClient _api = ApiClient(baseUrl: resolveApiBaseUrl());
  int _index = 0;

  late final List<_Section> _sections = [
    _Section(
      title: 'Auth',
      subtitle: 'register · login · refresh · me',
      icon: Icons.lock_outline,
      builder: (api) => AuthScreen(api: api),
    ),
    _Section(
      title: 'Metas & Módulos',
      subtitle: 'metas CRUD · módulos',
      icon: Icons.flag_outlined,
      builder: (api) => GoalsScreen(api: api),
    ),
    _Section(
      title: 'Plan',
      subtitle: 'plan · steps · activities',
      icon: Icons.list_alt,
      builder: (api) => PlanScreen(api: api),
    ),
    _Section(
      title: 'Schedule',
      subtitle: 'list · move slot',
      icon: Icons.calendar_month_outlined,
      builder: (api) => ScheduleScreen(api: api),
    ),
    _Section(
      title: 'Reconciliation',
      subtitle: 'concluir módulo · escolha de prazo',
      icon: Icons.rule,
      builder: (api) => ReconciliationScreen(api: api),
    ),
    _Section(
      title: 'PDF Content',
      subtitle: 'content · translation',
      icon: Icons.picture_as_pdf_outlined,
      builder: (api) => ContentScreen(api: api),
    ),
    _Section(
      title: 'Admin',
      subtitle: 'users list · patch',
      icon: Icons.admin_panel_settings_outlined,
      builder: (api) => AdminScreen(api: api),
    ),
    _Section(
      title: 'Raw Console',
      subtitle: 'any method · any path',
      icon: Icons.terminal,
      builder: (api) => ConsoleScreen(api: api),
    ),
    _Section(
      title: '8a Prototype',
      subtitle: 'onboarding · today · calendar · roadmap · activity',
      icon: Icons.design_services_outlined,
      builder: (api) => const PrototypeShell(),
    ),
  ];

  @override
  void dispose() {
    _api.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Cronogramando',
      debugShowCheckedModeBanner: false,
      locale: const Locale('pt', 'BR'),
      supportedLocales: const [Locale('pt', 'BR'), Locale('en')],
      localizationsDelegates: const [
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      theme: ThemeData(
        colorSchemeSeed: Colors.indigo,
        useMaterial3: true,
      ),
      home: AnimatedBuilder(
        animation: _api,
        builder: (context, _) {
          final section = _sections[_index];
          return Scaffold(
            appBar: AppBar(
              title: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(section.title),
                  Text(
                    _api.baseUrl,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
              actions: [
                Padding(
                  padding: const EdgeInsets.only(right: 12),
                  child: Chip(
                    avatar: Icon(
                      _api.hasSession ? Icons.lock_open : Icons.lock,
                      size: 16,
                    ),
                    label: Text(_api.hasSession ? 'token' : 'no token'),
                  ),
                ),
              ],
            ),
            drawer: Drawer(
              child: SafeArea(
                child: ListView(
                  children: [
                    const DrawerHeader(
                      child: Align(
                        alignment: Alignment.bottomLeft,
                        child: Text(
                          'Phase 08b validation client',
                          style: TextStyle(fontSize: 18),
                        ),
                      ),
                    ),
                    for (var i = 0; i < _sections.length; i++)
                      ListTile(
                        selected: i == _index,
                        leading: Icon(_sections[i].icon),
                        title: Text(_sections[i].title),
                        subtitle: Text(_sections[i].subtitle),
                        onTap: () {
                          Navigator.pop(context);
                          setState(() => _index = i);
                        },
                      ),
                  ],
                ),
              ),
            ),
            body: SafeArea(
              child: section.builder(_api),
            ),
          );
        },
      ),
    );
  }
}
