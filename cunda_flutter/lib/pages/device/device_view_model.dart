// State that only the device page widgets depend on

import 'dart:math';
import 'dart:typed_data';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/v1.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/types.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/v1/endpoints.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part "device_view_model.g.dart";

final _log = Logger('DeviceViewModel');

@Riverpod(keepAlive: true, retry: noRetry)
Future<DeviceId> deviceId(Ref ref, BluetoothDevice device) async {
  final eDispatcher = await ref.watch(
    endpointDispatcherProvider(device).future,
  );
  _log.fine("Calling getDeviceId RPC endpoint");
  return eDispatcher.getDeviceId(req: NoArg());
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

/// Downstream bandwidth: subscribes to BandwidthTestTopic and measures incoming bytes/sec.
/// Uses keepAlive so the topic stream is created only once. Start/stop is
/// controlled by watching/unwatching from the widget, triggering
/// onCancel (stopTestTopicBandwidth) / onResume (startTestTopicBandwidth) — same pattern as
/// systemStatsStream.
@Riverpod(keepAlive: true, retry: noRetry)
Stream<String> downstreamBandwidth(Ref ref, BluetoothDevice device) async* {
  _log.info("Downstream bandwidth stream");
  final eDispatcher = await ref.watch(
    endpointDispatcherProvider(device).future,
  );
  final tDispatcher = await ref.watch(topicDispatcherProvider(device).future);
  final bandwidthTopicStream = tDispatcher.createBandwidthTestTopicStream();

  await eDispatcher.startTestTopicBandwidth(req: NoArg());
  ref.onCancel(() async {
    _log.info("onCancel fired: stopping bandwidth test topic");
    try {
      await eDispatcher.stopTestTopicBandwidth(req: NoArg());
      _log.info("stopTestTopicBandwidth completed successfully");
    } catch (e) {
      _log.warning("stopTestTopicBandwidth failed: $e");
    }
  });
  ref.onResume(() async {
    _log.fine("Resume bandwidth test topic (downstream bandwidth resumed)");
    await eDispatcher.startTestTopicBandwidth(req: NoArg());
  });
  ref.onDispose(() {
    _log.fine("Disposing downstream bandwidth stream");
  });

  int bytesReceived = 0;
  DateTime lastReport = DateTime.now();

  await for (final msg in bandwidthTopicStream) {
    bytesReceived += msg.nums.lengthInBytes;
    final now = DateTime.now();
    final elapsed = now.difference(lastReport);
    if (elapsed.inMilliseconds >= 1000) {
      final kbps = bytesReceived / 1024 / (elapsed.inMilliseconds / 1000);
      yield '${kbps.toStringAsFixed(2)} KB/s';
      bytesReceived = 0;
      lastReport = now;
    }
  }
}

/// Upstream bandwidth: sends BandwidthTestData via testBandwidth in a loop, measures bytes/sec.
/// Auto-dispose: the loop runs while watched and stops when unwatched.
/// Uses the same loop + ref.mounted pattern as pingStream.
@riverpod
Stream<String> upstreamBandwidth(Ref ref, BluetoothDevice device) async* {
  _log.info("Upstream bandwidth stream");
  final eDispatcher = await ref.watch(
    endpointDispatcherProvider(device).future,
  );

  final rng = Random();
  int bytesSent = 0;
  DateTime lastReport = DateTime.now();

  while (true) {
    final payload = BandwidthTestData(
      data: Uint8List.fromList(List.generate(2048, (_) => rng.nextInt(1 << 8))),
    );
    if (!ref.mounted) {
      _log.fine("Upstream bandwidth: ref unmounted, stopping");
      break;
    }

    try {
      await eDispatcher.testBandwidth(req: payload);
      bytesSent += payload.data.lengthInBytes;
    } catch (e) {
      _log.warning("testBandwidth error: $e");
      break;
    }

    if (!ref.mounted) {
      _log.fine("Upstream bandwidth: ref unmounted after send, stopping");
      break;
    }

    final now = DateTime.now();
    final elapsed = now.difference(lastReport);
    if (elapsed.inMilliseconds >= 1000) {
      final kbps = bytesSent / 1024 / (elapsed.inMilliseconds / 1000);
      yield '${kbps.toStringAsFixed(2)} KB/s';
      bytesSent = 0;
      lastReport = now;
    }
  }
}
