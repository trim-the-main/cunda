import 'package:flutter_blue_plus/flutter_blue_plus.dart';

abstract class BleAdapter {
  bool get adapterStateNow;
  Stream<BluetoothAdapterState> get adapterState;

  Future<void> setLogLevel(LogLevel level, {color = true});
  LogLevel get logLevel;

  Future<String> get adapterName;
}

abstract class BleScanner {
  bool get isScanningNow;
  Stream<bool> get isScanning;
  Future<void> startScan({
    List<Guid> withServices = const [],
    Duration? timeout,
    Duration? removeIfGone,
    bool oneByOne = false,
    bool androidUsesFineLocation = false,
  });
  Future<void> stopScan();

  Stream<List<ScanResult>> get scanResults;
  Future<List<BluetoothDevice>> systemDevices(List<Guid> withServices);
}

abstract class BleService implements BleAdapter, BleScanner {}
