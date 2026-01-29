// State that only the device page widgets depend on

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/v1.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/v1/endpoints.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part "device_view_model.g.dart";

final _log = Logger('DeviceViewModel');

@Riverpod(keepAlive: true, retry: noRetry)
Future<String> firmwareVersion(Ref ref, BluetoothDevice device) async {
  final eDispatcher = await ref.watch(
    endpointDispatcherProvider(device).future,
  );
  _log.fine("Calling getFirmware RPC endpoint");
  final res = await eDispatcher.getFirmwareVersion(req: NoArg());
  return res;
}

@riverpod
Stream<Duration> pingStream(Ref ref, BluetoothDevice device) async* {
  final eDispatcher = await ref.watch(
    endpointDispatcherProvider(device).future,
  );
  Stopwatch stopwatch = Stopwatch();
  while (true) {
    stopwatch.start();
    _log.fine("Started the clock, pinging");
    await eDispatcher.pingEndpoint(req: NoArg());
    stopwatch.stop();
    if (!ref.mounted) {
      _log.fine(
        "Ping took ${stopwatch.elapsed.inMilliseconds}. But not yielding as ref.mounted is false",
      );
      break;
    }
    _log.fine("Ping took ${stopwatch.elapsed.inMilliseconds}. Yielding this");
    yield stopwatch.elapsed;
    stopwatch.reset();
    await Future.delayed(Duration(seconds: 1));
    if (!ref.mounted) {
      break;
    }
  }
}

@Riverpod(keepAlive: true, retry: noRetry)
Stream<SysStats> systemStatsStream(Ref ref, BluetoothDevice device) async* {
  _log.info("System stats stream");
  final eDispatcher = await ref.watch(
    endpointDispatcherProvider(device).future,
  );

  // Subscribe to the topic
  final tDispatcher = await ref.watch(topicDispatcherProvider(device).future);
  final sysStatsStream = tDispatcher.createSysStatsTopicStream();

  await eDispatcher.startSysStatsTopic(req: NoArg());
  ref.onCancel(() async {
    _log.fine("Stop system stats stream");
    await eDispatcher.stopSysStatsTopic(req: NoArg());
  });
  ref.onResume(() async {
    _log.fine("Resume system stats stream");
    await eDispatcher.startSysStatsTopic(req: NoArg());
  });
  ref.onDispose(() {
    _log.fine("Disposing system stats stream");
  });
  yield* sysStatsStream;
}
