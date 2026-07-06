import 'dart:async';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/cunda_device_base.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/types.dart';
import 'package:cunda_flutter/providers/ble/ble_providers.dart';
import 'package:cunda_flutter/providers/rpc/ble_wiring.dart';
import 'package:cunda_flutter/services/ble/ble.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'base.g.dart';

final _log = Logger('BaseRpcProvider');

// Base rpc client provider
// We create rpc client objects all with keepAlive:true so that even if the
// widgets don't need the client anymore we keep it alive. This doesn't
// mean we keep it alive forever, we dispose it when the device disconnects.
// The base client only implements get_device_id api but more importantly
// handles the BLE wiring.
// Downstream providers that depend on this will call get_device_id and
// construct the correct protocol client to consume.
@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<CundaDeviceBase> cundaDeviceBase(
  Ref ref,
  BluetoothDevice device,
) async {
  _log.fine("Creating rpc client");

  // We will be notified when connectionManager state changes. If disconnected we invalidate
  // the provider (hence the downstream protocol clients go away).
  if (ref.watch(connectionManagerProvider(device)) ==
      ConnectionTransitionState.disconnected) {
    throw ConnectionLost();
  }

  final timer = Stopwatch()..start();
  final base = CundaDeviceBase();
  await bleWire(device, base);
  _log.fine("Client creation took ${timer.elapsedMilliseconds} milliseconds");
  timer.reset();
  ref.onDispose(() {
    _log.fine("Disposing rpc client");
    base.dispose(); // calls drop on the rust side
  });

  return base;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<CundaDevice> cundaDeviceClient(Ref ref, BluetoothDevice device) async {
  final base = await ref.watch(cundaDeviceBaseProvider(device).future);
  return base as CundaDevice;
}

@Riverpod(keepAlive: true, retry: noRetry)
Future<DeviceId> deviceId(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(cundaDeviceClientProvider(device).future);
  _log.fine("Calling getDeviceId RPC endpoint");
  return client.getDeviceId(req: NoArg());
}
