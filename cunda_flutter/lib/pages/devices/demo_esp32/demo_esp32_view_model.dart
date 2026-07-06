import 'dart:math';
import 'dart:typed_data';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/devices/demo_esp32/v1/types.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/types.dart';
import 'package:cunda_flutter/providers/rpc/devices/demo_esp32.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part "demo_esp32_view_model.g.dart";

final _log = Logger('DemoEsp32ViewModel');

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

/// Downstream bandwidth: subscribes to BandwidthTestTopic and measures incoming bytes/sec.
/// Uses keepAlive so the topic stream is created only once. Start/stop is
/// controlled by watching/unwatching from the widget, triggering
/// onCancel (stopTestTopicBandwidth) / onResume (startTestTopicBandwidth) — same pattern as
/// systemStatsStream.
@Riverpod(keepAlive: true, retry: noRetry)
Stream<String> downstreamBandwidth(Ref ref, BluetoothDevice device) async* {
  _log.info("Downstream bandwidth stream");
  final appD = await ref.watch(appEndpointsProvider(device).future);
  final tDispatcher = await ref.watch(appTopicsProvider(device).future);
  final bandwidthTopicStream = tDispatcher.createBandwidthTestTopicStream();

  await appD.startTestTopicBandwidth(req: NoArg());
  ref.onCancel(() async {
    _log.info("onCancel fired: stopping bandwidth test topic");
    try {
      await appD.stopTestTopicBandwidth(req: NoArg());
      _log.info("stopTestTopicBandwidth completed successfully");
    } catch (e) {
      _log.warning("stopTestTopicBandwidth failed: $e");
    }
  });
  ref.onResume(() async {
    _log.fine("Resume bandwidth test topic (downstream bandwidth resumed)");
    await appD.startTestTopicBandwidth(req: NoArg());
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
  final appD = await ref.watch(appEndpointsProvider(device).future);

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
      await appD.testBandwidth(req: payload);
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
