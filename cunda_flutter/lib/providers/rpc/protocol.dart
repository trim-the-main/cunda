import 'dart:async';

import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/rpc.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/v1/endpoints.dart';
import 'package:cunda_flutter/frb_generated/rust_lib_cunda_flutter/third_party/protocol/v1/topics.dart';
import 'package:cunda_flutter/providers/rpc/client.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'protocol.g.dart';

final log = Logger('ProtocolProvider');

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<EndpointDispatcher> endpointDispatcher(
  Ref ref,
  BluetoothDevice device,
) async {
  final client = await ref.watch(rpcClientProvider(device).future);
  return client as EndpointDispatcher;
}

@Riverpod(keepAlive: true, retry: noRetry)
FutureOr<TopicDispatcher> topicDispatcher(
  Ref ref,
  BluetoothDevice device,
) async {
  final client = await ref.watch(rpcClientProvider(device).future);
  return client as TopicDispatcher;
}

// A simple wrapper around button events stream. We attach timestamps
// to the button events.
@Riverpod(keepAlive: true, retry: noRetry)
Stream<(DateTime, ButtonEvent)> gpioButtonEvents(
  Ref ref,
  BluetoothDevice device,
) async* {
  final eDispatcher = await ref.watch(
    endpointDispatcherProvider(device).future,
  );
  final tDispatcher = await ref.watch(topicDispatcherProvider(device).future);
  final buttonEventsStream = tDispatcher.createButtonEventsStream();

  eDispatcher.startButtonEventsTopic(req: NoArg());
  ref.onCancel(() {
    log.warning("Stop button events stream, we got canceled");
    eDispatcher.stopButtonEventsTopic(req: NoArg());
  });
  ref.onResume(() {
    log.warning("Resume button events stream");
    eDispatcher.startButtonEventsTopic(req: NoArg());
  });
  ref.onDispose(() {
    log.severe("BUTTON EVENT STREAM IS GOING AWAY");
  });
  await for (final event in buttonEventsStream) {
    log.fine("Yielding button event: $event");
    yield (DateTime.now(), event);
  }
}

@Riverpod(keepAlive: true, retry: noRetry)
class SystemSettings extends _$SystemSettings {
  @override
  FutureOr<SysSettings> build(BluetoothDevice device) async {
    final eDispatcher = await ref.watch(
      endpointDispatcherProvider(device).future,
    );
    log.fine("Calling getSysSettings");
    return eDispatcher.getSysSettings(req: NoArg());
  }

  void save(SysSettings newSettings) async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(
      () => ref
          .read(endpointDispatcherProvider(device).future)
          .then(
            (eDispatcher) => eDispatcher
                .setSysSettings(req: newSettings)
                .then((_) => eDispatcher.getSysSettings(req: NoArg())),
          ),
    );
  }
}

@Riverpod(keepAlive: true, retry: noRetry)
class ApplicationSettings extends _$ApplicationSettings {
  @override
  FutureOr<ApplSettings> build(BluetoothDevice device) async {
    final eDispatcher = await ref.watch(
      endpointDispatcherProvider(device).future,
    );
    log.fine("Calling getApplSettings");
    return eDispatcher.getApplSettings(req: NoArg());
  }

  void save(ApplSettings newSettings) async {
    state = const AsyncValue.loading();
    state = await AsyncValue.guard(
      () => ref
          .read(endpointDispatcherProvider(device).future)
          .then(
            (eDispatcher) => eDispatcher
                .setApplSettings(req: newSettings)
                .then((_) => eDispatcher.getApplSettings(req: NoArg())),
          ),
    );
  }
}
