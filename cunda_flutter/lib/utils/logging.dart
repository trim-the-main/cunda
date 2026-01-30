import 'package:logging/logging.dart';

void initializeLogging({Level level = Level.INFO}) {
  Logger.root.level = level; // defaults to Level.INFO
  Logger.root.onRecord.listen((record) {
    // Matching the rust default
    // ignore: avoid_print
    print(
      '[${record.time.toUtc().toIso8601String()} ${record.level.name.toUpperCase()}  flutter::${record.loggerName}] ${record.message}',
    );
  });
}
