import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';

final log = Logger('RiverpodUtils');

/// Riverpod custom retry handler.
Duration? noRetry(int retryCount, Object error) {
  return null;
}

// A basic logger, which logs any state changes.
final class DebugRiverpod extends ProviderObserver {
  @override
  void didUpdateProvider(
    ProviderObserverContext context,
    Object? previousValue,
    Object? newValue,
  ) {
    log.fine('''
{
  "provider": "${context.provider}",
  "newValue": "$newValue",
  "mutation": "${context.mutation}"
}''');
  }
}
