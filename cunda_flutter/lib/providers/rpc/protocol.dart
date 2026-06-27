import 'dart:async';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc/client.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common/v1/endpoints.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common/v1/types.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/devices/demo_esp32/v1/endpoints.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/devices/demo_esp32/v1/types.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/types.dart';
import 'package:cunda_flutter/providers/rpc/client.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'protocol.g.dart';

final _log = Logger('ProtocolProvider');

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<CundaSysE> sysEndpoints(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(rpcClientProvider(device).future);
  return client as CundaSysE;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<DemoAppEndpoints> appEndpoints(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(rpcClientProvider(device).future);
  return client as DemoAppEndpoints;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<CundaSysT> sysTopics(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(rpcClientProvider(device).future);
  return client as CundaSysT;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<DemoAppTopics> appTopics(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(rpcClientProvider(device).future);
  return client as DemoAppTopics;
}

// A simple wrapper around button events stream. We attach timestamps
// to the button events.
@Riverpod(keepAlive: true, retry: noRetry)
Stream<(DateTime, ButtonEvent)> gpioButtonEvents(
  Ref ref,
  BluetoothDevice device,
) async* {
  final eDispatcher = await ref.watch(appEndpointsProvider(device).future);
  final tDispatcher = await ref.watch(appTopicsProvider(device).future);
  final buttonEventsStream = tDispatcher.createButtonEventsStream();

  eDispatcher.startButtonEventsTopic(req: NoArg());
  ref.onCancel(() {
    _log.warning("Stop button events stream, we got canceled");
    eDispatcher.stopButtonEventsTopic(req: NoArg());
  });
  ref.onResume(() {
    _log.warning("Resume button events stream");
    eDispatcher.startButtonEventsTopic(req: NoArg());
  });
  await for (final event in buttonEventsStream) {
    _log.fine("Yielding button event: $event");
    yield (DateTime.now(), event);
  }
}

@Riverpod(keepAlive: true, retry: noRetry)
class SystemSettings extends _$SystemSettings {
  @override
  FutureOr<SysSettings> build(BluetoothDevice device) async {
    final sysD = await ref.watch(sysEndpointsProvider(device).future);
    _log.fine("Calling getSysSettings");
    return sysD.getSysSettings(req: NoArg());
  }

  void save(SysSettings newSettings) async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(
      () => ref
          .read(sysEndpointsProvider(device).future)
          .then(
            (sysD) => sysD
                .setSysSettings(req: newSettings)
                .then((_) => sysD.getSysSettings(req: NoArg())),
          ),
    );
  }
}

@Riverpod(keepAlive: true, retry: noRetry)
class ApplicationSettings extends _$ApplicationSettings {
  @override
  FutureOr<ApplSettings> build(BluetoothDevice device) async {
    final appD = await ref.watch(appEndpointsProvider(device).future);
    _log.fine("Calling getApplSettings");
    return appD.getApplSettings(req: NoArg());
  }

  void save(ApplSettings newSettings) async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(
      () => ref
          .read(appEndpointsProvider(device).future)
          .then(
            (appD) => appD
                .setApplSettings(req: newSettings)
                .then((_) => appD.getApplSettings(req: NoArg())),
          ),
    );
  }
}

@riverpod
Future<int> getMtuFromDevice(Ref ref, BluetoothDevice device) async {
  final sysD = await ref.watch(sysEndpointsProvider(device).future);
  _log.fine("Calling getMtu RPC endpoint");
  return await sysD.getMtu(req: NoArg());
}

@Riverpod(keepAlive: true, retry: noRetry)
Future<DeviceId> deviceId(Ref ref, BluetoothDevice device) async {
  final sysD = await ref.watch(rpcClientProvider(device).future);
  _log.fine("Calling getDeviceId RPC endpoint");
  return sysD.getDeviceId(req: NoArg());
}
