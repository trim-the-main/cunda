import 'package:flutter/foundation.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';

final log = Logger('BleDeviceExtension');

enum ConnectionTransition { noTransition, connecting, disconnecting }

final Map<DeviceIdentifier, ValueNotifier<ConnectionTransition>> _transitions =
    {};

extension Extra on BluetoothDevice {
  String get chosenName {
    return advName == ""
        ? platformName == ""
              ? remoteId.toString()
              : platformName
        : advName;
  }

  // This is just for convenience, it is not global source of truth. For example
  // the disconnect command could be invoked from outside the app too. Or even
  // if the app invoke the connect and disconnect futures at the same time
  // the library will serialize them using a mutex but here we don't do that
  // so the ValueNotifier may be showing no transition while there is a transition
  // in flight.
  // This should be eventually consistent I guess.
  ValueNotifier<ConnectionTransition> get transitionState {
    _transitions[remoteId] ??= ValueNotifier<ConnectionTransition>(
      ConnectionTransition.noTransition,
    );
    return _transitions[remoteId]!;
  }

  // connect & update stream
  Future<void> connectTrackingTransitionState({Duration? timeout}) async {
    transitionState.value = ConnectionTransition.connecting;
    try {
      if (timeout != null) {
        log.fine("Connecting with timeout $timeout");
        await connect(license: License.free, mtu: null, timeout: timeout);
        log.fine("Connect returned");
      } else {
        log.fine("Connecting with timeout $timeout");
        await connect(license: License.free, mtu: null);
        log.fine("Connect returned");
      }
    } finally {
      transitionState.value = ConnectionTransition.noTransition;
    }
  }

  Future<void> disconnectTrackingTransitionState({int? timeout}) async {
    transitionState.value = ConnectionTransition.disconnecting;
    try {
      if (timeout != null) {
        await disconnect(timeout: timeout);
      } else {
        await disconnect();
      }
    } finally {
      transitionState.value = ConnectionTransition.noTransition;
    }
  }

  Stream<int> rssiStream(Duration pollingInterval) async* {
    while (true) {
      yield await readRssi();
      await Future.delayed(pollingInterval);
    }
  }

  Stream<double> mtuStream() async* {
    yield mtuNow.toDouble();
    yield* mtu.map((mtu) => mtu.toDouble());
  }
}

enum SignalStrength {
  poor,
  fair,
  good,
  excellent;

  @override
  String toString() {
    switch (this) {
      case SignalStrength.poor:
        return "Poor";
      case SignalStrength.fair:
        return "Fair";
      case SignalStrength.good:
        return "Good";
      case SignalStrength.excellent:
        return "Excellent";
    }
  }

  static SignalStrength fromRssiValue(int rssi) {
    if (rssi > -64) {
      return SignalStrength.excellent;
    } else if (rssi > -74) {
      return SignalStrength.good;
    } else if (rssi > -84) {
      return SignalStrength.fair;
    } else {
      return SignalStrength.poor;
    }
  }
}
