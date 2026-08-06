import 'dart:async';
import 'dart:math';

import 'package:cunda_flutter/services/ble/ble.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:logging/logging.dart';

final _log = Logger('BleFake');

String _generateRandomMacAddress() {
  final random = Random();
  final hexDigits = '0123456789ABCDEF';
  final macAddress = List.generate(6, (index) {
    return hexDigits[random.nextInt(16)] + hexDigits[random.nextInt(16)];
  }).join(':');
  return macAddress;
}

final List<BluetoothDevice> _connectedFakeDevices = [];
final StreamController<void> _connectedFakeDevicesStreamController =
    StreamController.broadcast();

class FakeBluetoothDevice extends BluetoothDevice {
  Duration actionDelay = Duration(seconds: 1);
  FakeBluetoothDevice() : super.fromId(_generateRandomMacAddress());

  void setActionDelay(Duration delay) {
    actionDelay = delay;
  }

  @override
  String get advName => "FakeDev ${remoteId.hashCode % 100}";

  bool _isConnected = false;
  @override
  bool get isConnected => _isConnected;

  final StreamController<BluetoothConnectionState> _conStateController =
      StreamController.broadcast();

  @override
  Stream<BluetoothConnectionState> get connectionState =>
      _conStateController.stream;

  @override
  Future<void> connect({
    required License license,
    Duration timeout = const Duration(seconds: 35),
    int? mtu = 512,
    bool autoConnect = false,
  }) {
    _log.fine("FakeBluetoothDevice.connect");
    return Future.delayed(actionDelay, () {
      _isConnected = true;
      _conStateController.sink.add(BluetoothConnectionState.connected);
      _connectedFakeDevices.add(this);
      _connectedFakeDevicesStreamController.sink.add(null);
    });
  }

  @override
  Future<void> disconnect({
    int timeout = 35,
    bool queue = true,
    int androidDelay = 2000,
  }) {
    return Future.delayed(actionDelay, () {
      _isConnected = false;
      _conStateController.sink.add(BluetoothConnectionState.disconnected);
      _connectedFakeDevices.remove(this);
      _connectedFakeDevicesStreamController.sink.add(null);
    });
  }

  int rssi = -40;
  @override
  Future<int> readRssi({int timeout = 15}) async {
    if (rssi < -90) {
      rssi = -44;
    }
    return rssi -= 4;
  }
}

class FakeBleService implements BleService {
  final List<FakeBluetoothDevice> _fakeDevices = [
    FakeBluetoothDevice(),
    FakeBluetoothDevice(),
    FakeBluetoothDevice(),
    FakeBluetoothDevice(),
    FakeBluetoothDevice(),
    FakeBluetoothDevice(),
    FakeBluetoothDevice(),
    FakeBluetoothDevice(),
    FakeBluetoothDevice(),
  ];

  bool _isScanning = false;
  final StreamController<bool> _isScanningController =
      StreamController.broadcast();
  final StreamController<List<ScanResult>> _scanResultsController =
      StreamController.broadcast();

  @override
  bool get adapterStateNow => true;

  @override
  Stream<BluetoothAdapterState> get adapterState =>
      Stream.value(BluetoothAdapterState.on);

  @override
  Future<void> setLogLevel(LogLevel level, {color = true}) async {}

  @override
  LogLevel get logLevel => LogLevel.info;

  @override
  Future<String> get adapterName async => 'FakeAdapter';

  @override
  bool get isScanningNow => _isScanning;

  @override
  Stream<bool> get isScanning => _isScanningController.stream;

  @override
  Future<void> startScan({
    List<Guid> withServices = const [],
    Duration? timeout,
    Duration? removeIfGone,
    bool oneByOne = false,
    bool androidUsesFineLocation = false,
  }) async {
    _isScanning = true;
    _isScanningController.add(true);
    _log.info('Fake scan started');

    await Future.delayed(const Duration(milliseconds: 500));

    _scanResultsController.add(
      _fakeDevices
          .map(
            (d) => ScanResult(
              device: d,
              advertisementData: AdvertisementData(
                advName: d.advName,
                txPowerLevel: null,
                appearance: null,
                connectable: true,
                manufacturerData: {},
                serviceData: {},
                serviceUuids: [],
              ),
              rssi: -50,
              timeStamp: DateTime.now(),
            ),
          )
          .toList(),
    );

    if (timeout != null) {
      Future.delayed(timeout, stopScan);
    }
  }

  @override
  Future<void> stopScan() async {
    _isScanning = false;
    _isScanningController.add(false);
    _log.info('Fake scan stopped');
  }

  @override
  Stream<List<ScanResult>> get scanResults => _scanResultsController.stream;

  @override
  Future<List<BluetoothDevice>> systemDevices(List<Guid> withServices) async =>
      [];
}
