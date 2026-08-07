import 'dart:math';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/demo_esp32.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/fake_dev.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/mocks/mock_demo_esp32.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/mocks/mock_nokta.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/devices/mocks/mock_tirbod.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/log_decoder.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common/v1/endpoints.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/devices/demo_esp32/v1/endpoints.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/types.dart';
import 'package:cunda_flutter/providers/ble/ble_providers.dart';
import 'package:cunda_flutter/providers/rpc/cunda_gps.dart';
import 'package:cunda_flutter/providers/rpc/cunda_sys.dart';
import 'package:cunda_flutter/providers/rpc/device_logs.dart';
import 'package:cunda_flutter/providers/rpc/devices/demo_esp32.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/services/ble/ble_fake_impl.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

final _dummyClients = <String, Object>{};
final _deviceIds = <String, DeviceId>{};

final List<Object Function()> possibleClientConstructors = [
  FakeDevV1Client.new,
  MockDemoV1Client.new,
  MockNoktaV1Client.new,
  MockTirbodV1Client.new,
];

Object _clientFor(BluetoothDevice device) {
  final random = Random();
  final f =
      possibleClientConstructors[random.nextInt(
        possibleClientConstructors.length,
      )];
  return _dummyClients.putIfAbsent(device.remoteId.str, f);
}

Future<DeviceId> _deviceIdFor(BluetoothDevice device) async {
  if (_deviceIds.containsKey(device.remoteId.str)) {
    return _deviceIds[device.remoteId.str]!;
  }

  final client = _clientFor(device);
  final deviceId = await (client as CundaDevice).getDeviceId(req: NoArg());
  return _deviceIds.putIfAbsent(device.remoteId.str, () => deviceId);
}

List<Override> fakeProviderOverrides() {
  return [
    bleServiceProvider.overrideWithValue(FakeBleService()),
    deviceIdProtocolClientPairProvider.overrideWith(
      (ref, device) async => (await _deviceIdFor(device), _clientFor(device)),
    ),
    appEndpointsProvider.overrideWith(
      (ref, device) async => _clientFor(device) as DemoAppEndpoints,
    ),
    gpsEndpointsProvider.overrideWith(
      (ref, device) async => _clientFor(device) as CundaGpsE,
    ),
    sysEndpointsProvider.overrideWith(
      (ref, device) async => _clientFor(device) as CundaSysE,
    ),
    appTopicsProvider.overrideWith(
      (ref, device) async => _clientFor(device) as DemoAppTopics,
    ),
    gpsTopicsProvider.overrideWith(
      (ref, device) async => _clientFor(device) as CundaGpsT,
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
