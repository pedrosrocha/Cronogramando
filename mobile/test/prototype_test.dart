import 'package:cronogramando/prototype/mock_data.dart';
import 'package:cronogramando/prototype/prototype_shell.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';

void main() {
  setUpAll(() async {
    await initializeDateFormatting('pt_BR');
  });

  testWidgets('prototype walks onboarding, today, detail and roadmap',
      (tester) async {
    tester.view.physicalSize = const Size(900, 2000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final goal = await tester.runAsync(
      () => const PrototypeRepository().load(),
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: PrototypeShell(loadOverride: Future.value(goal))),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Vamos montar seu cronograma'), findsOneWidget);
    expect(find.text('Passar em Cálculo II'), findsOneWidget);

    await tester.tap(find.text('Criar meu plano'));
    await tester.pumpAndSettle();

    expect(find.text('Hoje'), findsWidgets);
    expect(find.text('Progresso de hoje'), findsOneWidget);
    expect(find.text('Vídeo: limites em 8 minutos'), findsOneWidget);

    await tester.tap(find.text('Vídeo: limites em 8 minutos'));
    await tester.pumpAndSettle();
    expect(find.text('Atividade'), findsOneWidget);
    expect(
      find.text('IFrame do YouTube entra na Phase 09d'),
      findsOneWidget,
    );
    await tester.pageBack();
    await tester.pumpAndSettle();

    await tester.tap(find.text('Calendário').last);
    await tester.pumpAndSettle();
    expect(find.text('Calendário'), findsWidgets);
    expect(find.text('Disponibilidade: seg, qua, sex · 60 min/dia'),
        findsOneWidget);

    await tester.tap(find.text('Roadmap').last);
    await tester.pumpAndSettle();
    expect(find.text('Limites e derivadas'), findsOneWidget);
    expect(find.text('Integrais e aplicações'), findsOneWidget);

    await tester.tap(find.byTooltip('Pré-visualizar celebração').first);
    await tester.pumpAndSettle();
    expect(find.text('Prévia da celebração'), findsOneWidget);
    await tester.tap(find.text('Continuar'));
    await tester.pumpAndSettle();
  });

  testWidgets('PDF activity toggles between reflowed text and original',
      (tester) async {
    tester.view.physicalSize = const Size(900, 2000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final goal = await tester.runAsync(
      () => const PrototypeRepository().load(),
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: PrototypeShell(loadOverride: Future.value(goal))),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Criar meu plano'));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Leitura: limites laterais (2 páginas)'));
    await tester.pumpAndSettle();

    expect(find.text('Texto reflowado'), findsOneWidget);
    expect(find.textContaining('Um limite existe quando'), findsOneWidget);

    await tester.tap(find.text('PDF original'));
    await tester.pumpAndSettle();
    expect(find.text('Visualizador do PDF original'), findsOneWidget);
  });

  testWidgets('quiz gives feedback after checking an answer', (tester) async {
    tester.view.physicalSize = const Size(900, 2000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final goal = await tester.runAsync(
      () => const PrototypeRepository().load(),
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: PrototypeShell(loadOverride: Future.value(goal))),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Criar meu plano'));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Quiz: propriedades de limites'));
    await tester.pumpAndSettle();

    await tester.tap(find.text('5'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Conferir resposta'));
    await tester.pumpAndSettle();
    expect(find.textContaining('Isso! Você aplicou'), findsOneWidget);
  });
}
