import 'dart:convert';
import 'package:oresoftware_next_loggers/oresoftware_next_loggers.dart';

typedef Transport =
    Future<Map<String, Object?>> Function(
      String method,
      Uri url,
      Map<String, String> headers,
      String? body,
    );

final class HappyWakeyClient {
  HappyWakeyClient({
    required String baseUrl,
    required this.transport,
    this.token,
    this.telemetry,
    bool allowInsecureLoopback = false,
  }) : base = Uri.parse(baseUrl) {
    final loopback = const {
      'localhost',
      '127.0.0.1',
      '::1',
    }.contains(base.host);
    if (base.scheme != 'https' && !(allowInsecureLoopback && loopback))
      throw ArgumentError('HTTPS required');
  }
  final Uri base;
  final String? token;
  final Transport transport;
  final Logger? telemetry;
  Future<Object?> health() => _call('health', 'GET', '/healthz', null, false);
  Future<Object?> listAlarms() => _call('list_alarms', 'GET', '/v1/alarms');
  Future<Object?> createAlarm(Object request) =>
      _call('create_alarm', 'POST', '/v1/alarms', request);
  Future<Object?> transitionOccurrence(String id, Object request) => _call(
    'transition_occurrence',
    'POST',
    '/v1/occurrences/${Uri.encodeComponent(id)}/transitions',
    request,
  );
  Future<Object?> pullChanges({String cursor = '0', int limit = 100}) => _call(
    'pull_changes',
    'GET',
    '/v1/sync/pull?cursor=${Uri.encodeQueryComponent(cursor)}&limit=$limit',
  );
  Future<Object?> pushChanges(List<Object?> changes) =>
      _call('push_changes', 'POST', '/v1/sync/push', changes);
  Future<Object?> _call(
    String operation,
    String method,
    String path, [
    Object? body,
    bool auth = true,
  ]) async {
    if (auth && (token == null || token!.isEmpty))
      throw StateError('Shared Auth bearer token required');
    try {
      final response = await transport(method, base.resolve(path), {
        'accept': 'application/json',
        if (auth) 'authorization': 'Bearer $token',
        if (body != null) 'content-type': 'application/json',
      }, body == null ? null : jsonEncode(body));
      await telemetry?.info('happy_wakey.client.request').addFields({
        'operation': operation,
        'status': response['status'],
      }).send();
      return response['body'];
    } catch (error) {
      await telemetry?.error('happy_wakey.client.request').addFields({
        'operation': operation,
      }).send();
      rethrow;
    }
  }
}
