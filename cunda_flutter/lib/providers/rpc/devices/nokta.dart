import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/nokta.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/devices/nokta/v1/endpoints.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'nokta.g.dart';

// ignore: unused_element
final _log = Logger('noktaProvider');

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<NoktaV1Client> noktaClient(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(protocolClientProvider(device).future);
  return client as NoktaV1Client;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<NoktaEndpoints> appEndpoints(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(noktaClientProvider(device).future);
  return client as NoktaEndpoints;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<NoktaTopics> appTopics(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(noktaClientProvider(device).future);
  return client as NoktaTopics;
}
