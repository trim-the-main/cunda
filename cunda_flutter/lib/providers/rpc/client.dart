import 'dart:async';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc/client.dart';
import 'package:cunda_flutter/providers/ble/ble_providers.dart';
import 'package:cunda_flutter/services/ble/ble.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'client.g.dart';

final _log = Logger('RpcClientProvider');

enum RpcUuid {
  service(uuidStr: '408813DF-5DD4-1F87-EC11-CDB001100000'),
  toServer(
    uuidStr: '408813df-5dd4-1f87-ec11-cdb001100001',
  ), // to microcontroller
  toClient(uuidStr: '408813df-5dd4-1f87-ec11-cdb001100002'); // to us

  final String uuidStr;

  const RpcUuid({required this.uuidStr});

  Guid get guidValue => Guid(uuidStr);
}

class NotRpcDeviceException implements Exception {
  final RpcUuid notFound;

  NotRpcDeviceException({required this.notFound});
}

Future<FlutterClient> _createRpcClientFor(BluetoothDevice device) async {
  _log.fine("Setting up the Rpc client");
  final services = await device.discoverServices();
  _log.fine("discovered services");
  final List<BluetoothCharacteristic> rpcServiceCharacteristics;
  try {
    rpcServiceCharacteristics = services
        .where((s) => s.serviceUuid == RpcUuid.service.guidValue)
        .single
        .characteristics;
  } on StateError {
    _log.warning("No Rpc service: {}", services);
    throw NotRpcDeviceException(notFound: RpcUuid.service);
  }

  BluetoothCharacteristic toServer;
  _log.fine("List of characteristics: $rpcServiceCharacteristics");
  try {
    toServer = rpcServiceCharacteristics
        .where((c) => c.characteristicUuid == RpcUuid.toServer.guidValue)
        .single;
  } on StateError {
    _log.warning("No characteristic with UUID rpcToServerCharUuid");
    throw NotRpcDeviceException(notFound: RpcUuid.toServer);
  }
  BluetoothCharacteristic toClient;
  try {
    toClient = rpcServiceCharacteristics
        .where((c) => c.characteristicUuid == RpcUuid.toClient.guidValue)
        .single;
  } on StateError {
    _log.warning("No characteristic with UUID rpcToServerCharUuid");
    throw NotRpcDeviceException(notFound: RpcUuid.toClient);
  }

  _log.fine("Creating Rpc client");
  final client = FlutterClient();

  // Send data to rust ffi using the callback
  // It is important we wait for completion of the rxCallback,
  // otherwise the data may reach out of order. There shouldn't be
  // a second Future in flight before the first one completes.
  final rxStreamSub = toClient.onValueReceived.listen((data) async {
    await client.rxCallback(data: data);
  });
  device.cancelWhenDisconnected(rxStreamSub);

  // Data coming from the rust ffi uses the stream api
  // In flutter rust bridge, the generated code for function/method
  // calls usually have the same signature except when we're dealing
  // with streams. Here client.init() on the flutter side returns
  // a Stream, however on the rust end the function signature is
  // different and `init` method takes in a StreamSink argument. So
  // long story short the stream is not generated in the rust code that
  // we write but rather in the flutter_rust_bridge generated code.
  final txStreamSub = client.init().listen((data) async {
    await toServer.write(data);
  });
  device.cancelWhenDisconnected(txStreamSub);

  if (!toClient.isNotifying) {
    _log.fine("notifications are not on so we are turning them on");
    await toClient.setNotifyValue(true);
  }
  return client;
}

// Rpc client provider
// We create rpc client object with keepAlive:true so that even if the
// widgets don't need the client anymore we keep it alive. This doesn't
// mean we keep it alive forever, we dispose it when the device disconnects.
@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<FlutterClient> rpcClient(Ref ref, BluetoothDevice device) async {
  _log.fine("Creating rpc client");

  // We will be notified when connectionManager state changes. If disconnected we
  // should not continue.
  if (ref.watch(connectionManagerProvider(device)) ==
      ConnectionTransitionState.disconnected) {
    throw ConnectionLost();
  }

  final client = await _createRpcClientFor(device);
  ref.onDispose(() {
    _log.fine("Disposing rpc client");
    client.dispose(); // calls drop on the rust side
  });
  ref.onCancel(() {
    _log.fine("Canceling rpcClient");
  });

  return client;
}
