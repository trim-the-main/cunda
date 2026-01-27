import 'package:cunda_flutter/pages/scanner/scanner.dart';
import 'package:flutter/material.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/frb_generated.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:logging/logging.dart';

final log = Logger('CundaMain');

Future<void> main() async {
  Logger.root.level = Level.FINE; // defaults to Level.INFO
  Logger.root.onRecord.listen((record) {
    // Matching the rust default
    // ignore: avoid_print
    print(
      '[${record.time.toUtc().toIso8601String()} ${record.level.name.toUpperCase()}  flutter::${record.loggerName}] ${record.message}',
    );
  });
  await RustLib.init();
  WidgetsFlutterBinding.ensureInitialized();
  runApp(ProviderScope(child: const BlePostcardRpcApp()));
}

class BlePostcardRpcApp extends StatelessWidget {
  const BlePostcardRpcApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(title: 'Postcard Rpc BLE App', home: ScannerPage());
  }
}
