import 'dart:async';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/cunda_device_base.dart';
import 'package:cunda_flutter/providers/rpc/ble_wiring.dart';
import 'package:cunda_flutter/services/ble/ble.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';

final _log = Logger('BaseRpcProvider');

// Create a simple Cunda base client. We do a device_id query and move
// this base class to the real protocol client. This move is a rust move,
// the base is inaccessible so we cannot call base.rx_callback anymore
// This is why the ble characteristic stream subscription is temporary
// The protocol client will cancel this subscription and re-listen to the
// same stream, calling `protocolClient.rx_callback`.
FutureOr<(CundaDeviceBase, StreamSubscription)> temporaryBaseClient(
  BluetoothDevice device,
) async {
  _log.fine("Creating rpc client");

  if (device.isDisconnected) {
    throw ConnectionLost();
  }

  final timer = Stopwatch()..start();
  final base = CundaDeviceBase();
  await bleWireTx(device, base);
  final rxSubs = await bleWireRx(device, base);
  _log.fine("Client creation took ${timer.elapsedMilliseconds} milliseconds");
  timer.reset();

  return (base, rxSubs);
}
