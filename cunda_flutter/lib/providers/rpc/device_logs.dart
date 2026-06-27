import 'dart:collection';
import 'dart:typed_data';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/defmt_log_translation.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/types.dart';
import 'package:cunda_flutter/providers/package_registry_provider.dart';
import 'package:cunda_flutter/providers/rpc/client.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/services/mayna/mayna_types.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part "device_logs.g.dart";

final _log = Logger('DeviceLogs');

@Riverpod(keepAlive: true)
class LogTopicEnabled extends _$LogTopicEnabled {
  @override
  bool build(BluetoothDevice device) => false;

  void toggle() => state = !state;
  void set(bool value) => state = value;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<LogDecoder> logDecoder(Ref ref, BluetoothDevice device) async {
  final client = await ref.watch(rpcClientProvider(device).future);
  return client as LogDecoder;
}

@Riverpod(keepAlive: true, retry: noRetry)
Future<LogDecoder?> initializedLogDecoder(
  Ref ref,
  BluetoothDevice device,
) async {
  final deviceId = await ref.watch(deviceIdProvider(device).future);
  final registry = await ref.watch(packageRegistryProvider.future);

  final tableFile = registry.componentPath(
    deviceId.deviceType,
    deviceId.firmwareVersion,
    ComponentType.defmtTable,
  );
  if (tableFile == null) {
    _log.info(
      'No defmt table available for '
      '${deviceId.deviceType}/${deviceId.firmwareVersion}',
    );
    return null;
  }

  final locFile = registry.componentPath(
    deviceId.deviceType,
    deviceId.firmwareVersion,
    ComponentType.defmtLocations,
  );

  final decoder = await ref.watch(logDecoderProvider(device).future);
  await decoder.initLogDecoder(
    tableBytes: await tableFile.readAsBytes(),
    locBytes: locFile != null ? await locFile.readAsBytes() : [],
  );
  _log.info('Log decoder initialized');
  return decoder;
}

// defmt logs coming through a topic
@Riverpod(keepAlive: true, retry: noRetry)
Stream<Uint8List> deviceLogs(Ref ref, BluetoothDevice device) async* {
  final sysD = await ref.watch(sysEndpointsProvider(device).future);
  final tDispatcher = await ref.watch(sysTopicsProvider(device).future);
  final logStream = tDispatcher.createSysLogsTopicStream();

  sysD.startSysLogsTopic(req: NoArg());
  bool keepRunning = ref.read(logTopicEnabledProvider(device));
  ref.listen(logTopicEnabledProvider(device), (_, next) => keepRunning = next);
  ref.onCancel(() {
    if (!keepRunning) {
      _log.warning("Stop sys logs stream, we got canceled");
      sysD.stopSysLogsTopic(req: NoArg());
    }
  });
  ref.onResume(() {
    if (!keepRunning) {
      _log.warning("Resume sys logs stream");
      sysD.startSysLogsTopic(req: NoArg());
    }
  });
  await for (final logMsg in logStream) {
    yield logMsg.defmtBytes;
    _log.info("[DEVICE LOG] ${logMsg.defmtBytes}");
  }
}

const _maxDecodedLogEntries = 500;

@Riverpod(keepAlive: true, retry: noRetry)
class DecodedDeviceLogs extends _$DecodedDeviceLogs {
  final Queue<DefmtLogEntry> _queue = Queue();

  @override
  Queue<DefmtLogEntry>? build(BluetoothDevice device) {
    final decoder = ref.watch(initializedLogDecoderProvider(device)).value;
    if (decoder == null) return null;

    ref.listen(deviceLogsProvider(device), (_, next) {
      final bytes = next.value;
      if (bytes == null) return;
      try {
        final entries = decoder.decodeLog(bytes: bytes);
        _queue.addAll(entries);
        while (_queue.length > _maxDecodedLogEntries) {
          _queue.removeFirst();
        }
        ref.notifyListeners();
      } catch (e) {
        _log.warning('Failed to decode log: $e');
      }
    });

    return _queue;
  }
}
