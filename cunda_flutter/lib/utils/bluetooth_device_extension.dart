import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';

final log = Logger('BleDeviceExtension');

extension Extra on BluetoothDevice {
  String get chosenName {
    return advName == ""
        ? platformName == ""
              ? remoteId.toString()
              : platformName
        : advName;
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
