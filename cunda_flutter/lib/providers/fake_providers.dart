import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc/client.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc/dummy_client.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common/v1/endpoints.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/devices/demo_esp32/v1/endpoints.dart';
import 'package:cunda_flutter/providers/ble/ble_providers.dart';
import 'package:cunda_flutter/providers/rpc/client.dart';
import 'package:cunda_flutter/providers/rpc/device_logs.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/services/ble/ble_fake_impl.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

final _dummyClients = <String, DummyFlutterProtocolClient>{};

DummyFlutterProtocolClient _clientFor(BluetoothDevice device) {
  return _dummyClients.putIfAbsent(
    device.remoteId.str,
    DummyFlutterProtocolClient.new,
  );
}

List<Override> fakeProviderOverrides() {
  return [
    bleServiceProvider.overrideWithValue(FakeBleService()),
    cundaDeviceClientProvider.overrideWith(
      (ref, device) async => _clientFor(device) as CundaDevice,
    ),
    appEndpointsProvider.overrideWith(
      (ref, device) async => _clientFor(device) as DemoAppEndpoints,
    ),
    sysEndpointsProvider.overrideWith(
      (ref, device) async => _clientFor(device) as CundaSysE,
    ),
    appTopicsProvider.overrideWith(
      (ref, device) async => _clientFor(device) as DemoAppTopics,
    ),
    sysTopicsProvider.overrideWith(
      (ref, device) async => _clientFor(device) as CundaSysT,
    ),
    logDecoderProvider.overrideWith(
      (ref, device) async => _clientFor(device) as LogDecoder,
    ),
    initializedLogDecoderProvider.overrideWith((ref, device) async {
      final decoder = _clientFor(device) as LogDecoder;
      await decoder.initLogDecoder(tableBytes: [], locBytes: []);
      return decoder;
    }),
  ];
}
