import 'dart:async';

import 'package:cunda_flutter/constants.dart';
import 'package:cunda_flutter/services/ble/ble.dart';
import 'package:cunda_flutter/services/ble/ble_fbp_impl.dart';
import 'package:cunda_flutter/utils/bluetooth_device_extension.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/misc.dart';
import 'package:logging/logging.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

part 'ble_providers.g.dart';

final _log = Logger('BleProviders');

@riverpod
BleService bleService(Ref ref) {
  final bleService = BleServiceFBP();
  bleService.setLogLevel(LogLevel.info);
  return bleService;
}

@riverpod
BleAdapter bleAdapterService(Ref ref) {
  final bleService = ref.read(bleServiceProvider);
  return bleService;
}

// TODO: Revisit if it's possible to use a streamprovider here
// the challenge was to avoid initial AsyncLoading state since we
// are indeed sure that the stream has a value ready to consume
@riverpod
class BluetoothAdapterOn extends _$BluetoothAdapterOn {
  StreamSubscription? _adapterStateSub;

  @override
  bool build() {
    final bleAdapterService = ref.read(bleAdapterServiceProvider);
    ref.onDispose(() {
      _log.fine("Disposing adapter on provider");
      _adapterStateSub?.cancel();
      _adapterStateSub = null;
    });
    _adapterStateSub = bleAdapterService.adapterState.listen((value) {
      state = value == BluetoothAdapterState.on;
    });

    return bleAdapterService.adapterStateNow;
  }
}

@riverpod
BleScanner bleScannerService(Ref ref) {
  final bleService = ref.read(bleServiceProvider);
  return bleService;
}

// TODO: The providers below could have been simple StreamProviders
// however I simply don't want them to be in AsyncLoading state even
// for one frame.
@riverpod
class BleScanningNow extends _$BleScanningNow {
  StreamSubscription? _isScanningSub;

  @override
  bool build() {
    final bleScannerService = ref.read(bleScannerServiceProvider);
    final isAdapterOn = ref.watch(bluetoothAdapterOnProvider);
    if (!isAdapterOn) {
      return false;
    }
    ref.onDispose(() {
      _isScanningSub?.cancel();
      _isScanningSub = null;
    });
    _isScanningSub = bleScannerService.isScanning.listen((value) {
      state = value;
    });
    return bleScannerService.isScanningNow;
  }
}

@riverpod
class BleSysDevices extends _$BleSysDevices {
  List<BluetoothDevice> _sysDevices = [];
  @override
  List<BluetoothDevice> build() {
    final bleScannerService = ref.read(bleScannerServiceProvider);
    final isScanning = ref.watch(bleScanningNowProvider);
    if (isScanning || _sysDevices.isEmpty) {
      bleScannerService.systemDevices([Guid("180f")]).then((value) {
        _sysDevices = value.toList();
        state = _sysDevices;
      });
    }
    return _sysDevices;
  }
}

@riverpod
class BleScannedDevices extends _$BleScannedDevices {
  StreamSubscription? _scanResultsSub;

  @override
  List<ScanResult> build() {
    final bleScannerService = ref.read(bleScannerServiceProvider);
    ref.onDispose(() {
      _scanResultsSub?.cancel();
      _scanResultsSub = null;
    });

    _scanResultsSub = bleScannerService.scanResults.listen((results) {
      state = results;
    });

    return [];
  }
}

enum ConnectionTransitionState {
  connected,
  connecting,
  disconnecting,
  disconnected,
}

ConnectionTransitionState fromConnectionState(
  BluetoothConnectionState connState,
) {
  switch (connState) {
    case BluetoothConnectionState.connected:
      return ConnectionTransitionState.connected;
    case BluetoothConnectionState.disconnected:
      return ConnectionTransitionState.disconnected;
    // ignore: deprecated_member_use
    case BluetoothConnectionState.connecting:
      return ConnectionTransitionState.connecting;
    // ignore: deprecated_member_use
    case BluetoothConnectionState.disconnecting:
      return ConnectionTransitionState.disconnecting;
  }
}

// Connection manager that keeps the provider alive if connected.
@riverpod
class ConnectionManager extends _$ConnectionManager {
  StreamSubscription? _connStateSub;
  KeepAliveLink? _keepAliveLink;
  int tryReconnect = 0;

  void updateStateAndKeepAlive({ConnectionTransitionState? newState}) {
    newState ??= device.isConnected
        ? ConnectionTransitionState.connected
        : ConnectionTransitionState.disconnected;
    switch (newState) {
      case ConnectionTransitionState.connected:
      case ConnectionTransitionState.connecting:
        _keepAliveLink ??= ref.keepAlive();
        break;
      case ConnectionTransitionState.disconnecting:
      case ConnectionTransitionState.disconnected:
        _keepAliveLink?.close();
        _keepAliveLink = null;
        break;
    }
    state = newState;
  }

  Future<void> connectWithReconnect(
    int reConnectCount, {
    Duration? timeout,
  }) async {
    tryReconnect = reConnectCount;
    while (true) {
      try {
        await connect(timeout: timeout);
        return;
      } catch (e) {
        if (reConnectCount == 0) {
          rethrow;
        } else {
          reConnectCount--;
        }
      }
    }
  }

  Future<void> connect({Duration? timeout}) async {
    timeout ??= Constants.connectTimeout;
    updateStateAndKeepAlive(newState: ConnectionTransitionState.connecting);
    try {
      _log.fine("Connecting to device ${device.chosenName}");
      await device.connect(license: License.free, timeout: timeout);
    } finally {
      _log.fine("==> connect returned ${device.chosenName}");
      updateStateAndKeepAlive();
    }
  }

  Future<void> disconnect({int? timeout}) async {
    timeout ??= Constants.disconnectTimeoutSecs;
    tryReconnect = 0;
    updateStateAndKeepAlive(newState: ConnectionTransitionState.disconnecting);
    try {
      await device.disconnect(timeout: timeout);
    } finally {
      updateStateAndKeepAlive();
    }
  }

  @override
  ConnectionTransitionState build(BluetoothDevice device) {
    ref.onDispose(() {
      _log.fine("Disposing ConnectionManager for device ${device.chosenName}");
      _connStateSub?.cancel();
      _connStateSub = null;
    });

    _connStateSub = device.connectionState.listen((_) {
      updateStateAndKeepAlive();
      if (device.isDisconnected && tryReconnect > 0) {
        _log.fine("Automatically reconnecting to device ${device.chosenName}");
        connectWithReconnect(tryReconnect);
      }
    });

    if (device.isConnected) {
      _keepAliveLink ??= ref.keepAlive();
      return ConnectionTransitionState.connected;
    } else {
      _keepAliveLink?.close();
      _keepAliveLink = null;
      return ConnectionTransitionState.disconnected;
    }
  }
}

@riverpod
Stream<int> rssiStream(Ref ref, BluetoothDevice device) async* {
  _log.fine("RSSI stream provider restarting");
  yield* device.rssiStream(Duration(seconds: 2));
}

@riverpod
Stream<int> mtuStream(Ref ref, BluetoothDevice device) async* {
  yield* device.mtu;
}
