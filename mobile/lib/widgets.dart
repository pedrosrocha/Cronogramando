import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';

import 'api_client.dart';

String prettyJson(dynamic value) {
  if (value == null) return '';
  try {
    return const JsonEncoder.withIndent('  ').convert(value);
  } catch (_) {
    return value.toString();
  }
}

String? emptyToNull(String value) {
  final trimmed = value.trim();
  return trimmed.isEmpty ? null : trimmed;
}

String formatDate(DateTime date) => DateFormat('yyyy-MM-dd').format(date);

Future<String?> pickDate(BuildContext context, {String? current}) async {
  final today = DateTime.now();
  final parsed = current == null ? null : DateTime.tryParse(current);
  final initial = parsed ?? today;
  final firstDate = DateTime(today.year, today.month, today.day);
  final lastDate = DateTime(today.year + 5, 12, 31);
  final clamped = initial.isBefore(firstDate) ? firstDate : initial;
  final picked = await showDatePicker(
    context: context,
    initialDate: clamped.isAfter(lastDate) ? lastDate : clamped,
    firstDate: firstDate,
    lastDate: lastDate,
  );
  return picked == null ? null : formatDate(picked);
}

Map<String, dynamic> parseJsonObject(String source) {
  final trimmed = source.trim();
  if (trimmed.isEmpty) return <String, dynamic>{};
  final decoded = jsonDecode(trimmed);
  if (decoded is Map<String, dynamic>) return decoded;
  if (decoded is Map) return decoded.cast<String, dynamic>();
  throw const FormatException('body must be a JSON object');
}

mixin ApiRunner<T extends StatefulWidget> on State<T> {
  bool busy = false;
  String? errorMessage;
  dynamic result;

  Future<void> runApi(Future<dynamic> Function() action) async {
    setState(() {
      busy = true;
      errorMessage = null;
    });
    try {
      final value = await action();
      if (!mounted) return;
      setState(() {
        result = value ?? <String, dynamic>{'ok': true};
      });
    } on ApiException catch (error) {
      if (!mounted) return;
      setState(() {
        errorMessage = error.toString();
        result = null;
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        errorMessage = error.toString();
        result = null;
      });
    } finally {
      if (mounted) {
        setState(() {
          busy = false;
        });
      }
    }
  }
}

class ResultPanel extends StatelessWidget {
  const ResultPanel({
    super.key,
    required this.busy,
    this.errorMessage,
    this.result,
  });

  final bool busy;
  final String? errorMessage;
  final dynamic result;

  @override
  Widget build(BuildContext context) {
    if (busy) {
      return const Padding(
        padding: EdgeInsets.symmetric(vertical: 16),
        child: Center(child: CircularProgressIndicator()),
      );
    }
    if (errorMessage != null) {
      return Container(
        width: double.infinity,
        margin: const EdgeInsets.only(top: 12),
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: Theme.of(context).colorScheme.errorContainer,
          borderRadius: BorderRadius.circular(8),
        ),
        child: SelectableText(
          'Error: $errorMessage',
          style: TextStyle(
            color: Theme.of(context).colorScheme.onErrorContainer,
          ),
        ),
      );
    }
    if (result == null) return const SizedBox.shrink();
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(top: 12),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: SelectableText(
        prettyJson(result),
        style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
      ),
    );
  }
}

class SectionCard extends StatelessWidget {
  const SectionCard({
    super.key,
    required this.title,
    required this.children,
    this.subtitle,
  });

  final String title;
  final String? subtitle;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            if (subtitle != null) ...[
              const SizedBox(height: 2),
              Text(
                subtitle!,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
            const SizedBox(height: 10),
            ...children,
          ],
        ),
      ),
    );
  }
}

class AppTextField extends StatelessWidget {
  const AppTextField({
    super.key,
    required this.controller,
    required this.label,
    this.hint,
    this.keyboardType,
    this.maxLines = 1,
    this.onSubmitted,
    this.suffixIcon,
  });

  final TextEditingController controller;
  final String label;
  final String? hint;
  final TextInputType? keyboardType;
  final int maxLines;
  final ValueChanged<String>? onSubmitted;
  final Widget? suffixIcon;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: TextField(
        controller: controller,
        keyboardType: keyboardType,
        maxLines: maxLines,
        onSubmitted: onSubmitted,
        decoration: InputDecoration(
          labelText: label,
          hintText: hint,
          suffixIcon: suffixIcon,
          isDense: true,
          border: const OutlineInputBorder(),
        ),
      ),
    );
  }
}

class WrapActions extends StatelessWidget {
  const WrapActions({super.key, required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: children,
    );
  }
}
