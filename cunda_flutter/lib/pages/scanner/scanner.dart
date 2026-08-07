import 'dart:ui';

import 'package:collection/collection.dart';
import 'package:cunda_flutter/pages/devices/demo_esp32/device.dart';
import 'package:cunda_flutter/pages/devices/fake_dev/device.dart';
import 'package:cunda_flutter/pages/devices/nokta/home.dart';
import 'package:cunda_flutter/pages/devices/tirbod/home.dart';
import 'package:cunda_flutter/providers/rpc/protocol.dart';
import 'package:cunda_flutter/utils/bluetooth_device_extension.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:logging/logging.dart';
import 'package:cunda_flutter/providers/ble/ble_providers.dart';

import 'package:cunda_flutter/constants.dart';

final _log = Logger('ScannerPage');

// This page is responsible for scanning, connecting and navigating between
// different ble devices
class ScannerPage extends ConsumerWidget {
  const ScannerPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final bluetoothAdapterOn = ref.watch(bluetoothAdapterOnProvider);

    if (bluetoothAdapterOn) {
      return Scaffold(
        appBar: AppBar(
          title: const Text('Nearby Devices'),
          centerTitle: true,
          actions: [],
        ),
        body: Center(child: DeviceListWidget()),
        floatingActionButton: ScanFloatingActionButton(),
      );
    } else {
      return BluetoothAdapterOffScreen();
    }
  }
}

class BluetoothAdapterOffScreen extends StatelessWidget {
  const BluetoothAdapterOffScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Nearby Devices'), centerTitle: true),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.bluetooth_disabled, size: 80),
            Text("Bluetooth is off", style: TextStyle(fontSize: 20)),
          ],
        ),
      ),
    );
  }
}

class DeviceListWidget extends ConsumerWidget {
  const DeviceListWidget({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return RefreshIndicator(
      child: ScrollConfiguration(
        behavior: ScrollConfiguration.of(context).copyWith(
          dragDevices: {PointerDeviceKind.touch, PointerDeviceKind.mouse},
        ),
        child: DeviceList(),
      ),
      onRefresh: () async {
        final bleScannerService = ref.read(bleScannerServiceProvider);
        if (bleScannerService.isScanningNow) {
          await bleScannerService.stopScan();
        }
        if (context.mounted) {
          await ref
              .read(bleScannerServiceProvider)
              .startScan(timeout: Constants.scanTimeout);
        }
      },
    );
  }
}

class ScanFloatingActionButton extends ConsumerWidget {
  const ScanFloatingActionButton({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final bleScanner = ref.read(bleScannerServiceProvider);
    final bool isScanningNow = ref.watch(bleScanningNowProvider);
    if (isScanningNow) {
      return FloatingActionButton.extended(
        onPressed: () {
          bleScanner.stopScan();
        },
        icon: Icon(Icons.stop_outlined),
        label: Text("Stop"),
      );
    } else {
      return FloatingActionButton.extended(
        onPressed: () {
          bleScanner.startScan(timeout: Constants.scanTimeout);
        },
        icon: Icon(Icons.refresh),
        label: Text("Scan"),
      );
    }
  }
}

class DeviceList extends ConsumerWidget {
  const DeviceList({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final bleScannedDevices = ref.watch(bleScannedDevicesProvider);
    final bleSysDevices = ref.watch(bleSysDevicesProvider);
    final isScanning = ref.watch(bleScanningNowProvider);

    return bleScannedDevices.isEmpty && bleSysDevices.isEmpty && isScanning
        ? const CircularProgressIndicator()
        : ListView(
            children:
                bleSysDevices.map((d) => ScanResultCard(d)).toList() +
                bleScannedDevices
                    .map((r) => ScanResultCard(r.device, rssi: r.rssi))
                    .sortedBy((r) => -(r.rssi ?? 0))
                    .toList(),
          );
  }
}

class ScanResultCard extends ConsumerWidget {
  final BluetoothDevice device;
  final int? rssi;
  const ScanResultCard(this.device, {super.key, this.rssi});

  void _connectCb(BuildContext context, WidgetRef ref) async {
    final connState = ref.watch(connectionManagerProvider(device));
    final connManager = ref.read(connectionManagerProvider(device).notifier);

    if (connState == ConnectionTransitionState.disconnected) {
      _log.fine("Initiating connect on ${device.chosenName}");
      try {
        await connManager.connectWithReconnect(1);
      } catch (e) {
        _log.severe("Error connecting to device: $e");
        if (context.mounted) {
          await showDialog(
            context: context,
            builder: (context) => AlertDialog(
              title: Text("Error"),
              content: Text("Failed to connect: $e"),
              actions: [
                TextButton(
                  onPressed: Navigator.of(context).pop,
                  child: const Text("OK"),
                ),
              ],
            ),
          );
        }
        return;
      }
    }

    if (context.mounted) {
      _openCb(context, ref);
    }
  }

  void _openCb(BuildContext context, WidgetRef ref) async {
    _log.fine("openning connection for cunda device");

    final deviceId = await ref.read(deviceIdProvider(device).future);
    if (!context.mounted) {
      return;
    }
    switch ((deviceId.deviceType, deviceId.protocolVersion)) {
      case ("demo_esp32", 1):
        {
          MaterialPageRoute route = MaterialPageRoute(
            builder: (context) => DemoEsp32DevicePage(device: device),
            settings: RouteSettings(name: '/connection'),
          );
          Navigator.of(context).push(route);
        }
      case ("nokta", 1):
        {
          MaterialPageRoute route = MaterialPageRoute(
            builder: (context) => NoktaDevicePage(device: device),
            settings: RouteSettings(name: '/connection'),
          );
          Navigator.of(context).push(route);
        }
      case ("tirbod", 1):
        {
          MaterialPageRoute route = MaterialPageRoute(
            builder: (context) => TirbodDevicePage(device: device),
            settings: RouteSettings(name: '/connection'),
          );
          Navigator.of(context).push(route);
        }
      case ("fake-dev", <= 2):
        {
          MaterialPageRoute route = MaterialPageRoute(
            builder: (context) => FakeDevicePage(device: device),
            settings: RouteSettings(name: '/connection'),
          );
          Navigator.of(context).push(route);
        }
      default:
        {
          throw StateError("Not a recognized device");
        }
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final connState = ref.watch(connectionManagerProvider(device));

    return Padding(
      padding: const EdgeInsets.all(4.0),
      child: Card(
        child: ListTile(
          leading: ConnectionStatusIcon(device: device),
          title: Text(device.chosenName),
          subtitle: _deviceConnectionStateLabel(connState),
          trailing: switch (connState) {
            ConnectionTransitionState.connecting ||
            ConnectionTransitionState.disconnecting =>
              CircularProgressIndicator(),
            ConnectionTransitionState.disconnected => _connectButton(
              context,
              ref,
            ),
            ConnectionTransitionState.connected => _disconnectButton(
              context,
              ref,
            ),
          },
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(10.0),
          ),
          onTap: () {
            switch (connState) {
              case ConnectionTransitionState.connected:
                _openCb(context, ref);
              case ConnectionTransitionState.disconnected:
                _connectCb(context, ref);
              case ConnectionTransitionState.connecting:
              case ConnectionTransitionState.disconnecting:
                break;
            }
          },
        ),
      ),
    );
  }

  Widget _connectButton(BuildContext context, WidgetRef ref) {
    return TextButton(
      child: Text("Connect"),
      onPressed: () => _connectCb(context, ref),
    );
  }

  Widget _disconnectButton(BuildContext context, WidgetRef ref) {
    return TextButton(
      child: Text("Disconnect"),
      onPressed: () => _disconnectCb(context, ref),
    );
  }

  Text _deviceConnectionStateLabel(ConnectionTransitionState transitionState) {
    switch (transitionState) {
      case ConnectionTransitionState.connecting:
        return Text("Connecting...");
      case ConnectionTransitionState.disconnecting:
        return Text("Disconnecting...");
      case ConnectionTransitionState.connected:
        return Text("Connected");
      case ConnectionTransitionState.disconnected:
        {
          if (rssi != null) {
            return Text(
              "Available, Signal: ${SignalStrength.fromRssiValue(rssi!).toString()}",
            );
          }
          return Text("Available, No Signal");
        }
    }
  }

  void _disconnectCb(BuildContext context, WidgetRef ref) async {
    ref.read(connectionManagerProvider(device).notifier).disconnect();
  }
}

class ConnectionStatusIcon extends StatelessWidget {
  final BluetoothDevice device;

  const ConnectionStatusIcon({super.key, required this.device});

  @override
  Widget build(BuildContext context) {
    return StreamBuilder(
      stream: device.connectionState,
      initialData: device.isConnected
          ? BluetoothConnectionState.connected
          : BluetoothConnectionState.disconnected,
      builder: (BuildContext context, AsyncSnapshot snapshot) {
        return Icon(
          snapshot.data == BluetoothConnectionState.connected
              ? Icons.bluetooth_connected
              : Icons.bluetooth,
        );
      },
    );
  }
}
