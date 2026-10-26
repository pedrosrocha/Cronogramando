import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;

const String defaultApiBaseUrl = String.fromEnvironment(
  'API_BASE_URL',
  defaultValue: 'http://localhost:8000',
);

class ApiException implements Exception {
  ApiException(this.statusCode, this.message);

  final int? statusCode;
  final String message;

  @override
  String toString() =>
      statusCode == null ? message : 'HTTP $statusCode — $message';
}

String normalizeBaseUrl(String value) {
  var trimmed = value.trim();
  if (trimmed.isEmpty) return defaultApiBaseUrl;
  if (!trimmed.contains('://')) trimmed = 'http://$trimmed';
  while (trimmed.endsWith('/') && !trimmed.endsWith('://')) {
    trimmed = trimmed.substring(0, trimmed.length - 1);
  }
  return trimmed;
}

class ApiClient extends ChangeNotifier {
  ApiClient({String? baseUrl})
      : _baseUrl = normalizeBaseUrl(baseUrl ?? defaultApiBaseUrl);

  String _baseUrl;
  String? accessToken;
  String? refreshToken;
  Map<String, dynamic>? currentUser;

  String get baseUrl => _baseUrl;

  set baseUrl(String value) {
    final normalized = normalizeBaseUrl(value);
    if (normalized.isEmpty || normalized == _baseUrl) return;
    _baseUrl = normalized;
    notifyListeners();
  }

  bool get hasSession => accessToken != null;

  void clearSession() {
    accessToken = null;
    refreshToken = null;
    currentUser = null;
    notifyListeners();
  }

  Uri _uri(String path, [Map<String, dynamic>? query]) {
    final normalizedBase = _baseUrl.endsWith('/') ? _baseUrl : '$_baseUrl/';
    final normalizedPath = path.startsWith('/') ? path.substring(1) : path;
    final uri = Uri.parse(normalizedBase).resolve(normalizedPath);
    if (query == null || query.isEmpty) return uri;

    final params = <String, String>{};
    query.forEach((key, value) {
      if (value == null) return;
      final text = value.toString().trim();
      if (text.isNotEmpty) params[key] = text;
    });
    if (params.isEmpty) return uri;
    return uri.replace(queryParameters: params);
  }

  Future<dynamic> request(
    String method,
    String path, {
    Map<String, dynamic>? query,
    Object? body,
    bool authenticated = true,
  }) async {
    final uri = _uri(path, query);
    final headers = <String, String>{'Accept': 'application/json'};
    if (body != null) headers['Content-Type'] = 'application/json';
    if (authenticated && accessToken != null) {
      headers['Authorization'] = 'Bearer $accessToken';
    }

    late http.Response response;
    try {
      final request = http.Request(method.toUpperCase(), uri);
      request.headers.addAll(headers);
      if (body != null) request.body = jsonEncode(body);
      final streamed =
          await request.send().timeout(const Duration(seconds: 180));
      response = await http.Response.fromStream(streamed);
    } catch (error) {
      throw ApiException(null, 'network error: $error');
    }

    final text = response.body.trim();
    dynamic payload;
    if (text.isNotEmpty) {
      try {
        payload = jsonDecode(text);
      } on FormatException {
        payload = text;
      }
    }

    if (response.statusCode < 200 || response.statusCode >= 300) {
      var message = response.reasonPhrase ?? 'request failed';
      if (payload is Map && payload['message'] is String) {
        message = payload['message'] as String;
      } else if (payload is Map && payload['error'] is String) {
        message = payload['error'] as String;
      }
      throw ApiException(response.statusCode, message);
    }

    return payload;
  }

  Map<String, dynamic> _asMap(dynamic payload) {
    if (payload is Map<String, dynamic>) return payload;
    if (payload is Map) return payload.cast<String, dynamic>();
    throw ApiException(null, 'expected a JSON object but received $payload');
  }

  List<dynamic> _asList(dynamic payload) {
    if (payload is List) return payload;
    throw ApiException(null, 'expected a JSON array but received $payload');
  }

  Map<String, dynamic> _adoptAuth(dynamic payload) {
    final map = _asMap(payload);
    final access = map['access_token'];
    final refresh = map['refresh_token'];
    final user = map['user'];
    if (access is String) accessToken = access;
    if (refresh is String) refreshToken = refresh;
    if (user is Map) currentUser = user.cast<String, dynamic>();
    notifyListeners();
    return map;
  }

  Future<Map<String, dynamic>> register({
    required String email,
    required String password,
    String? preferredLanguage,
  }) async {
    final payload = await request(
      'POST',
      '/auth/register',
      authenticated: false,
      body: {
        'email': email,
        'password': password,
        if (preferredLanguage != null && preferredLanguage.trim().isNotEmpty)
          'preferred_language': preferredLanguage.trim(),
      },
    );
    return _adoptAuth(payload);
  }

  Future<Map<String, dynamic>> login({
    required String email,
    required String password,
  }) async {
    final payload = await request(
      'POST',
      '/auth/login',
      authenticated: false,
      body: {'email': email, 'password': password},
    );
    return _adoptAuth(payload);
  }

  Future<Map<String, dynamic>> refresh() async {
    final token = refreshToken;
    if (token == null || token.isEmpty) {
      throw ApiException(null, 'no refresh token in this session');
    }
    final payload = await request(
      'POST',
      '/auth/refresh',
      authenticated: false,
      body: {'refresh_token': token},
    );
    return _adoptAuth(payload);
  }

  Future<Map<String, dynamic>> me() async =>
      _asMap(await request('GET', '/auth/me'));

  Future<List<dynamic>> listUsers() async =>
      _asList(await request('GET', '/admin/users'));

  Future<Map<String, dynamic>> updateUser(
    String id, {
    String? role,
    String? preferredLanguage,
  }) async {
    return _asMap(await request(
      'PATCH',
      '/admin/users/$id',
      body: {
        if (role != null && role.isNotEmpty) 'role': role,
        if (preferredLanguage != null && preferredLanguage.trim().isNotEmpty)
          'preferred_language': preferredLanguage.trim(),
      },
    ));
  }

  Future<Map<String, dynamic>> createGoal(Map<String, dynamic> body) async =>
      _asMap(await request('POST', '/goals', body: body));

  Future<List<dynamic>> listGoals() async =>
      _asList(await request('GET', '/goals'));

  Future<Map<String, dynamic>> getGoal(String id) async =>
      _asMap(await request('GET', '/goals/$id'));

  Future<Map<String, dynamic>> updateGoal(
    String id,
    Map<String, dynamic> body,
  ) async =>
      _asMap(await request('PATCH', '/goals/$id', body: body));

  Future<void> deleteGoal(String id) async {
    await request('DELETE', '/goals/$id');
  }

  Future<List<dynamic>> listGoalBooks(String goalId) async =>
      _asList(await request('GET', '/goals/$goalId/books'));

  Future<Map<String, dynamic>> getBook(String id) async =>
      _asMap(await request('GET', '/books/$id'));

  Future<Map<String, dynamic>> completeBook(String id) async =>
      _asMap(await request('POST', '/books/$id/complete'));

  Future<Map<String, dynamic>> reconcileDeadline(
    String id,
    String choice,
  ) async =>
      _asMap(await request(
        'POST',
        '/books/$id/reconcile-deadline',
        body: {'choice': choice},
      ));

  Future<Map<String, dynamic>> getPlan(String bookId) async =>
      _asMap(await request('GET', '/books/$bookId/plan'));

  Future<Map<String, dynamic>> generateSchedule(String bookId) async =>
      _asMap(await request('POST', '/books/$bookId/generate-schedule'));

  Future<Map<String, dynamic>> createStep(
    String bookId,
    Map<String, dynamic> body,
  ) async =>
      _asMap(await request('POST', '/books/$bookId/steps', body: body));

  Future<Map<String, dynamic>> updateStep(
    String stepId,
    Map<String, dynamic> body,
  ) async =>
      _asMap(await request('PATCH', '/steps/$stepId', body: body));

  Future<void> deleteStep(String stepId) async {
    await request('DELETE', '/steps/$stepId');
  }

  Future<Map<String, dynamic>> regenerateResource(String stepId) async =>
      _asMap(await request('POST', '/steps/$stepId/regenerate-resource'));

  Future<Map<String, dynamic>> generateLesson(String stepId) async =>
      _asMap(await request('POST', '/steps/$stepId/generate-lesson'));

  Future<Map<String, dynamic>> getActivity(String activityId) async =>
      _asMap(await request('GET', '/activities/$activityId'));

  Future<Map<String, dynamic>> sendFeedback(
    String activityId, {
    required String rating,
    List<String> reasons = const [],
    String? comment,
  }) async =>
      _asMap(await request(
        'POST',
        '/activities/$activityId/feedback',
        body: {
          'rating': rating,
          'reasons': reasons,
          if (comment != null && comment.trim().isNotEmpty)
            'comment': comment.trim(),
        },
      ));

  Future<Map<String, dynamic>> getTrace(String activityId) async =>
      _asMap(await request('GET', '/activities/$activityId/trace'));

  Future<Map<String, dynamic>> updateBook(
    String id,
    Map<String, dynamic> body,
  ) async =>
      _asMap(await request('PATCH', '/books/$id', body: body));

  Future<Map<String, dynamic>> redecomposeGoal(String id) async =>
      _asMap(await request(
        'POST',
        '/goals/$id/redecompose',
        body: {'confirm': true},
      ));

  Future<Map<String, dynamic>> completeActivity(
    String activityId,
    bool completed,
  ) async =>
      _asMap(await request(
        'PATCH',
        '/activities/$activityId/complete',
        body: {'completed': completed},
      ));

  Future<Map<String, dynamic>> getContent(String activityId) async =>
      _asMap(await request('GET', '/activities/$activityId/content'));

  Future<Map<String, dynamic>> getTranslation(
    String activityId, {
    String? language,
  }) async =>
      _asMap(await request(
        'GET',
        '/activities/$activityId/translation',
        query: {'language': language},
      ));

  Future<List<dynamic>> listSchedule({
    String? date,
    String? goalId,
    String? from,
    String? to,
  }) async =>
      _asList(await request(
        'GET',
        '/schedule',
        query: {'date': date, 'goal_id': goalId, 'from': from, 'to': to},
      ));

  Future<List<dynamic>> moveSlot(String slotId, String date) async =>
      _asList(await request(
        'PATCH',
        '/schedule/$slotId',
        body: {'date': date},
      ));
}
