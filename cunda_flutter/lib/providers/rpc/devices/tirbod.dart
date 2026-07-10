import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/tirbod.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/devices/tirbod/v1/endpoints.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'tirbod.g.dart';

// ignore: unused_element
final _log = Logger('TirbodProvider');

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<TirbodV1Client> tirbodClient(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(protocolClientProvider(device).future);
  return client as TirbodV1Client;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<TirbodEndpoints> appEndpoints(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(tirbodClientProvider(device).future);
  return client as TirbodEndpoints;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<TirbodTopics> appTopics(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(tirbodClientProvider(device).future);
  return client as TirbodTopics;
}
