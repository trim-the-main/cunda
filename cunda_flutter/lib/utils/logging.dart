import 'package:logging/logging.dart';

void initializeLogging({Level level = Level.INFO}) {
  const envLevel = String.fromEnvironment("LOG_LEVEL");
  if (envLevel.isNotEmpty) {
    final l = Level.LEVELS.firstWhere(
      (element) => element.name == envLevel.toUpperCase(),
      orElse: () => Level.INFO,
    );
    level = l;
  }
  Logger.root.level = level;
  Logger.root.onRecord.listen((record) {
    // Matching the rust default
    // ignore: avoid_print
    print(
      '[${record.time.toUtc().toIso8601String()} ${record.level.name.toUpperCase()}  flutter::${record.loggerName}] ${record.message}',
    );
  });
}
