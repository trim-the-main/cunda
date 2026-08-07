import 'dart:async';
import 'dart:io';
import 'dart:math';

import 'package:async/async.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';

final _log = Logger('BleWiring');
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

FutureOr<List<BluetoothCharacteristic>> rpcCharacteristics(
  BluetoothDevice device,
) async {
  final timer = Stopwatch()..start();
  final services = await device.discoverServices();
  _log.fine(
    "Discovering services took ${timer.elapsedMilliseconds} milliseconds",
  );
  timer.reset();
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

  _log.fine("List of characteristics: $rpcServiceCharacteristics");
  return rpcServiceCharacteristics;
}

Future<Stream<List<int>>> rxDataStream(BluetoothDevice device) async {
  final rpcServiceCharacteristics = await rpcCharacteristics(device);
  BluetoothCharacteristic toClientNotAcked;
  try {
    toClientNotAcked = rpcServiceCharacteristics
        .where(
          (c) => c.characteristicUuid == RpcUuid.toClientNotAcked.guidValue,
        )
        .single;
  } on StateError {
    _log.warning("No characteristic with UUID rpcToServerNotAckedCharUuid");
    throw NotRpcDeviceException(notFound: RpcUuid.toServerNotAcked);
  }
  BluetoothCharacteristic toClientAcked;
  try {
    toClientAcked = rpcServiceCharacteristics
        .where((c) => c.characteristicUuid == RpcUuid.toClientAcked.guidValue)
        .single;
  } on StateError {
    _log.warning("No characteristic with UUID rpcToServerAckedCharUuid");
    throw NotRpcDeviceException(notFound: RpcUuid.toServerAcked);
  }
  final mergedDataRx = StreamGroup.mergeBroadcast([
    toClientNotAcked.onValueReceived,
    toClientAcked.onValueReceived,
  ]);

  final logSub = mergedDataRx.listen((data) async {
    _log.info("function rx from device: ${data.length} bytes");
  });
  device.cancelWhenDisconnected(logSub);
  return mergedDataRx;
}

Future<StreamSubscription<List<int>>> bleWire(
  BluetoothDevice device,
  FlutterWire wire,
) async {
  final List<BluetoothCharacteristic> rpcServiceCharacteristics =
      await rpcCharacteristics(device);
  final timer = Stopwatch()..start();
  BluetoothCharacteristic toServerNotAcked;
  try {
    toServerNotAcked = rpcServiceCharacteristics
        .where(
          (c) => c.characteristicUuid == RpcUuid.toServerNotAcked.guidValue,
        )
        .single;
  } on StateError {
    _log.warning("No characteristic with UUID rpcToServerNotAckedCharUuid");
    throw NotRpcDeviceException(notFound: RpcUuid.toServerNotAcked);
  }
  BluetoothCharacteristic toServerAcked;
  try {
    toServerAcked = rpcServiceCharacteristics
        .where((c) => c.characteristicUuid == RpcUuid.toServerAcked.guidValue)
        .single;
  } on StateError {
    _log.warning("No characteristic with UUID rpcToServerAckedCharUuid");
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
    _log.warning("No characteristic with UUID rpcToClientNotAckedCharUuid");
    throw NotRpcDeviceException(notFound: RpcUuid.toClientNotAcked);
  }

  BluetoothCharacteristic toClientAcked;
  try {
    toClientAcked = rpcServiceCharacteristics
        .where((c) => c.characteristicUuid == RpcUuid.toClientAcked.guidValue)
        .single;
  } on StateError {
    _log.warning("No characteristic with UUID rpcToClientAckedCharUuid");
    throw NotRpcDeviceException(notFound: RpcUuid.toClientAcked);
  }
  _log.fine("Creating Rpc client");

  // Send data to rust ffi using the callback
  // It is important we wait for completion of the rxCallback,
  // otherwise the data may reach out of order. There shouldn't be
  // a second Future in flight before the first one completes.
  final rxStreamSub = (await rxDataStream(device)).listen((data) async {
    _log.fine("rx from device: ${data.length} bytes");
    await wire.rxCallback(data: data);
  });
  device.cancelWhenDisconnected(rxStreamSub);
  _log.fine("Subscribed to call rxCallback on wire");
  final logSub = (await rxDataStream(device)).listen((data) async {
    _log.info("rx from device: ${data.length} bytes");
  });
  device.cancelWhenDisconnected(logSub);
  _log.fine("Subscribed to log");

  // Data coming from the rust ffi uses the stream api
  // In flutter rust bridge, the generated code for function/method
  // calls usually have the same signature except when we're dealing
  // with streams. Here client.init() on the flutter side returns
  // a Stream, however on the rust end the function signature is
  // different and `init` method takes in a StreamSink argument. So
  // long story short the stream is not generated in the rust code that
  // we write but rather in the flutter_rust_bridge generated code.
  final txStreamSub = wire.init().listen((data) async {
    _log.fine("tx to device: ${data.length} bytes");
    _log.fine("$data");
    final chunkSize = Platform.isLinux
        ? 251 - gattOverhead
        : device.mtuNow -
              gattOverhead; // 3 bytes are used for ATT protocol overhead
    if (data.length <= chunkSize) {
      _log.finest("data is smaller than mtu, sending reliably in one go");
      final writeTimer = Stopwatch()..start();
      await toServerAcked.write(data);
      _log.finest("Write took ${writeTimer.elapsedMicroseconds} microseconds");
    } else {
      _log.finest(
        "data is larger than mtu, splitting into chunks of size $chunkSize",
      );
      for (var i = 0; i < data.length; i += chunkSize) {
        final chunk = data.sublist(i, min(data.length, i + chunkSize));
        _log.finest(
          "sending chunk of size ${chunk.length}/${data.length} from offset $i",
        );
        _log.finest("$chunk");
        if (i >= data.length - chunkSize) {
          final last = chunk.length < 2 ? null : chunk[chunk.length - 2];
          _log.finest(
            "        --->> sending last chunk, last byte before terminating zero: $last",
          );
          await toServerAcked.write(chunk);
          _log.finest(" <<---        successfully sent the last chunk");
        } else {
          _log.finest(
            "--->> sending unacked, last byte before terminating zero ${chunk.length}",
          );
          await toServerNotAcked.write(chunk, withoutResponse: true);
          _log.finest("<<--- unacked sent ");
        }
      }
    }
  });
  device.cancelWhenDisconnected(txStreamSub);

  timer.reset();
  if (!toClientAcked.isNotifying) {
    await toClientAcked.setNotifyValue(true);
  }
  if (!toClientNotAcked.isNotifying) {
    await toClientNotAcked.setNotifyValue(true);
  }
  _log.fine(
    "setting notifications took ${timer.elapsedMilliseconds} milliseconds",
  );
  return rxStreamSub;
}
