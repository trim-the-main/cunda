import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/demo_esp32.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/fake_dev.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/nokta.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/types.dart';
import 'package:cunda_flutter/providers/rpc/base.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'protocol.g.dart';

// ignore: unused_element
final _log = Logger('ProtocolProvider');

// This is the downstream protocol client that can return any client type that wraps the Base
@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<Object> protocolClient(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(cundaDeviceBaseProvider(device).future);
  _log.fine("Asking for device id");
  final deviceId = await ref.watch(deviceIdProvider(device).future);
  _log.info(
    "Getting protocol client for ${deviceId.deviceType}, ${deviceId.protocolVersion}",
  );

  if ((deviceId.deviceType, deviceId.protocolVersion) ==
      (DemoV1Client.cundaDeviceType(), DemoV1Client.cundaRpcProtocol())) {
    final demoClient = DemoV1Client(base: client);
    demoClient.approveFirmware(req: NoArg());
    _log.fine("Returning a demo client");
    return demoClient;
  } else if ((deviceId.deviceType, deviceId.protocolVersion) ==
      (NoktaV1Client.cundaDeviceType(), NoktaV1Client.cundaRpcProtocol())) {
    final noktaClient = NoktaV1Client(base: client);
    noktaClient.approveFirmware(req: NoArg());
    _log.fine("Returning a nokta client");
    return noktaClient;
  } else if ((deviceId.deviceType, deviceId.protocolVersion) ==
      (FakeDevV1Client.cundaDeviceType(), FakeDevV1Client.cundaRpcProtocol())) {
    final fakeClient = FakeDevV1Client();
    fakeClient.approveFirmware(req: NoArg());
    _log.fine("Returning a fake client");
    return fakeClient;
  } else {
    _log.warning("Trying to connect to an unrecognized device");
    throw Exception("Not a recognized device, protocol pair");
  }
}
