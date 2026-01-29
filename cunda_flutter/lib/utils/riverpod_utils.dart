import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart';
import 'package:logging/logging.dart';

final _log = Logger('RiverpodUtils');

class NoNeedToRetry implements Exception {
  final String message;
  NoNeedToRetry(this.message);
  @override
  String toString() => 'NoNeedToRetry: $message';
}

/// Riverpod custom retry handler.
Duration? noRetry(int retryCount, Object error) {
  return null;
}

Duration? retryOnce(int retryCount, Object error) {
  if (retryCount >= 1) return null;

  if (error is ProviderException) return null;
  if (error is NoNeedToRetry) return null;

  return Duration(); // immediate retry
}

// A basic logger, which logs any state changes.
final class DebugRiverpod extends ProviderObserver {
  @override
  void didUpdateProvider(
    ProviderObserverContext context,
    Object? previousValue,
    Object? newValue,
  ) {
    _log.fine('''
{
  "provider": "${context.provider}",
  "newValue": "$newValue",
  "mutation": "${context.mutation}"
}''');
  }
}
