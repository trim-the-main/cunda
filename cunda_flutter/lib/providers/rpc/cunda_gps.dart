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

part 'cunda_gps.g.dart';

final _log = Logger('CundaGpsProvider');

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<CundaGpsE> gpsEndpoints(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(protocolClientProvider(device).future);
  return client as CundaGpsE;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<CundaGpsT> gpsTopics(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(protocolClientProvider(device).future);
  return client as CundaGpsT;
}

@Riverpod(keepAlive: true, retry: noRetry)
Stream<String> rawNmeaSentenceStream(Ref ref, BluetoothDevice device) async* {
  // Subscribe to the topic
  final tDispatcher = await ref.watch(gpsTopicsProvider(device).future);
  final rawNmeaStream = tDispatcher.createRawNmeaTopicStream();

  yield* rawNmeaStream.map((r) => r.field0);
}

@Riverpod(keepAlive: true, retry: noRetry)
Stream<GpsDataWire> parsedGpsStream(Ref ref, BluetoothDevice device) async* {
  _log.info("Parsed gps data stream");
  final gpsD = await ref.watch(gpsEndpointsProvider(device).future);

  // Subscribe to the topic
  final tDispatcher = await ref.watch(gpsTopicsProvider(device).future);
  final gpsDataStream = tDispatcher.createParsedGpsTopicStream();

  await gpsD.startParsedGpsTopic(req: NoArg());
  ref.onCancel(() async {
    _log.fine("Stop pasrsed gps stream");
    await gpsD.stopParsedGpsTopic(req: NoArg());
  });
  ref.onResume(() async {
    _log.fine("Resume parsed gps stream");
    await gpsD.startParsedGpsTopic(req: NoArg());
  });
  ref.onDispose(() {
    _log.fine("Disposing parsed gps stream");
  });
  yield* gpsDataStream;
}

@Riverpod()
class RawNmeaTopicEnabled extends _$RawNmeaTopicEnabled {
  @override
  Future<bool> build(BluetoothDevice device) async {
    state = AsyncValue.loading();
    final gpsD = await ref.watch(gpsEndpointsProvider(device).future);
    await gpsD.stopRawNmeaTopic(req: NoArg());
    return false;
  }

  Future<void> set(bool value) async {
    if (state == AsyncValue.data(value)) {
      return;
    }

    state = AsyncValue.loading();
    state = await AsyncValue.guard(() async {
      final gpsD = await ref.read(gpsEndpointsProvider(device).future);
      if (value) {
        await gpsD.startRawNmeaTopic(req: NoArg());
      } else {
        await gpsD.stopRawNmeaTopic(req: NoArg());
      }
      return value;
    });
  }
}
