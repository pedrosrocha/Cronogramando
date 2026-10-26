import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart' show rootBundle;

/// Mock domain model for the Phase 08a design prototype.
///
/// The shape intentionally mirrors the backend entities (goal -> books -> steps
/// -> activities -> schedule slots) so Phase 09c can swap this repository for the
/// API-backed one without redesigning the screens.
enum MockActivityKind { video, pdf, quiz, task }

class MockActivity {
  const MockActivity({
    required this.id,
    required this.title,
    required this.kind,
    required this.durationMins,
    this.description,
    this.resourceUrl,
    this.pdfText,
    this.quizQuestion,
    this.quizOptions = const [],
    this.quizAnswerIndex,
    this.taskChecklist = const [],
  });

  final String id;
  final String title;
  final MockActivityKind kind;
  final int durationMins;
  final String? description;
  final String? resourceUrl;
  final String? pdfText;
  final String? quizQuestion;
  final List<String> quizOptions;
  final int? quizAnswerIndex;
  final List<String> taskChecklist;

  factory MockActivity.fromJson(Map<String, dynamic> json) {
    return MockActivity(
      id: json['id'] as String,
      title: json['title'] as String,
      kind: MockActivityKind.values.byName(json['kind'] as String),
      durationMins: (json['duration_mins'] as num).toInt(),
      description: json['description'] as String?,
      resourceUrl: json['resource_url'] as String?,
      pdfText: json['pdf_text'] as String?,
      quizQuestion: json['quiz_question'] as String?,
      quizOptions:
          (json['quiz_options'] as List<dynamic>? ?? const []).cast<String>(),
      quizAnswerIndex: (json['quiz_answer_index'] as num?)?.toInt(),
      taskChecklist:
          (json['task_checklist'] as List<dynamic>? ?? const []).cast<String>(),
    );
  }
}

class MockStep {
  const MockStep({
    required this.id,
    required this.title,
    required this.outcome,
    required this.durationMins,
    required this.activities,
  });

  final String id;
  final String title;
  final String outcome;
  final int durationMins;
  final List<MockActivity> activities;

  factory MockStep.fromJson(Map<String, dynamic> json) {
    return MockStep(
      id: json['id'] as String,
      title: json['title'] as String,
      outcome: json['outcome'] as String,
      durationMins: (json['duration_mins'] as num).toInt(),
      activities: (json['activities'] as List<dynamic>)
          .map((item) => MockActivity.fromJson(item as Map<String, dynamic>))
          .toList(),
    );
  }
}

class MockBook {
  const MockBook({
    required this.id,
    required this.title,
    required this.objective,
    required this.status,
    required this.steps,
  });

  final String id;
  final String title;
  final String objective;
  final String status;
  final List<MockStep> steps;

  factory MockBook.fromJson(Map<String, dynamic> json) {
    return MockBook(
      id: json['id'] as String,
      title: json['title'] as String,
      objective: json['objective'] as String,
      status: json['status'] as String,
      steps: (json['steps'] as List<dynamic>)
          .map((item) => MockStep.fromJson(item as Map<String, dynamic>))
          .toList(),
    );
  }
}

class MockSlot {
  const MockSlot({
    required this.date,
    required this.stepId,
    required this.startMinute,
    required this.endMinute,
  });

  final DateTime date;
  final String stepId;
  final int startMinute;
  final int endMinute;

  String get timeRangeLabel {
    String format(int minutes) {
      final hour = minutes ~/ 60;
      final minute = minutes % 60;
      return '${hour.toString().padLeft(2, '0')}:'
          '${minute.toString().padLeft(2, '0')}';
    }

    return '${format(startMinute)}–${format(endMinute)}';
  }
}

class MockGoal {
  const MockGoal({
    required this.title,
    required this.why,
    required this.deadline,
    required this.availabilityDays,
    required this.minutesPerDay,
    required this.streakDays,
    required this.books,
    required this.slots,
  });

  final String title;
  final String why;
  final DateTime deadline;
  final List<int> availabilityDays;
  final int minutesPerDay;
  final int streakDays;
  final List<MockBook> books;
  final List<MockSlot> slots;

  MockGoal copyWith({
    String? title,
    String? why,
    DateTime? deadline,
    List<int>? availabilityDays,
    int? minutesPerDay,
  }) {
    return MockGoal(
      title: title ?? this.title,
      why: why ?? this.why,
      deadline: deadline ?? this.deadline,
      availabilityDays: availabilityDays ?? this.availabilityDays,
      minutesPerDay: minutesPerDay ?? this.minutesPerDay,
      streakDays: streakDays,
      books: books,
      slots: slots,
    );
  }

  MockBook? bookForStep(String stepId) {
    for (final book in books) {
      if (book.steps.any((step) => step.id == stepId)) return book;
    }
    return null;
  }

  MockBook? bookById(String id) {
    for (final book in books) {
      if (book.id == id) return book;
    }
    return null;
  }

  MockStep? stepById(String id) {
    for (final book in books) {
      for (final step in book.steps) {
        if (step.id == id) return step;
      }
    }
    return null;
  }

  MockActivity? activityById(String id) {
    for (final book in books) {
      for (final step in book.steps) {
        for (final activity in step.activities) {
          if (activity.id == id) return activity;
        }
      }
    }
    return null;
  }

  List<MockSlot> slotsFor(DateTime date) {
    final target = DateUtils.dateOnly(date);
    return slots
        .where((slot) => DateUtils.isSameDay(slot.date, target))
        .toList()
      ..sort((a, b) => a.startMinute.compareTo(b.startMinute));
  }

  int get totalActivities => books
      .expand((book) => book.steps)
      .expand((step) => step.activities)
      .length;
}

class PrototypeRepository {
  const PrototypeRepository();

  Future<MockGoal> load() async {
    final source = await rootBundle.loadString('assets/mock/prototype.json');
    final json = jsonDecode(source) as Map<String, dynamic>;
    final goalJson = json['goal'] as Map<String, dynamic>;
    final availability = goalJson['availability'] as Map<String, dynamic>;
    final today = DateUtils.dateOnly(DateTime.now());
    final availableDays = (availability['days'] as List<dynamic>)
        .cast<String>()
        .map(_weekdayFromCode)
        .whereType<int>()
        .toList();

    final books = (json['books'] as List<dynamic>)
        .map((item) => MockBook.fromJson(item as Map<String, dynamic>))
        .toList();

    final slots = (json['slots'] as List<dynamic>).map((item) {
      final slot = item as Map<String, dynamic>;
      return MockSlot(
        date: today.add(Duration(days: (slot['day_offset'] as num).toInt())),
        stepId: slot['step_id'] as String,
        startMinute: (slot['start_minute'] as num).toInt(),
        endMinute: (slot['end_minute'] as num).toInt(),
      );
    }).toList();

    return MockGoal(
      title: goalJson['title'] as String,
      why: goalJson['why'] as String,
      deadline:
          today.add(Duration(days: (goalJson['deadline_days'] as num).toInt())),
      availabilityDays: availableDays,
      minutesPerDay: (availability['minutes_per_day'] as num).toInt(),
      streakDays: (goalJson['streak_days'] as num).toInt(),
      books: books,
      slots: slots,
    );
  }

  int? _weekdayFromCode(String code) {
    const map = {
      'mon': DateTime.monday,
      'tue': DateTime.tuesday,
      'wed': DateTime.wednesday,
      'thu': DateTime.thursday,
      'fri': DateTime.friday,
      'sat': DateTime.saturday,
      'sun': DateTime.sunday,
    };
    return map[code];
  }
}
