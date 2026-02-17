import 'package:cunda_flutter/pages/scanner/scanner.dart';
import 'package:cunda_flutter/utils/logging.dart';
import 'package:flutter/material.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/frb_generated.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:logging/logging.dart';

final _log = Logger('CundaMain');

Future<void> main() async {
  initializeLogging();
  await RustLib.init();
  WidgetsFlutterBinding.ensureInitialized();
  runApp(ProviderScope(child: const CundaApp()));
}

class CundaApp extends StatelessWidget {
  const CundaApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(title: 'Cunda', home: ScannerPage());
  }
}
