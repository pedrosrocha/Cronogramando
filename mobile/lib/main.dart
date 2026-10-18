import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

/// Phase 08a design prototype entry — mocked "today" view, no backend.
/// pt-BR default, en fallback.
void main() => runApp(const CronogramandoApp());

final _mockToday = [
  ("Revisao rapida: derivadas", "5 min", "quick-win"),
  ("Video: integrais definidas", "30 min", "video"),
  ("Quiz: exercicios 1-5", "15 min", "quiz"),
];

class CronogramandoApp extends StatelessWidget {
  const CronogramandoApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Cronogramando',
      locale: const Locale('pt', 'BR'),
      supportedLocales: const [Locale('pt', 'BR'), Locale('en')],
      localizationsDelegates: const [
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      home: const TodayPage(),
    );
  }
}

class TodayPage extends StatelessWidget {
  const TodayPage({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Hoje')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          const Card(
            child: ListTile(
              title: Text('Por que: passar em Calculo II'),
              subtitle: Text('Meta: Calculo II em 3 meses'),
            ),
          ),
          for (final (title, duration, kind) in _mockToday)
            Card(
              child: ListTile(
                leading: Icon(
                  kind == 'quick-win'
                      ? Icons.bolt
                      : kind == 'video'
                          ? Icons.play_circle
                          : Icons.quiz,
                ),
                title: Text(title),
                subtitle: Text('$duration • $kind'),
                trailing: Checkbox(value: false, onChanged: (_) {}),
              ),
            ),
        ],
      ),
    );
  }
}
