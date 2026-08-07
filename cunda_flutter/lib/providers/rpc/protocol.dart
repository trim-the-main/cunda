import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/demo_esp32.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/fake_dev.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/nokta.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common/v1/endpoints.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/types.dart';
import 'package:cunda_flutter/providers/ble/ble_providers.dart';
import 'package:cunda_flutter/providers/rpc/base.dart';
import 'package:cunda_flutter/providers/rpc/ble_wiring.dart';
import 'package:cunda_flutter/services/ble/ble.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'protocol.g.dart';

// ignore: unused_element
final _log = Logger('ProtocolProvider');

// This is the downstream protocol client that can return any client type that wraps the Base
@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<(DeviceId, Object)> deviceIdProtocolClientPair(
  Ref ref,
  BluetoothDevice device,
) async {
  // We rebuild if the connection state changes
  if (ref.watch(connectionManagerProvider(device)) ==
      ConnectionTransitionState.disconnected) {
    throw ConnectionLost();
  }

  final (base, rxSubs) = await temporaryBaseClient(device);
  _log.fine("Asking for device id");
  final deviceId = await base.getDeviceId(req: NoArg());
  _log.info(
    "Getting protocol client for ${deviceId.deviceType}, ${deviceId.protocolVersion}",
  );

  // we will return this
  final FlutterWire protocolClient;

  // We are moving the base client, we can no longer call base.rx_callback:
  await rxSubs.cancel();

  if ((deviceId.deviceType, deviceId.protocolVersion) ==
      (DemoV1Client.cundaDeviceType(), DemoV1Client.cundaRpcProtocol())) {
    protocolClient = DemoV1Client(base: base);
  } else if ((deviceId.deviceType, deviceId.protocolVersion) ==
      (NoktaV1Client.cundaDeviceType(), NoktaV1Client.cundaRpcProtocol())) {
    protocolClient = NoktaV1Client(base: base);
  } else if ((deviceId.deviceType, deviceId.protocolVersion) ==
      (FakeDevV1Client.cundaDeviceType(), FakeDevV1Client.cundaRpcProtocol())) {
    protocolClient = FakeDevV1Client();
  } else {
    _log.warning("Trying to connect to an unrecognized device");
    throw Exception("Not a recognized device, protocol pair");
  }

  final newStream = await rxDataStream(device);
  final newSub = newStream.listen((data) async {
    _log.fine("rx from nokta device(newstream): ${data.length} bytes");
    await protocolClient.rxCallback(data: data);
  });
  device.cancelWhenDisconnected(newSub);

  _log.fine("Created new sub");

  if (protocolClient is CundaSysE) {
    await (protocolClient as CundaSysE).approveFirmware(req: NoArg());
  }

  return (deviceId, protocolClient);
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<DeviceId> deviceId(Ref ref, BluetoothDevice device) async {
  final (deviceId, _) = await ref.watch(
    deviceIdProtocolClientPairProvider(device).future,
  );
  return deviceId;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<Object> protocolClient(Ref ref, BluetoothDevice device) async {
  final (_, pc) = await ref.watch(
    deviceIdProtocolClientPairProvider(device).future,
  );
  return pc;
}
