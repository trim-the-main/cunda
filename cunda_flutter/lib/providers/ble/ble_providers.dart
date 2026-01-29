import 'dart:async';

import 'package:cunda_flutter/constants.dart';
import 'package:cunda_flutter/services/ble/ble.dart';
import 'package:cunda_flutter/services/ble/ble_fbp_impl.dart';
import 'package:cunda_flutter/utils/bluetooth_device_extension.dart';
import 'package:cunda_flutter/utils/riverpod_utils.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
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

@riverpod
bool isThisDeviceConnected(Ref ref, BluetoothDevice device) {
  final sub = device.connectionState.listen((_) {
    ref.invalidateSelf();
  });
  ref.onDispose(sub.cancel);
  return device.isConnected;
}

@Riverpod(keepAlive: true, retry: retryOnce)
FutureOr<BluetoothDevice> connectedBluetoothDevice(
  Ref ref,
  BluetoothDevice device,
) async {
  _log.fine("Rebuilding connectedDeviceProvider");

  if (!ref.read(bluetoothAdapterOnProvider)) {
    throw NoNeedToRetry("Bluetooth adapter is off");
  }

  if (device.isDisconnected) {
    await device.connectTrackingTransitionState(timeout: connectTimeout);
  }

  final sub = device.connectionState.listen((connState) {
    if (connState == BluetoothConnectionState.disconnected) {
      _log.fine("Device disconnected");
      ref.invalidateSelf();
    }
  });
  ref.onDispose(sub.cancel);
  ref.onDispose(() {
    _log.fine("Disposing connectedDeviceProvider");
  });

  return device;
}

// Helper stream providers from riverpod. They are handy because
// riverpod helps managing the subscriptions to these easily. We
// don't need a stateful widget to keep the subscription so that we
// cancel it when we dispose the widget. ref.listen or ref.watch
// automatically handles that for us.

@riverpod
Stream<int> rssiStream(Ref ref, BluetoothDevice device) async* {
  _log.fine("RSSI stream provider restarting");
  yield* device.rssiStream(Duration(seconds: 2));
}

@riverpod
Stream<int> mtuStream(Ref ref, BluetoothDevice device) async* {
  yield* device.mtu;
}
