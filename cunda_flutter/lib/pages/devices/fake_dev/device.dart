import 'package:cunda_flutter/pages/system/logs_page.dart';
import 'package:cunda_flutter/pages/system/ota_page.dart';
import 'package:cunda_flutter/pages/system/system_status.dart';
import 'package:cunda_flutter/providers/ble/ble_providers.dart';
import 'package:cunda_flutter/utils/bluetooth_device_extension.dart';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

enum _DeviceMenuAction { logs, ota }

class FakeDevicePage extends StatelessWidget {
  final BluetoothDevice device;
  const FakeDevicePage({super.key, required this.device});

  @override
  Widget build(BuildContext context) {
    return DefaultTabController(
      length: 2,
      child: Scaffold(
        appBar: AppBar(
          title: _connectionPageTitle(),
          centerTitle: true,
          actions: [
            PopupMenuButton<_DeviceMenuAction>(
              onSelected: (action) => switch (action) {
                _DeviceMenuAction.logs => Navigator.push(
                  context,
                  MaterialPageRoute(builder: (_) => LogsPage(device: device)),
                ),
                _DeviceMenuAction.ota => Navigator.push(
                  context,
                  MaterialPageRoute(builder: (_) => OtaPage(device: device)),
                ),
              },
              itemBuilder: (BuildContext context) => const [
                PopupMenuItem(
                  value: _DeviceMenuAction.logs,
                  child: Text('Logs'),
                ),
                PopupMenuItem(
                  value: _DeviceMenuAction.ota,
                  child: Text('Firmware Update'),
                ),
              ],
            ),
          ],
          bottom: const TabBar(
            tabs: [
              Tab(text: "Application"),
              Tab(text: "System"),
            ],
          ),
        ),
        body: TabBarView(
          children: [
            Center(child: Text("Fake Application Page")),
            SystemStatus(device: device),
          ],
        ),
      ),
    );
  }

  Row _connectionPageTitle() {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(device.chosenName),
        SizedBox(width: 18),
        ConnectionStatusDot(device: device),
      ],
    );
  }
}

class ConnectionStatusDot extends ConsumerWidget {
  const ConnectionStatusDot({super.key, required this.device});

  final BluetoothDevice device;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    bool isConnected =
        ref.watch(connectionManagerProvider(device)) ==
        ConnectionTransitionState.connected;
    return Container(
      width: 10,
      height: 10,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: isConnected ? Colors.green : Colors.red,
      ),
    );
  }
}
