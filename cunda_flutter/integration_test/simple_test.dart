import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/frb_generated.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:cunda_flutter/main.dart';
import 'package:integration_test/integration_test.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());
  testWidgets('Can call rust function', (WidgetTester tester) async {});
}
