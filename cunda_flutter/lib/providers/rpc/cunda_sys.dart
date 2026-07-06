import 'dart:async';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common/v1/endpoints.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/cunda_common/v1/types.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/types.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'cunda_sys.g.dart';

final _log = Logger('CundaSysProvider');

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<CundaSysE> sysEndpoints(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(protocolClientProvider(device).future);
  return client as CundaSysE;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<CundaSysT> sysTopics(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(protocolClientProvider(device).future);
  return client as CundaSysT;
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

@riverpod
Future<int> getMtuFromDevice(Ref ref, BluetoothDevice device) async {
  final sysD = await ref.watch(sysEndpointsProvider(device).future);
  _log.fine("Calling getMtu RPC endpoint");
  return await sysD.getMtu(req: NoArg());
}

@riverpod
Stream<Duration> pingStream(Ref ref, BluetoothDevice device) async* {
  final sysD = await ref.watch(sysEndpointsProvider(device).future);
  Stopwatch stopwatch = Stopwatch();
  while (true) {
    stopwatch.start();
    _log.fine("Started the clock, pinging");
    await sysD.pingEndpoint(req: NoArg());
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
  final sysD = await ref.watch(sysEndpointsProvider(device).future);

  // Subscribe to the topic
  final tDispatcher = await ref.watch(sysTopicsProvider(device).future);
  final sysStatsStream = tDispatcher.createSysStatsTopicStream();

  await sysD.startSysStatsTopic(req: NoArg());
  ref.onCancel(() async {
    _log.fine("Stop system stats stream");
    await sysD.stopSysStatsTopic(req: NoArg());
  });
  ref.onResume(() async {
    _log.fine("Resume system stats stream");
    await sysD.startSysStatsTopic(req: NoArg());
  });
  ref.onDispose(() {
    _log.fine("Disposing system stats stream");
  });
  yield* sysStatsStream;
}
