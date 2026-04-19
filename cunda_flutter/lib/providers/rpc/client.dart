import 'dart:async';
import 'dart:io';
import 'dart:math';

import 'package:async/async.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc/client.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/v1/endpoints.dart';
import 'package:cunda_flutter/providers/ble/ble_providers.dart';
import 'package:cunda_flutter/services/ble/ble.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'client.g.dart';

final log = Logger('RpcClientProvider');
const gattOverhead = 7;

enum RpcUuid {
  service(uuidStr: '408813DF-5DD4-1F87-EC11-CDB001100000'),
  toServerNotAcked(
    uuidStr: '408813df-5dd4-1f87-ec11-cdb001100001',
  ), // to microcontroller
  toServerAcked(
    uuidStr: '408813df-5dd4-1f87-ec11-cdb001100002',
  ), // to microcontroller
  toClientNotAcked(uuidStr: '408813df-5dd4-1f87-ec11-cdb001100003'), // to us
  toClientAcked(uuidStr: '408813df-5dd4-1f87-ec11-cdb001100004'); // to us

  final String uuidStr;

  const RpcUuid({required this.uuidStr});

  Guid get guidValue => Guid(uuidStr);
}

class NotRpcDeviceException implements Exception {
  final RpcUuid notFound;

  NotRpcDeviceException({required this.notFound});
}

Future<FlutterClient> _createRpcClientFor(BluetoothDevice device) async {
  log.fine("Setting up the Rpc client");
  final services = await device.discoverServices();
  log.fine("discovered services");
  final List<BluetoothCharacteristic> rpcServiceCharacteristics;
  try {
    rpcServiceCharacteristics = services
        .where((s) => s.serviceUuid == RpcUuid.service.guidValue)
        .single
        .characteristics;
  } on StateError {
    log.warning("No Rpc service: {}", services);
    throw NotRpcDeviceException(notFound: RpcUuid.service);
  }

  log.fine("List of characteristics: $rpcServiceCharacteristics");
  BluetoothCharacteristic toServerNotAcked;
  try {
    toServerNotAcked = rpcServiceCharacteristics
        .where(
          (c) => c.characteristicUuid == RpcUuid.toServerNotAcked.guidValue,
        )
        .single;
  } on StateError {
    log.warning("No characteristic with UUID rpcToServerNotAckedCharUuid");
    throw NotRpcDeviceException(notFound: RpcUuid.toServerNotAcked);
  }
  BluetoothCharacteristic toServerAcked;
  try {
    toServerAcked = rpcServiceCharacteristics
        .where((c) => c.characteristicUuid == RpcUuid.toServerAcked.guidValue)
        .single;
  } on StateError {
    log.warning("No characteristic with UUID rpcToServerAckedCharUuid");
    throw NotRpcDeviceException(notFound: RpcUuid.toServerAcked);
  }
  BluetoothCharacteristic toClientNotAcked;
  try {
    toClientNotAcked = rpcServiceCharacteristics
        .where(
          (c) => c.characteristicUuid == RpcUuid.toClientNotAcked.guidValue,
        )
        .single;
  } on StateError {
    log.warning("No characteristic with UUID rpcToClientNotAckedCharUuid");
    throw NotRpcDeviceException(notFound: RpcUuid.toClientNotAcked);
  }

  BluetoothCharacteristic toClientAcked;
  try {
    toClientAcked = rpcServiceCharacteristics
        .where((c) => c.characteristicUuid == RpcUuid.toClientAcked.guidValue)
        .single;
  } on StateError {
    log.warning("No characteristic with UUID rpcToClientAckedCharUuid");
    throw NotRpcDeviceException(notFound: RpcUuid.toClientAcked);
  }
  log.fine("Creating Rpc client");
  final client = FlutterClient();

  // Send data to rust ffi using the callback
  // It is important we wait for completion of the rxCallback,
  // otherwise the data may reach out of order. There shouldn't be
  // a second Future in flight before the first one completes.
  final rxStreamSub =
      StreamGroup.merge([
        toClientNotAcked.onValueReceived,
        toClientAcked.onValueReceived,
      ]).listen((data) async {
        log.fine("rx from device: ${data.length} bytes");
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
    log.finest("tx to device: ${data.length} bytes");
    log.finest("$data");
    final chunkSize = Platform.isLinux
        ? 251 - gattOverhead
        : device.mtuNow -
              gattOverhead; // 3 bytes are used for ATT protocol overhead
    if (data.length <= chunkSize) {
      log.finest("data is smaller than mtu, sending in one go");
      final writeTimer = Stopwatch()..start();
      await toServerAcked.write(data);
      log.fine("Write took ${writeTimer.elapsedMicroseconds} microseconds");
    } else {
      log.finest(
        "data is larger than mtu, splitting into chunks of size $chunkSize",
      );
      for (var i = 0; i < data.length; i += chunkSize) {
        final chunk = data.sublist(i, min(data.length, i + chunkSize));
        log.finest(
          "sending chunk of size ${chunk.length}/${data.length} from offset $i",
        );
        log.finest("$chunk");
        if (i >= data.length - chunkSize) {
          final last = chunk.length < 2 ? null : chunk[chunk.length - 2];
          log.info(
            "        --->> sending last chunk, last byte before terminating zero: $last",
          );
          await toServerAcked.write(chunk);
          log.info(" <<---        successfully sent the last chunk");
        } else {
          log.info(
            "--->> sending unacked, last byte before terminating zero ${chunk.length}",
          );
          await toServerNotAcked.write(chunk, withoutResponse: true);
          log.info("<<--- unacked sent ");
        }
      }
    }
  });
  device.cancelWhenDisconnected(txStreamSub);

  if (!toClientAcked.isNotifying) {
    log.fine("notifications are not on so we are turning them on");
    await toClientAcked.setNotifyValue(true);
  }
  if (!toClientNotAcked.isNotifying) {
    log.fine("notifications are not on so we are turning them on");
    await toClientNotAcked.setNotifyValue(true);
  }
  return client;
}

// Rpc client provider
// We create rpc client object with keepAlive:true so that even if the
// widgets don't need the client anymore we keep it alive. This doesn't
// mean we keep it alive forever, we dispose it when the device disconnects.
@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<FlutterClient> rpcClient(Ref ref, BluetoothDevice device) async {
  log.fine("Creating rpc client");

  // We will be notified when connectionManager state changes. If disconnected we
  // should not continue.
  if (ref.watch(connectionManagerProvider(device)) ==
      ConnectionTransitionState.disconnected) {
    throw ConnectionLost();
  }

  final client = await _createRpcClientFor(device);
  await client.approveFirmware(req: NoArg());
  ref.onDispose(() {
    log.fine("Disposing rpc client");
    client.dispose(); // calls drop on the rust side
  });
  ref.onCancel(() {
    log.fine("Canceling rpcClient");
  });

  return client;
}
