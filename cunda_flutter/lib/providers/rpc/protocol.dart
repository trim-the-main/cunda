import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/demo_esp32.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/fake_dev.dart';
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
  final deviceId = await ref.watch(deviceIdProvider(device).future);

  if ((deviceId.deviceType, deviceId.protocolVersion) ==
      (DemoV1Client.cundaDeviceType(), DemoV1Client.cundaRpcProtocol())) {
    final demoClient = DemoV1Client(base: client);
    demoClient.approveFirmware(req: NoArg());
    return demoClient;
  } else if ((deviceId.deviceType, deviceId.protocolVersion) ==
      (FakeDevV1Client.cundaDeviceType(), FakeDevV1Client.cundaRpcProtocol())) {
    final fakeClient = FakeDevV1Client();
    fakeClient.approveFirmware(req: NoArg());
    return fakeClient;
  } else {
    throw Exception("Not a recognized device, protocol pair");
  }
}
