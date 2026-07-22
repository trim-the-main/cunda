import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/demo_esp32.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/devices/demo_esp32/v1/endpoints.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'demo_esp32.g.dart';

// ignore: unused_element
final _log = Logger('DemoEsp32Provider');

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<DemoV1Client> demoEsp32Client(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(protocolClientProvider(device).future);
  return client as DemoV1Client;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<DemoAppEndpoints> appEndpoints(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(demoEsp32ClientProvider(device).future);
  return client as DemoAppEndpoints;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<DemoAppTopics> appTopics(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(demoEsp32ClientProvider(device).future);
  return client as DemoAppTopics;
}
