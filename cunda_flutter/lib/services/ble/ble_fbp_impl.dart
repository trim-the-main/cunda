import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:cunda_flutter/services/ble/ble.dart';

class BleServiceFBP implements BleService {
  @override
  bool get adapterStateNow {
    return FlutterBluePlus.adapterStateNow == BluetoothAdapterState.on;
  }

  @override
  Stream<BluetoothAdapterState> get adapterState {
    return FlutterBluePlus.adapterState;
  }

  @override
  Future<void> startScan({
    List<Guid> withServices = const [],
    Duration? timeout,
    Duration? removeIfGone,
    bool oneByOne = false,
    bool androidUsesFineLocation = false,
  }) {
    return FlutterBluePlus.startScan(
      withServices: withServices,
      timeout: timeout,
      removeIfGone: removeIfGone,
      oneByOne: oneByOne,
      androidUsesFineLocation: androidUsesFineLocation,
    );
  }

  @override
  Stream<List<ScanResult>> get scanResults {
    return FlutterBluePlus.scanResults;
  }

  @override
  Future<List<BluetoothDevice>> systemDevices(List<Guid> withServices) {
    return FlutterBluePlus.systemDevices(withServices);
  }

  @override
  bool get isScanningNow {
    return FlutterBluePlus.isScanningNow;
  }

  @override
  Stream<bool> get isScanning {
    return FlutterBluePlus.isScanning;
  }

  @override
  Future<void> stopScan() {
    return FlutterBluePlus.stopScan();
  }

  @override
  Future<void> setLogLevel(LogLevel level, {color = true}) {
    return FlutterBluePlus.setLogLevel(level, color: color);
  }

  @override
  LogLevel get logLevel {
    return FlutterBluePlus.logLevel;
  }

  @override
  Future<String> get adapterName {
    return FlutterBluePlus.adapterName;
  }

  List<BluetoothDevice> get connectedDevices {
    return FlutterBluePlus.connectedDevices;
  }

  Stream<void> get onConnectionStateChanged {
    return FlutterBluePlus.events.onConnectionStateChanged;
  }
}
